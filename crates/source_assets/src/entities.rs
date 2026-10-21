//! The BSP entity lump as typed-enough records.

use bevy::math::{EulerRot, Quat, Vec3};
use shared::{source_to_world, SOURCE_UNIT};

/// One entity from the entity lump. Keys are lowercased; duplicate keys
/// (outputs such as `OnPressed`) are kept in order.
#[derive(Clone, Debug, Default)]
pub struct SourceEntity {
    pub classname: String,
    pub targetname: Option<String>,
    /// Source units, Z-up.
    pub origin: Vec3,
    /// Pitch, yaw, roll in degrees.
    pub angles: Vec3,
    pub keyvalues: Vec<(String, String)>,
}

impl SourceEntity {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.keyvalues
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn get_f32(&self, key: &str) -> Option<f32> {
        self.get(key).and_then(crate::kv::parse_f32)
    }

    pub fn get_vec3(&self, key: &str) -> Option<Vec3> {
        let v: Vec<f32> = self
            .get(key)?
            .split_whitespace()
            .filter_map(crate::kv::parse_f32)
            .collect();
        (v.len() >= 3).then(|| Vec3::new(v[0], v[1], v[2]))
    }

    /// Brush model index for `model "*N"`.
    pub fn brush_model(&self) -> Option<usize> {
        self.get("model")?.strip_prefix('*')?.parse().ok()
    }

    /// `.mdl` model path, normalised.
    pub fn studio_model(&self) -> Option<String> {
        let m = self.get("model")?;
        m.to_ascii_lowercase()
            .ends_with(".mdl")
            .then(|| crate::fs::normalize_path(m))
    }

    /// World-space position (metres, Y-up), relative to the map origin.
    pub fn world_position(&self) -> Vec3 {
        source_to_world(self.origin)
    }

    /// World-space rotation.
    pub fn world_rotation(&self) -> Quat {
        source_angles_to_world(self.angles)
    }
}

/// Source QAngle (pitch, yaw, roll degrees) to a world-space rotation.
///
/// Source composes `Rz(yaw) * Ry(pitch) * Rx(roll)` in Z-up space; the world
/// rotation is that conjugated by the basis change of [`source_to_world`].
pub fn source_angles_to_world(angles: Vec3) -> Quat {
    let (p, y, r) = (
        angles.x.to_radians(),
        angles.y.to_radians(),
        angles.z.to_radians(),
    );
    // Source-space rotation.
    let q_src = Quat::from_rotation_z(y) * Quat::from_rotation_y(p) * Quat::from_rotation_x(r);
    // Basis change: world = B * source, B maps x->x, y->-z, z->y.
    // B is a rotation of -90 degrees about X.
    let b = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    b * q_src * b.inverse()
}

/// Yaw-only facing for a player start: the world rotation whose -Z (Bevy
/// forward) points along the Source view direction.
pub fn source_view_to_world(angles: Vec3) -> Quat {
    let fwd = source_angles_to_world(angles) * Vec3::X; // Source forward is +X
    let yaw = (-fwd.x).atan2(-fwd.z);
    let pitch = fwd.y.clamp(-1.0, 1.0).asin();
    Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0)
}

/// Keys holding an output connection (`OnTrigger`, `OutValue`, `OnPressed`...).
pub fn is_output_key(k: &str) -> bool {
    k.starts_with("on") || k.starts_with("out") || k.starts_with("pressed") || k.starts_with("unpressed")
}

/// The entity lump as written in the map. `vbsp` lowercases all of it,
/// which breaks script code in outputs; read the raw lump when we can.
pub fn entity_lump(data: &[u8], parsed: &vbsp::Entities) -> vbsp::Entities {
    crate::bsp_raw::RawBsp::new(data)
        .ok()
        .and_then(|raw| raw.lump(0).map(|b| String::from_utf8_lossy(b).trim_end_matches('\0').to_string()))
        .map(|entities| vbsp::Entities { entities })
        .unwrap_or_else(|| vbsp::Entities { entities: parsed.entities.clone() })
}

