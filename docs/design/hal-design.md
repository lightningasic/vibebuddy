# `vibe-hal` — the hardware abstraction layer

Status: draft · Date: 2026-09-09 · Owner: IClawMini

Implements the HAL row of [`../product/VBOS-blueprint.md`](../product/VBOS-blueprint.md)
and REQ-HAL-001..005 of [`../product/PRD-VBOS-001.md`](../product/PRD-VBOS-001.md). The
starting point is [`robotd-design.md`](robotd-design.md) §2.5: *"There is exactly one robot;
a second revision can be a second table."* VibeBuddy OS exists to make that second robot a
YAML file instead of a decision.

## 1. What is abstracted, and what is not

The alpha robot's hardware coupling is real and load-bearing:

- `vibe-control/src/model.rs` hard-codes **15 joints**, their **Dynamixel IDs**
  (`20–24 / 30–34 / 10–14`), the mouth index, home pose and the expected registers.
- `vibe-control/src/bus.rs` (`DynamixelIo`) hard-codes the combined `sync_read` of one IMU
  board + 15 servos, the register addresses, the conversion factors.
- `robotd/src/params.rs` hard-codes the tuning presets.

The abstraction must not bury any of that. The control loop is a real-time system whose
correctness depends on register-level facts (which byte is position, how a count converts to
radians, which device answers in a `sync_read`). A HAL that hides those facts would make the
robot *less* safe, not more portable. So the boundary is:

> **`vibe-hal` owns *what* a form is. `vibe-control` owns *how* its bus is driven.**

`vibe-hal` therefore carries:

- **Descriptors** — the static truth about a form, parsed and validated ([`descriptor`]).
- **Contracts** — the trait shapes the control stack asks for ([`bus`], [`device`]).

`vibe-hal` does **not** carry:

- drivers (rustypot, `serialport`, the SFLP decoder), or the conversion factors;
- the control loop, the policy, or the safety layer.

That division is why `vibe-hal` depends on only `serde`/`serde_yaml`/`thiserror`/`tracing`,
and why it deliberately does **not** depend on `vibe-ipc-proto` or `vibe-control`. The
caller that knows both worlds (e.g. `vibe-control` itself) is where a cross-check between
descriptor order and wire order lives.

## 2. The three layers

### 2.1 Descriptors: `hal/manifests/{form}.yaml`

A YAML file per form. It records, in the control loop's index order:

- `joints`: name, Dynamixel ID, anatomical travel `(min, max)` in radians;
- `imu`: bus + device id; `perception`: ToF / camera / mic / expression screen presence;
- `bus`: kind, control-rate, baud; `compute`: SoC, RAM, storage; `power`: battery.

The chicken manifest is the verified table (it mirrors `model.rs` exactly). The cat and doge
manifests are placeholders — structurally valid, mechanically unverified, `version: 0.1` —
and carry a comment saying so. **A manifest is not real until its version is bumped past
0.1.** `vibe-hal`'s tests enforce internal consistency (unique IDs, valid limits, IMU not
colliding with a servo), so a bad file fails CI on the way in, not on the robot.

### 2.2 Bus contract: [`bus::MotorBus`]

The runtime surface the loop needs: read one tick, write one tick, torque on/off, presence.
The alpha `RobotIo` already has exactly these verbs — `MotorBus` is its form-agnostic
shape, with runtime-sized vectors instead of `[f64; 15]`. This is the trait a simulation
transport (`RemoteIo`/MuJoCo) or a second bus implements next.

### 2.3 Device contracts: [`device`]

The four kinds REQ-HAL-001 names: `Motor`, `Imu`, `Camera`, `Tof`. These are capability
traits — what a caller may ask, not how a driver answers. The IMU's `orientation()` is a
quaternion because that is the one shape the control loop consumes; `Camera` is the narrow
contract `mediad`'s health reporting needs; `Tof` exists because the chicken carries one
and `tof` is served by its own daemon.

## 3. Migration path

Nothing that works today changes today. The alpha tables in `model.rs` stay authoritative
until three things have happened:

1. **The chicken descriptor is verified as *the* table.** A test in `vibe-control` (or
   `robotd`) asserts `model.rs` still matches `hal/manifests/chicken.yaml`. Until then the
   two can legitimately disagree and the YAML is the junior copy.
2. **A consumer exists and the loop is sized by the descriptor.** `robotd` loads the
   descriptor at boot; when present, its `bus.rate` *is* the control loop rate (the
   tuned rate is a fact about the form's silicon, so the manifest wins over the params
   default and says so in the journal when they disagree). The params file's
   `control.hz` remains the editable fallback for boards with no manifest, and the
   boot log reports the form name and joint count either way.
3. **A second form exists.** The whole point; until a cat or doge board exists, this is
   speculative generality in the shape of YAML. The procedure that turns a placeholder
   into a verified `1.0` is [`descriptor-bom-checklist.md`](descriptor-bom-checklist.md):
   what to measure on a real board, how, and what a `1.0` asserts. Current `cat.yaml` /
   `doge.yaml` are `0.1` placeholders pending that ceremony.

Each step is a small, reviewable change. The one that is *not* small — replacing the
`[f64; 15]` arrays in the loop with runtime-sized vectors — is deliberately out of scope
until step 3 forces it, because doing it before a second form exists would churn every
policy and every client for a benefit that cannot be measured yet.

## 4. Testing

- `vibe-hal` unit tests parse **every** manifest in the repo and run the invariant checks
  (the file that fails when someone adds a form with a duplicated servo ID).
- Validation rejects duplicate IDs, empty joint tables, bad limit ranges, and IMU/servo
  collisions.
- The `chicken_has_fifteen_joints_in_wire_order` test pins the index that matters most:
  the mouth at index 9, skipped by every alpha policy.
- Cross-crate order checks (`JOINT_NAMES` vs descriptor) are the caller's job, by design
  (§1 boundary) — written when step 1 lands.

## 5. Open questions

- **Actuator ranges**: the chicken manifest's limits are working values, not yet verified
  against the MJCF (the authority, which is not vendored). The safety layer today clamps to
  one actuator range; per-joint limits from the descriptor are a strictly finer protection
  (REQ-SEC-005) and a natural next change.
- **`bus.rate` as the loop clock**: `robotd` runs a literal 50 Hz. Reading it from the
  descriptor is step 2 work, and only meaningful once a form with a different rate exists.
- **Where `power` goes**: recorded today, read by nobody. The health report is the natural
  consumer once a form's battery chemistry matters.