//! The avian physics world as seen by the gun: shots and probes ignore the
//! shooter, `PortalHelperCollider`s, sensors and dynamic bodies.
use avian3d::prelude::*;
use bevy::prelude::*;

use crate::rules::{Hit, World};

/// Avian-backed [`World`]. `skip` returns true for colliders a shot passes through.
pub struct PhysicsWorld<'a> {
    /// Avian spatial queries.
    pub spatial: &'a SpatialQuery<'a, 'a>,
    /// Excludes the shooter.
    pub filter: SpatialQueryFilter,
    /// True for colliders a shot passes through.
    pub skip: &'a dyn Fn(Entity) -> bool,
    /// True for colliders (or bodies) tagged `Portalable`, or a point
    /// covered in portal gel.
    pub portalable: &'a dyn Fn(Entity, Vec3) -> bool,
}

impl World for PhysicsWorld<'_> {
    fn ray(&self, origin: Vec3, dir: Vec3, max: f32) -> Option<Hit> {
        let dir3 = Dir3::new(dir).ok()?;
        let h = self.spatial.cast_ray_predicate(origin, dir3, max, true, &self.filter, &|e| !(self.skip)(e))?;
        let point = origin + dir * h.distance;
        // Two surfaces in the same place: take the portalable one. A panel
        // swung into place can sit flush with a nodraw/clip brush, and which
        // one the ray reports first is down to rounding (sp_a2_intro).
        if !(self.portalable)(h.entity, point) {
            const TIE: f32 = 1.0 * p2_core::U;
            let back = point - dir * TIE;
            let other = self.spatial.cast_ray_predicate(back, dir3, 2.0 * TIE, true, &self.filter, &|e| e != h.entity && !(self.skip)(e));
            if let Some(o) = other {
                let at = back + dir * o.distance;
                if (self.portalable)(o.entity, at) && o.normal.dot(h.normal) > 0.99 {
                    return Some(Hit { entity: o.entity, point: at, normal: o.normal, distance: h.distance - TIE + o.distance });
                }
            }
        }
        Some(Hit { entity: h.entity, point, normal: h.normal, distance: h.distance })
    }

    fn portalable(&self, entity: Entity, point: Vec3) -> bool {
        (self.portalable)(entity, point)
    }
}
