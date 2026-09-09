//! # vibe-hal — the VibeBuddy OS hardware abstraction layer
//!
//! The layer that lets one OS image drive every robot form. Today the control loop in
//! `vibe-control` is compiled against one fixed joint table; this crate is the answer to
//! "a second form is a new file, not a fork":
//!
//! - [`descriptor`] parses and validates the hardware descriptor files in
//!   `hal/manifests/` — the *static* truth about a form (servo IDs, limits, IMU,
//!   perception, compute).
//! - [`bus`] defines the *runtime* surface a form exposes to the controllers: the
//!   motor-bus transaction shape that `robotd` needs from the hardware.
//! - [`device`] names the four device kinds the requirements call out (REQ-HAL-001):
//!   [`Motor`](device::Motor), [`Imu`](device::Imu), [`Camera`](device::Camera) and
//!   [`Tof`](device::Tof).
//!
//! **Boundary:** this crate owns *what* a form is. *How* a particular bus is driven (the
//! Dynamixel v2 protocol, rustypot, `sync_read` layouts) belongs to `vibe-control`, which
//! owns the control loop and must not have its knowledge of the real bus weakened by an
//! abstraction that hides the register-level facts the loop depends on. `vibe-hal` is the
//! *contract*, `vibe-control` the *implementation* — the same split the repo already uses
//! between IPC vocabulary and daemons.
//!
//! See `docs/design/hal-design.md` for the design and the migration path from the
//! hard-coded alpha table.

pub mod bus;
pub mod descriptor;
pub mod device;

pub use descriptor::{Form, HardwareDescriptor};
