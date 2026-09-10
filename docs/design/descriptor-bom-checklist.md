# Descriptor BOM verification checklist

What it takes to move a form's hardware descriptor from `0.1` (placeholder) to `1.0`
(mechanically verified). The companion to [`hal-design.md`](hal-design.md): that doc says
*why* the migration path stops at a second form; this one says *how* a form earns its
`1.0`. The ceremony exists because a `1.0` descriptor is a promise the runtime builds
safety on — the chicken's table is what the ankle limits, the neck limits and the IMU
ID are *believed* from — so bumping it must be a fact-finding run against a real board,
not a hopeful edit.

**Status today:** `chicken.yaml` is `1.0` and is the reference table (alpha hardware,
cross-checked against the compiled tables in `vibe-control`). `cat.yaml` and `doge.yaml`
are `0.1` placeholders — structurally valid, mechanically unverified. This document is
the procedure that moves them.

---

## 1. What a `1.0` descriptor asserts

Every field a bump validates, and the question the verifier must be able to answer
*from the board*:

| Field | Assertion | How verified |
|---|---|---|
| `name` | The form's identity | Human: matches the product's name |
| `version` | `1.0`, after everything below | This checklist |
| `bus.kind` | The transport (`dynamixel` today) | Board: wiring / datasheet |
| `bus.rate` | Control-loop Hz the form is tuned for | Derived: see §3.3 |
| `bus.baud` | Wire speed the servos actually run at | Tool: `scan_baud` sweep, or the servo's stored value read back |
| `joints[].id` | Every ID on the bus, **in wire order** | Tool: bus scan (§3.2) |
| `joints[].name` | Canonical wire-order names | Human: matches `vibe-ipc-proto` `JOINT_NAMES` conventions (left leg → neck/head → right leg for chickens; whatever the form's kinematics doc dictates) |
| `joints[].limit` | Anatomical travel in radians, `(min, max)`, **not servo register travel** | Human: mechanical measurement (§3.4) |
| `imu.bus` / `imu.id` | Where the IMU board hangs | Tool: presence read, like the servos |
| `perception.*` | Presence of ToF / camera / mic / expression screen | Board: is it wired? is it enumerated by its driver? |
| `compute` | SoC, RAM, storage | Board: `cat /proc/cpuinfo`, `free`, `lsblk` / datasheet |
| `power` | Battery chemistry, capacity, runtime | Board: pack label + measured runtime |

`validate()` in `vibe-hal` enforces the *invariants* (unique IDs, sane limits, IMU not
colliding with a servo, non-empty table) — but it cannot tell a true table from a
plausible one. That is exactly the gap this checklist fills.

---

## 2. When the bump is allowed

All four gates must pass. The first three are mechanical; the last is a code review.

1. **A real board exists.** A `1.0` is written against hardware that was powered,
   on the bench or in the chassis, by whoever is signing the manifest.
2. **Every field is measured, not copied.** No field may be taken from the chicken
   table, from PRD requirements ("13+ DOF"), or from a previous hope. A value no one
   measured is a value that stays `0.1`.
3. **The wire order is confirmed against the IPC layer.** The manifest's joint order
   is the order `robotd` indexes and the order `vibe-ipc-proto` serializes. A mismatch
   swaps limbs. This is a cross-check against `JOINT_NAMES`, not a wish.
4. **CI is green with the new table.** `cargo test -p vibe-hal -p vibe-control` passes
   — the new manifest parses, validates, and (where a cross-check test exists) agrees
   with the compiled tables.

---

## 3. The ceremony

Run in order. Stop at the first failure; a `0.1` that fails stays `0.1`.

### 3.1 Inventory the board

- Physical chassis: count servos, note their model numbers, note which limb/segment
  each one sits in.
- Note the IMU board, the compute board, the battery pack label, and every
  perception device actually wired (camera / ToF / mic / expression screen).
- Photograph the wiring if anything is ambiguous — the next verifier will thank you.

### 3.2 Enumerate the bus

The IDs a `1.0` manifest lists must be *found*, not assumed. With the board powered
and the Dynamixel bus connected:

- Do a bus scan that walks every possible Dynamixel ID (`PING`, conventionally
  0–252) and reports which ones answer. The upstream SDK utilities and the
  `vibe-control` bus layer both speak Protocol 2; a one-off scan binary under
  `duck-bench` is the expected home if one does not exist yet.
- Record ID → servo model per answer.
- The **wire order** is the order the servos physically appear on the bus, which the
  control loop indexes positionally. For a chicken it is left leg → neck/head/mouth →
  right leg; the form's kinematics doc says what it is for the cat and the doge.
- The IMU answers on its bus and ID (chicken: Dynamixel ID 200). Record it.

Cross-check against step 1: every physical servo accounted for, no phantom answers,
no missing IDs.

### 3.3 Bus rate / baud

- `baud`: sweep the common rates (1 Mbps, 57600, 115200…) against the real servos, or
  read the baud back from the servo's stored configuration. The verified value is the
  one the board actually runs on.
- `rate`: the control-loop frequency this form is **tuned for** — the rate the policy
  and the safety layer were exercised at. It is a *derived* value: pick the rate the
  board held stably under a full-body motion test, not the chicken's 50 Hz out of
  habit. See `robotd-design.md` for what the loop does per tick.

### 3.4 Joint limits

The hardest field, and the one most worth measuring. `limit` is **anatomical travel in
radians**, not the servo's register range ([0, 4095] on a 12-bit Dynamixel):

- Measure the physical travel of each joint — from mechanical stop to mechanical stop,
  in radians, with the limb driven by hand and a protractor or an angle gauge. Record
  `min` and `max` relative to the home pose the form defines.
- The safety layer clamps targets to this range; **an optimistic limit is a real risk**
  (a limb driven into its stop, or a gait that assumes range the mechanism doesn't
  have). If a value is uncertain, record the *conservative* number and flag it in the
  PR.
- Where the form has an MJCF / MuJoCo model, the model's joint ranges are the second
  source — but the physical measurement wins when they disagree.

### 3.5 Compute, power, perception presence

- `compute`: read the board (`soc` from `/proc/cpuinfo` or datasheet; `ram_gb`/`storage_gb`
  from `free`/`lsblk`). These must match what a shipping unit ships with.
- `power`: battery label (chemistry + capacity), then a measured runtime at a known load
  for `runtime_min`. Informational today — nothing drives code yet — but a `1.0` records
  the truth anyway.
- `perception`: list what is actually wired and enumerated: camera (resolution from its
  driver), ToF (bus + I²C address), mic, expression screen (resolution).

### 3.6 Rewrite the manifest

- Fill every field from the measurements above. Keep the block comment that names the
  verification source and date — future readers should be able to tell *which* board
  and *when*.
- Bump `version` to `1.0` **only** after everything else is green.
- If the joint table differs from the placeholder in structure (count, IDs, names,
  order), the cross-check test in `vibe-control` and the `vibe-ipc-proto` tables may
  need updating first — that is a full code change, not a manifest edit, and it goes
  through the normal review.

### 3.7 Prove it

- `cargo test -p vibe-hal` — parses, validates, form title matches.
- `cargo test -p vibe-control` — the cross-check tests against the compiled tables.
- Run `robotd --fake --manifest hal/manifests/{form}.yaml` and confirm the boot log
  reports the form name and the measured joint count and rate.
- If there is a code change (ID table, `JOINT_NAMES`, kinematics), the full workspace
  check and any hardware smoke test the form has.

---

## 4. What does NOT get you a `1.0`

- A Plausible Table: IDs that "look right", limits "borrowed" from the chicken, a rate
  that "should work". A `1.0` that nobody measured is a lie with a version number.
- PRD compliance: "13+ DOF" describes intent, not the board. The manifest records the
  board.
- CI green on unchanged placeholders: validation passing only proves the file is
  well-formed YAML. The checklist is the part CI cannot see.

---

## 5. Signing

The manifest `1.0` is a claim signed by the person who ran this ceremony. The block
comment in the YAML should say:

```yaml
# verified 2026-MM-DD by <who>: <board serial / chassis id>, <whatever failed or
# was measured conservatively>, hardware on the bench at <location>.
```

If the ceremony was run by an agent rather than a human, the agent records the raw
measurements (scan output, angle readings) in the PR body or an attached note, and a
human still signs the `1.0` — the agent measures, the human attests.