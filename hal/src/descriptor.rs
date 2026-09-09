//! Hardware descriptors — the YAML files in `hal/manifests/`.
//!
//! A descriptor is the *static* truth about one robot form: which servos exist, what their
//! Dynamixel IDs and limits are, where the IMU sits, and what extra perception the form
//! carries (ToF, camera, mic, expression screen). The control loop in `vibe-control` is
//! written against a fixed joint table today; these types are the layer that lets that
//! table be *derived* from a file instead of compiled in, so a second form is a new file
//! rather than a fork.
//!
//! The joint *order* in a manifest is the order `robotd` indexes and the order the IPC
//! layer serializes (`JOINT_NAMES` in `vibe-ipc-proto`) — keep all three in step or the
//! robot will swap limbs. `vibe-hal` deliberately does not depend on `vibe-ipc-proto`;
//! the caller that knows both (e.g. `vibe-control`) is where the cross-check lives.

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// The bus a device hangs off. One value today; a future form with a CSI camera or a
/// second servo bus needs the enum before it needs the code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BusKind {
    Dynamixel,
    #[serde(rename = "i2c")]
    I2c,
}

/// The motor bus, as the descriptor sizes it. `rate` is the control-loop frequency the
/// form is tuned for; `baud` is the wire speed the drivers open the port with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bus {
    /// The underlying transport; the driver chosen from this.
    #[serde(rename = "type")]
    pub kind: BusKind,
    /// Control-loop frequency, Hz.
    pub rate: u8,
    /// Wire baud if the bus is serial; meaningless for I²C but harmless.
    pub baud: u32,
}

/// One joint. `limit` is the actuator travel in radians; the controller's safety layer
/// clamps targets to this range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Joint {
    /// Dynamixel ID on the bus. Must be unique across the form.
    pub id: u8,
    /// Canonical name, as the wire order spells it (e.g. `left_hip_pitch`).
    pub name: String,
    /// Actuator travel in radians, `(min, max)`.
    pub limit: (f64, f64),
}

/// The IMU. On the classic form it is a device on the same Dynamixel bus (a board at
/// ID 200 serving SFLP orientation) — the form that made the runtime's combined
/// `sync_read` possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Imu {
    pub bus: BusKind,
    /// Device address on that bus.
    pub id: u8,
}

/// Extra perception. Forms differ here more than in the joint table — a doge has an
/// expression screen, a chicken a ToF, a cat a microphone. Presence is the cardinal
/// fact; the register details are a driver concern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Perception {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tof: Option<Tof>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<Camera>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mic: Option<bool>,
    /// An LED expression screen (the doge's face). `resolution` is `(width, height)`
    /// in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<Expression>,
}

/// An LED expression screen, e.g. the doge's 64×64 face.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Expression {
    /// Interface kind, e.g. `spi`.
    #[serde(rename = "type")]
    pub kind: String,
    pub resolution: (u16, u16),
}

/// A time-of-flight array, e.g. an 8×8 VL53L1 at I²C 0x29.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tof {
    pub bus: BusKind,
    pub id: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Camera {
    /// Interface kind, e.g. `mipi`; the driver is chosen from this plus the SoC.
    #[serde(rename = "type")]
    pub kind: String,
    pub resolution: (u16, u16),
}

/// The compute board. One board today (RK3566); the split exists so a form can be
/// described before its board exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Compute {
    pub soc: String,
    pub ram_gb: u8,
    pub storage_gb: u8,
}

/// Power source. Informational today — the alpha battery is a 2S pack whose voltage
/// `robotd` reads over the motor bus, so nothing here drives code yet — but a form's
/// chemistry *should* be recorded somewhere a health report can ask.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Power {
    pub battery: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity_mah: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_min: Option<u32>,
}

/// The full, parsed hardware descriptor — what `hal/manifests/{form}.yaml` describes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardwareDescriptor {
    /// Form name, e.g. "Vibe Chicken".
    pub name: String,
    /// Semantic version of the descriptor, not of the OS.
    pub version: String,
    /// The motor bus, sized and tuned as the form needs.
    pub bus: Bus,
    /// Joints, in the control loop's index order.
    pub joints: Vec<Joint>,
    pub imu: Imu,
    #[serde(default)]
    pub perception: Perception,
    pub compute: Compute,
    #[serde(default)]
    pub power: Option<Power>,
}

