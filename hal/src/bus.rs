//! The runtime bus contract a form must satisfy.
//!
//! This is the shape `robotd`'s control loop needs, and the shape any alternative
//! transport (the MuJoCo simulation, a remote board, a physical bus) must provide.
//! It deliberately mirrors `vibe-control`'s `RobotIo` — same read/write/set_torque
//! verbs, same arrays — so that implementing both for one hardware does not mean
//! writing the physics twice. The difference is that `RobotIo` answers with
//! `[f64; NUM_JOINTS]` sized to the alpha robot's 15; here the size comes from the
//! descriptor, which is the point: a 13-DOF cat gets a 13-wide vector.

/// One control tick of motor-bus state.
///
/// `NUM_JOINTS` is fixed at compile time in `vibe-control` today. This structure is the
/// runtime-length equivalent: the descriptor says how wide, the bus fills it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MotorSample {
    /// Present positions, radians, indexed as the descriptor's joint order.
    pub positions: Vec<f64>,
    /// Present velocities, rad/s.
    pub velocities: Vec<f64>,
    /// Present currents, milliamps (magnitude; direction is not reported by the
    /// Dynamixel present-current word).
    pub currents_ma: Vec<f64>,
}

/// The command a controller writes on one tick.
#[derive(Debug, Clone, PartialEq)]
pub struct MotorTargets {
    /// Goal positions, radians, one per joint in descriptor order.
    pub positions: Vec<f64>,
}

/// A motor bus. One instance, opened once, shared by whoever owns the loop.
///
/// The verbs are deliberately the *minimum* the control loop needs. Anything fancier
/// (register audits, per-servo gain, reboots) is a property of a concrete driver, not of
/// the contract — the alpha runtime's `DynamixelIo` has exactly such inherent methods
/// today and they stay there.
#[allow(clippy::len_without_is_empty)] // a zero-joint bus is a broken descriptor, never an empty collection
pub trait MotorBus {
    /// How many joints this bus presents. Must equal the descriptor's `joints.len()`.
    ///
    /// A zero-joint bus is a broken descriptor, never a legitimate empty collection, so
    /// there is deliberately no `is_empty`: nothing should ever be checking for one.
    fn len(&self) -> usize;

    /// Read one tick: all present state, in one transaction where the hardware allows.
    fn read(&mut self) -> Result<MotorSample, BusError>;

    /// Write one tick of goal positions.
    fn write(&mut self, targets: &MotorTargets) -> Result<(), BusError>;

    /// Torque on/off across all joints. `false` must always succeed as much as
    /// possible — a robot powering down must not be left half-locked because one servo
    /// failed to ACK (see the alpha runtime's comment on exactly this).
    fn set_torque(&mut self, on: bool) -> Result<(), BusError>;

    /// True when the bus is present and answering. A daemon that is `active` while this
    /// is false has not failed — it is *waiting for hardware* (the boot-net design).
    fn is_present(&self) -> bool {
        true
    }
}

/// Why a bus transaction failed. Structured just enough for the loop to decide: a read
/// failure is retried on the next tick; a write failure is the first thing the safety
/// layer reports.
#[derive(Debug, thiserror::Error)]
pub enum BusError {
    #[error("bus is not present")]
    NotPresent,
    #[error("bus transaction failed: {0}")]
    Transport(String),
    #[error("short read: expected {expected}, got {got}")]
    ShortRead { expected: usize, got: usize },
}