pub fn parse_entities(lump: &vbsp::Entities) -> Vec<SourceEntity> {
    lump.iter()
        .map(|raw| {
            let mut e = SourceEntity::default();
            for (k, v) in raw.properties() {
                let k = k.to_ascii_lowercase();
                match k.as_str() {
                    "classname" => e.classname = v.to_ascii_lowercase(),
                    "targetname" => e.targetname = Some(v.to_string()),
                    _ => {}
                }
                // Everything is matched lowercase, except script code and
                // function names (Squirrel is case-sensitive): output
                // parameters (RunScriptCode) and think functions.
                let keep = is_output_key(&k) || k == "thinkfunction";
                e.keyvalues.push((k, if keep { v.to_string() } else { v.to_ascii_lowercase() }));
            }
            e.origin = e.get_vec3("origin").unwrap_or(Vec3::ZERO);
            e.angles = e.get_vec3("angles").unwrap_or(Vec3::ZERO);
            // Some entities store pitch separately (lights, player starts).
            if let Some(p) = e.get_f32("pitch") {
                if e.classname.starts_with("light") {
                    e.angles.x = p;
                }
            }
            e
        })
        .collect()
}

/// Metres per Source unit, re-exported for callers scaling models.
pub const UNIT: f32 = SOURCE_UNIT;

/// The `env_fog_controller` a map starts with: the enabled one marked
/// Master (spawnflags 1), else the first enabled one. Source hands players
/// the master controller; sp_a1_intro1 lists the dense vault fog
/// (`fog_destroyed_chambers`) before its master ride fog (`fog_intro_ride`).
pub fn map_fog_controller(entities: &[SourceEntity]) -> Option<&SourceEntity> {
    let enabled = || entities.iter().filter(|e| e.classname == "env_fog_controller" && e.get("fogenable") == Some("1"));
    let master = |e: &&SourceEntity| e.get("spawnflags").and_then(|v| v.trim().parse::<u32>().ok()).unwrap_or(0) & 1 != 0;
    enabled().find(master).or_else(|| enabled().next())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn master_fog_controller_wins() {
        let fog = |name: &str, flags: &str, enable: &str| SourceEntity {
            classname: "env_fog_controller".into(),
            targetname: Some(name.into()),
            keyvalues: vec![("spawnflags".into(), flags.into()), ("fogenable".into(), enable.into())],
            ..Default::default()
        };
        let ents = vec![fog("off_master", "1", "0"), fog("vault", "0", "1"), fog("ride", "1", "1")];
        assert_eq!(map_fog_controller(&ents).and_then(|e| e.targetname.as_deref()), Some("ride"));
        assert_eq!(map_fog_controller(&ents[..2]).and_then(|e| e.targetname.as_deref()), Some("vault"));
        assert!(map_fog_controller(&ents[..1]).is_none());
    }

    #[test]
    fn angles_map_forward_correctly() {
        // yaw 0: Source +X -> world +X
        assert!(close(source_angles_to_world(Vec3::ZERO) * Vec3::X, Vec3::X));
        // yaw 90: Source +Y -> world -Z
        let q = source_angles_to_world(Vec3::new(0.0, 90.0, 0.0));
        assert!(close(q * Vec3::X, Vec3::NEG_Z));
        // pitch 90 looks down: Source -Z -> world -Y
        let q = source_angles_to_world(Vec3::new(90.0, 0.0, 0.0));
        assert!(close(q * Vec3::X, Vec3::NEG_Y));
        // consistency with source_to_world for arbitrary angles
        let a = Vec3::new(20.0, 135.0, 10.0);
        let q_src = Quat::from_rotation_z(a.y.to_radians())
            * Quat::from_rotation_y(a.x.to_radians())
            * Quat::from_rotation_x(a.z.to_radians());
        let v = Vec3::new(1.0, 2.0, 3.0);
        assert!(close(
            source_to_world(q_src * v) / SOURCE_UNIT,
            source_angles_to_world(a) * (source_to_world(v) / SOURCE_UNIT)
        ));
    }

    #[test]
    fn view_rotation_faces_along_yaw() {
        let q = source_view_to_world(Vec3::new(0.0, 90.0, 0.0));
        assert!(close(q * Vec3::NEG_Z, Vec3::NEG_Z));
        let q = source_view_to_world(Vec3::new(0.0, 0.0, 0.0));
        assert!(close(q * Vec3::NEG_Z, Vec3::X));
    }
}