impl HardwareDescriptor {
    /// Parse a manifest from its file. Errors name the path.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, DescriptorError> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path).map_err(DescriptorError::Io)?;
        let this: Self = serde_yaml::from_str(&raw).map_err(|e| DescriptorError::Parse {
            path: path.display().to_string(),
            source: e,
        })?;
        this.validate()?;
        Ok(this)
    }

    /// Invariant checks that hold for *every* form:
    /// - servo IDs unique (the bus can't tell two same-ID servos apart)
    /// - every limit is a valid `(min, max)` pair
    /// - the IMU id is not a servo id
    /// - the form is non-empty: a robot with zero joints commands nothing
    pub fn validate(&self) -> Result<(), DescriptorError> {
        if self.joints.is_empty() {
            return Err(DescriptorError::Empty("joints"));
        }
        let mut seen = std::collections::HashSet::new();
        for joint in &self.joints {
            if !seen.insert(joint.id) {
                return Err(DescriptorError::DuplicateId(joint.id));
            }
            if joint.limit.0 >= joint.limit.1 {
                return Err(DescriptorError::BadLimit {
                    name: joint.name.clone(),
                    min: joint.limit.0,
                    max: joint.limit.1,
                });
            }
        }
        if self.joints.iter().any(|j| j.id == self.imu.id) {
            return Err(DescriptorError::ImuCollidesWithServo(self.imu.id));
        }
        Ok(())
    }
}

/// The form identifier — the value `robotd` reports as the robot model on health.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    Chicken,
    Cat,
    Doge,
}

impl Form {
    /// The canonical slug; also the file base name in `hal/manifests/`.
    pub fn slug(self) -> &'static str {
        match self {
            Form::Chicken => "chicken",
            Form::Cat => "cat",
            Form::Doge => "doge",
        }
    }

    /// Human title, as the site spells it.
    pub fn title(self) -> &'static str {
        match self {
            Form::Chicken => "Vibe Chicken",
            Form::Cat => "Vibe Cat",
            Form::Doge => "Vibe Doge",
        }
    }
}

impl fmt::Display for Form {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.slug())
    }
}

/// Why a descriptor could not be used.
#[derive(Debug, thiserror::Error)]
pub enum DescriptorError {
    #[error("cannot read descriptor: {0}")]
    Io(#[from] std::io::Error),
    #[error("descriptor {path} is not valid YAML: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("descriptor has no {0}; a robot with nothing to command is a brick")]
    Empty(&'static str),
    #[error("servo id {0} appears more than once on the bus")]
    DuplicateId(u8),
    #[error("joint {name} has an invalid limit range ({min} >= {max})")]
    BadLimit { name: String, min: f64, max: f64 },
    #[error("imu id {0} collides with a servo id on the same bus")]
    ImuCollidesWithServo(u8),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_path(slug: &str) -> String {
        // Tests run with the crate dir as CWD (`cargo test -p vibe-hal`), so `manifests/`
        // is one level up from here.
        format!("manifests/{slug}.yaml")
    }

    /// Every manifest in the repo parses AND passes the invariant checks. This is the
    /// test that fails when someone adds a form with a duplicated servo ID or a bad
    /// limit; it is why adding a form is "write a YAML, run tests" rather than a code
    /// review.
    #[test]
    fn every_manifest_parses_and_validates() {
        for form in [Form::Chicken, Form::Cat, Form::Doge] {
            let desc = HardwareDescriptor::load(manifest_path(form.slug()))
                .unwrap_or_else(|e| panic!("{} manifest: {e}", form.slug()));
            assert_eq!(desc.name, form.title());
        }
    }

    #[test]
    fn chicken_has_fifteen_joints_in_wire_order() {
        let desc = HardwareDescriptor::load(manifest_path("chicken")).unwrap();
        let names: Vec<_> = desc.joints.iter().map(|j| j.name.as_str()).collect();
        assert_eq!(names.len(), 15);
        assert_eq!(names[9], "mouth", "index 9 is the mouth — policies skip it");
        assert_eq!(
            names[0], "left_hip_yaw",
            "left leg leads, as the wire order does"
        );
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let mut desc = HardwareDescriptor::load(manifest_path("chicken")).unwrap();
        desc.joints[0].id = desc.joints[1].id;
        assert!(matches!(
            desc.validate(),
            Err(DescriptorError::DuplicateId(_))
        ));
    }

    #[test]
    fn imu_id_colliding_with_a_servo_is_rejected() {
        let mut desc = HardwareDescriptor::load(manifest_path("chicken")).unwrap();
        desc.imu.id = desc.joints[0].id;
        assert!(matches!(
            desc.validate(),
            Err(DescriptorError::ImuCollidesWithServo(_))
        ));
    }

    #[test]
    fn empty_joint_table_is_rejected() {
        let mut desc = HardwareDescriptor::load(manifest_path("chicken")).unwrap();
        desc.joints.clear();
        assert!(matches!(desc.validate(), Err(DescriptorError::Empty(_))));
    }

    #[test]
    fn bad_limit_range_is_rejected() {
        let mut desc = HardwareDescriptor::load(manifest_path("chicken")).unwrap();
        desc.joints[0].limit = (desc.joints[0].limit.1, desc.joints[0].limit.0);
        assert!(matches!(
            desc.validate(),
            Err(DescriptorError::BadLimit { .. })
        ));
    }
}
