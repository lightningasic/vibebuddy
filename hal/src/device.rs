//! The four device kinds a form can carry (REQ-HAL-001): motor, IMU, camera, ToF.
//!
//! These are *capability* traits, not driver traits: they say what the control stack may
//! ask of a device, not how the device answers. The driver for a concrete camera or ToF
//! lives next to the hardware that needs it (`vibe-control` for the IMU on the motor
//! bus, `mediad` for the camera pipeline, `tof` for depth). A trait here means a new
//! form does not change the *callers* — it changes which driver is constructed from the
//! descriptor.

/// A single motor. The control loop does not talk to one motor, it talks to the bus
/// ([`crate::bus::MotorBus`]); this trait exists for the cases that are genuinely
/// per-joint: homing, register audits, replacing a servo. A bus implementation may use
/// it internally or not at all.
pub trait Motor {
    /// The joint this motor drives, by descriptor name.
    fn joint_name(&self) -> &str;

    /// Read one present position, radians.
    fn present_position(&self) -> f64;

    /// Command one goal position, radians.
    fn set_goal(&mut self, radians: f64);

    /// True when torque is enabled.
    fn torque_enabled(&self) -> bool;
}

/// The IMU. On the alpha form it is a device on the motor bus (a board answering an
/// SFLP orientation block inside the combined `sync_read`) — which is why `vibe-control`
/// owns its decoder and this trait stays a contract rather than an implementation.
pub trait Imu {
    /// Orientation as a right-handed quaternion `(x, y, z, w)` — the one shape the
    /// control loop consumes, so every driver converts at its own edge.
    fn orientation(&self) -> [f64; 4];

    /// True once the sensor reports converged orientation — the alpha runtime waits for
    /// SFLP to be *ready* before it lets the policy observe anything.
    fn ready(&self) -> bool;
}

/// A camera. The alpha camera is a MIPI sensor driven by `mediad`'s GStreamer pipeline
/// (hardware H.264 through the RKMPP encoder); this trait is the narrow contract the
/// rest of the stack needs: does a frame exist, and what does it look like.
pub trait Camera {
    /// Resolution the sensor is currently running at, `(width, height)`.
    fn resolution(&self) -> (u16, u16);

    /// True when the latest frame was decoded successfully. False while the pipeline is
    /// starting, or after a stall — `mediad`'s health report keys off this.
    fn frame_ready(&self) -> bool;
}

/// A time-of-flight array (e.g. the 8×8 VL53L1 on the chicken's front).
pub trait Tof {
    /// The most recent depth patch, row-major, 8×8. A cell of `None` means "no
    /// measurement" (out of range, or the sensor is settling).
    fn depth(&self) -> [[Option<f32>; 8]; 8];

    /// True when the last read was fresh — the alpha precedent of a stale detector
    /// (repeated identical blocks = frozen sensor) applies here too.
    fn fresh(&self) -> bool;
}
