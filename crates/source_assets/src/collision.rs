//! Brushes to convex collision hulls, split by portalability.
//!
//! A brush whose visible sides disagree about portalability (a white
//! portalable front with nodraw/black sides) is split exactly into one
//! pyramid per side, apex at the brush centroid. The pyramids of each class
//! become a compound collider, so every collider entity has a single answer
//! to "can a portal go here".

use bevy::math::{DVec3, Vec3};
use shared::{source_dir_to_world, source_to_world};

pub const CONTENTS_SOLID: u32 = 0x1;
pub const CONTENTS_WINDOW: u32 = 0x2;
pub const CONTENTS_GRATE: u32 = 0x8;
pub const CONTENTS_WATER: u32 = 0x20;
pub const CONTENTS_MOVEABLE: u32 = 0x4000;
pub const CONTENTS_PLAYERCLIP: u32 = 0x10000;
pub const CONTENTS_MONSTERCLIP: u32 = 0x20000;
pub const CONTENTS_DETAIL: u32 = 0x8000000;

/// Contents a player collides with (MASK_PLAYERSOLID minus CONTENTS_MONSTER).
pub const MASK_PLAYERSOLID: u32 =
    CONTENTS_SOLID | CONTENTS_MOVEABLE | CONTENTS_PLAYERCLIP | CONTENTS_WINDOW | CONTENTS_GRATE;
/// Contents that stop a portal shot (Portal 2 fires through grates and clips).
pub const MASK_SHOT_PORTAL: u32 = CONTENTS_SOLID | CONTENTS_MOVEABLE | CONTENTS_WINDOW;

/// One outward face of a brush.
#[derive(Clone, Debug)]
pub struct BrushSide {
    /// Source-space plane normal and distance.
    pub normal: DVec3,
    pub dist: f64,
    /// Source-space polygon (empty for bevel or degenerate sides).
    pub polygon: Vec<DVec3>,
    /// Index into the map's material table, if the side has a texinfo.
    pub material: Option<usize>,
    pub texinfo_flags: u32,
}

/// Polygon on plane `n.x = d` clipped by every other plane (keeping the back side).
pub fn side_polygon(n: DVec3, d: f64, others: &[(DVec3, f64)]) -> Vec<DVec3> {
    // Base quad: a huge square on the plane.
    let up = if n.z.abs() > 0.9 { DVec3::X } else { DVec3::Z };
    let right = up.cross(n).normalize();
    let up = n.cross(right).normalize();
    let c = n * d;
    let s = 65536.0;
    let mut poly = vec![
        c - right * s + up * s,
        c + right * s + up * s,
        c + right * s - up * s,
        c - right * s - up * s,
    ];
    for &(pn, pd) in others {
        if poly.is_empty() {
            break;
        }
        let mut out = Vec::with_capacity(poly.len() + 1);
        for i in 0..poly.len() {
            let a = poly[i];
            let b = poly[(i + 1) % poly.len()];
            let da = a.dot(pn) - pd;
            let db = b.dot(pn) - pd;
            const EPS: f64 = 0.01;
            if da <= EPS {
                out.push(a);
            }
            if (da > EPS && db < -EPS) || (da < -EPS && db > EPS) {
                let t = da / (da - db);
                out.push(a + (b - a) * t);
            }
        }
        poly = out;
    }
    // drop near-duplicate points
    let mut clean: Vec<DVec3> = Vec::with_capacity(poly.len());
    for p in poly {
        if clean.last().is_none_or(|q: &DVec3| q.distance_squared(p) > 1e-4) {
            clean.push(p);
        }
    }
    if clean.len() > 1 && clean[0].distance_squared(*clean.last().unwrap()) <= 1e-4 {
        clean.pop();
    }
    if clean.len() < 3 {
        clean.clear();
    }
    clean
}

/// Build the sides of brush `bi` with polygons.
pub fn brush_sides(bsp: &vbsp::Bsp, bi: usize, texinfo_material: &dyn Fn(i16) -> Option<usize>) -> Vec<BrushSide> {
    let brush = &bsp.brushes[bi];
    let sides = &bsp.brush_sides
        [brush.brush_side as usize..(brush.brush_side + brush.num_brush_sides) as usize];
    let planes: Vec<(DVec3, f64, bool)> = sides
        .iter()
        .map(|s| {
            let p = &bsp.planes[s.plane as usize];
            (
                DVec3::new(p.normal.x as f64, p.normal.y as f64, p.normal.z as f64),
                p.dist as f64,
                // BSP v21 dbrushside_t: `bevel` and `thin` are one byte each,
                // read together here; only the low byte marks a bevel plane.
                s.bevel & 0xff != 0,
            )
        })
        .collect();
    let bounding: Vec<(DVec3, f64)> = planes.iter().filter(|p| !p.2).map(|p| (p.0, p.1)).collect();
    sides
        .iter()
        .zip(&planes)
        .map(|(s, &(n, d, bevel))| {
            let others: Vec<(DVec3, f64)> = bounding
                .iter()
                .copied()
                .filter(|(on, od)| !(on.abs_diff_eq(n, 1e-6) && (od - d).abs() < 1e-6))
                .collect();
            let polygon = if bevel { Vec::new() } else { side_polygon(n, d, &others) };
            let texinfo_flags = bsp
                .textures_info
                .get(s.texture_info as usize)
                .filter(|_| s.texture_info >= 0)
                .map(|t| t.flags.bits())
                .unwrap_or(0);
            BrushSide {
                normal: n,
                dist: d,
                polygon,
                material: texinfo_material(s.texture_info),
                texinfo_flags,
            }
        })
        .collect()
}

/// Per-side facts the collider carries.
#[derive(Clone, Debug)]
pub struct SideInfo {
    /// Outward normal in world axes.
    pub normal: Vec3,
    /// Material name (canonical), empty if the side had no texinfo.
    pub material: String,
    pub surface_prop: Option<String>,
    pub portalable: bool,
}

/// A collider ready to spawn: convex pieces in world axes (metres), relative to the map origin.
#[derive(Clone, Debug)]
pub struct ColliderData {
    /// Convex point clouds; one piece = a plain convex hull, several = a compound.
    pub pieces: Vec<Vec<Vec3>>,
    pub portalable: bool,
    pub contents: u32,
    pub brush: u32,
    /// Brush model index: 0 = world, N = the entity with model `*N`.
    pub model: u32,
    /// The outward faces this collider covers.
    pub sides: Vec<SideInfo>,
    /// Only portal shots hit it: the portalable faces of a "never solid"
    /// func_brush (sp_a2_intro's panel GLaDOS swings into place).
    pub shot_only: bool,
}

impl ColliderData {
    /// Only a player (and NPC) clip: blocks player movement, not physics.
    pub fn player_clip_only(&self) -> bool {
        self.contents & (CONTENTS_SOLID | CONTENTS_MOVEABLE | CONTENTS_WINDOW | CONTENTS_GRATE) == 0
            && self.contents & CONTENTS_PLAYERCLIP != 0
    }

    /// Portal shots pass through (grates, player clips).
    pub fn shot_passthrough(&self) -> bool {
        contents_shot_passthrough(self.contents)
    }
}

/// Portal shots pass through these contents: grates (brushes, and models
/// compiled `$contents grate` such as handrails) and anything outside
/// MASK_SHOT_PORTAL (player clips).
pub fn contents_shot_passthrough(contents: u32) -> bool {
    contents & MASK_SHOT_PORTAL == 0 || contents & CONTENTS_GRATE != 0
}

fn to_world(p: DVec3, origin: DVec3) -> Vec3 {
    source_to_world((p - origin).as_vec3())
}

/// Turn one brush into one or two colliders.
/// `side_portalable(side, polygon_centroid)` decides each side's class.
pub fn brush_colliders(
    sides: &[BrushSide],
    contents: u32,
    brush: u32,
    model: u32,
    origin: DVec3,
    side_portalable: &dyn Fn(&BrushSide, DVec3) -> bool,
    side_info: &dyn Fn(&BrushSide) -> (String, Option<String>),
) -> Vec<ColliderData> {
    let faces: Vec<&BrushSide> = sides.iter().filter(|s| s.polygon.len() >= 3).collect();
    if faces.len() < 4 {
        return Vec::new();
    }
    let mut all: Vec<DVec3> = faces.iter().flat_map(|s| s.polygon.iter().copied()).collect();
    all.dedup_by(|a, b| a.distance_squared(*b) < 1e-4);
    let centroid = all.iter().copied().sum::<DVec3>() / all.len() as f64;
    let classes: Vec<bool> = faces
        .iter()
        .map(|s| {
            let c = s.polygon.iter().copied().sum::<DVec3>() / s.polygon.len() as f64;
            side_portalable(s, c)
        })
        .collect();
    let infos: Vec<SideInfo> = faces
        .iter()
        .zip(&classes)
        .map(|(s, &p)| {
            let (material, surface_prop) = side_info(s);
            SideInfo {
                normal: source_dir_to_world(s.normal.as_vec3()),
                material,
                surface_prop,
                portalable: p,
            }
        })
        .collect();

    let uniform = classes.iter().all(|&c| c == classes[0]);
    if uniform {
        return vec![ColliderData {
            pieces: vec![all.iter().map(|&p| to_world(p, origin)).collect()],
            portalable: classes[0],
            contents,
            brush,
            model,
            sides: infos,
            shot_only: false,
        }];
    }
    let mut out = Vec::new();
    for class in [true, false] {
        let mut pieces = Vec::new();
        let mut sides_c = Vec::new();
        for ((s, &c), info) in faces.iter().zip(&classes).zip(&infos) {
            if c != class {
                continue;
            }
            let mut pts: Vec<Vec3> = s.polygon.iter().map(|&p| to_world(p, origin)).collect();
            pts.push(to_world(centroid, origin));
            pieces.push(pts);
            sides_c.push(info.clone());
        }
        if !pieces.is_empty() {
            out.push(ColliderData {
                pieces,
                portalable: class,
                contents,
                brush,
                model,
                sides: sides_c,
            shot_only: false,
            });
        }
    }
    out
}

/// Most vertices a face of a collision hull may have. avian's contact
/// generation (parry 0.27) clips only the first four vertices of a face, so a
/// cube resting on a button's 14-sided pad found contact on a sliver at its
/// edge, sank into the rest and rocked forever (T-145).
const MAX_FACE_VERTICES: u32 = 4;
/// Most cuts per hull (a button pad takes 7); rounder hulls keep their
/// remaining big faces rather than shatter into slivers.
const MAX_CUTS: usize = 15;

/// The convex hull of `points` (world metres) as exact convex pieces whose
/// faces have at most [`MAX_FACE_VERTICES`] vertices: the biggest face is
/// cut along one of its diagonals, again and again, until none is left.
/// Hulls that already qualify come back whole.
pub fn contact_hulls(points: &[Vec3]) -> Vec<Vec<Vec3>> {
    let (mut out, mut cuts) = (Vec::new(), MAX_CUTS);
    split_hull(points.to_vec(), 0, &mut cuts, &mut out);
    out
}

fn split_hull(points: Vec<Vec3>, depth: u32, cuts: &mut usize, out: &mut Vec<Vec<Vec3>>) {
    use avian3d::parry::shape::ConvexPolyhedron;
    const MAX_DEPTH: u32 = 12;
    let Some(mut poly) = ConvexPolyhedron::from_convex_hull(&points) else {
        out.push(points);
        return;
    };
    // Vertices in the middle of a straight edge aren't corners: drop them so
    // faces list only their corners (they made quads look like pentagons).
    let kept = hull_corners(&poly);
    if kept.len() < poly.points().len() && kept.len() >= 4 {
        match ConvexPolyhedron::from_convex_hull(&kept) {
            Some(p) => poly = p,
            None => {
                out.push(points);
                return;
            }
        }
    }
    let pts = poly.points();
    let big = poly.faces().iter().filter(|f| f.num_vertices_or_edges > MAX_FACE_VERTICES).max_by_key(|f| f.num_vertices_or_edges);
    let Some(face) = big.filter(|_| *cuts > 0 && depth < MAX_DEPTH) else {
        out.push(pts.to_vec());
        return;
    };
    let first = face.first_vertex_or_edge as usize;
    let corners: Vec<Vec3> = poly.vertices_adj_to_face()[first..first + face.num_vertices_or_edges as usize].iter().map(|&i| pts[i as usize]).collect();
    let size = pts.iter().fold(Vec3::ZERO, |m, p| m.max(p.abs())).max_element().max(1e-3);
    let eps = size * 1e-5;
    // Candidate cuts through two corners of the face: perpendicular to it,
    // or through a third hull point. Each is scored by the biggest face it
    // leaves anywhere on the hull (a convex face cut in two keeps the corners
    // on each side plus two), then by how evenly it splits this face.
    let faces: Vec<Vec<Vec3>> = poly
        .faces()
        .iter()
        .map(|f| {
            let first = f.first_vertex_or_edge as usize;
            poly.vertices_adj_to_face()[first..first + f.num_vertices_or_edges as usize].iter().map(|&i| pts[i as usize]).collect()
        })
        .collect();
    let mut best: Option<((usize, usize), Vec3, Vec3)> = None;
    for (i, &a) in corners.iter().enumerate() {
        for &b in &corners[i + 1..] {
            let normals = std::iter::once(face.normal.cross(b - a)).chain(pts.iter().map(|&c| (b - a).cross(c - a)));
            for n in normals {
                let n = n.normalize_or_zero();
                if n == Vec3::ZERO {
                    continue;
                }
                let sides = |vs: &[Vec3]| (vs.iter().filter(|&&v| n.dot(v - a) > eps).count(), vs.iter().filter(|&&v| n.dot(v - a) < -eps).count());
                let (front, back) = sides(&corners);
                if front == 0 || back == 0 {
                    continue;
                }
                // The cut's own face: a corner per hull edge crossed or
                // hull point on the plane.
                let d = |p: Vec3| n.dot(p - a);
                let section = pts.iter().filter(|&&p| d(p).abs() <= eps).count()
                    + poly.edges().iter().filter(|e| { let (d0, d1) = (d(pts[e.vertices[0] as usize]), d(pts[e.vertices[1] as usize])); (d0 > eps && d1 < -eps) || (d0 < -eps && d1 > eps) }).count();
                let biggest = faces.iter().map(|vs| match sides(vs) { (f, k) if f > 0 && k > 0 => f.max(k) + 2, _ => vs.len() }).max().unwrap_or(0).max(section);
                let score = (biggest, usize::MAX - front.min(back));
                if best.is_none_or(|b| score < b.0) {
                    best = Some((score, n, a));
                }
            }
        }
    }
    let Some((_, n, a)) = best else {
        out.push(pts.to_vec());
        return;
    };
    *cuts -= 1;
    let dist = |p: Vec3| n.dot(p - a);
    let (mut front, mut back) = (Vec::new(), Vec::new());
    for &p in pts {
        let d = dist(p);
        if d >= -eps {
            front.push(p);
        }
        if d <= eps {
            back.push(p);
        }
    }
    // Where the plane crosses the hull's edges belongs to both halves.
    for e in poly.edges() {
        let (p0, p1) = (pts[e.vertices[0] as usize], pts[e.vertices[1] as usize]);
        let (d0, d1) = (dist(p0), dist(p1));
        if (d0 > eps && d1 < -eps) || (d0 < -eps && d1 > eps) {
            let x = p0 + (p1 - p0) * (d0 / (d0 - d1));
            front.push(x);
            back.push(x);
        }
    }
    split_hull(front, depth + 1, cuts, out);
    split_hull(back, depth + 1, cuts, out);
}

/// The hull's points that are corners of every face they're on (not in the
/// middle of a straight edge).
fn hull_corners(poly: &avian3d::parry::shape::ConvexPolyhedron) -> Vec<Vec3> {
    let pts = poly.points();
    let mut keep = vec![true; pts.len()];
    for f in poly.faces() {
        let first = f.first_vertex_or_edge as usize;
        let ids = &poly.vertices_adj_to_face()[first..first + f.num_vertices_or_edges as usize];
        for k in 0..ids.len() {
            let prev = pts[ids[(k + ids.len() - 1) % ids.len()] as usize];
            let cur = pts[ids[k] as usize];
            let next = pts[ids[(k + 1) % ids.len()] as usize];
            let (e1, e2) = ((cur - prev).normalize_or_zero(), (next - cur).normalize_or_zero());
            if e1.dot(e2) > 0.0 && e1.cross(e2).length() < 1e-3 {
                keep[ids[k] as usize] = false;
            }
        }
    }
    pts.iter().zip(keep).filter(|(_, k)| *k).map(|(p, _)| *p).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube_sides(half: f64) -> Vec<BrushSide> {
        let axes = [DVec3::X, -DVec3::X, DVec3::Y, -DVec3::Y, DVec3::Z, -DVec3::Z];
        let planes: Vec<(DVec3, f64)> = axes.iter().map(|&n| (n, half)).collect();
        planes
            .iter()
            .enumerate()
            .map(|(i, &(n, d))| {
                let others: Vec<_> = planes.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, p)| *p).collect();
                BrushSide {
                    normal: n,
                    dist: d,
                    polygon: side_polygon(n, d, &others),
                    material: Some(i),
                    texinfo_flags: 0,
                }
            })
            .collect()
    }

    /// A prism over a regular `n`-gon, `r` wide and `h` high (metres).
    fn prism(n: usize, r: f32, h: f32) -> Vec<Vec3> {
        (0..n)
            .flat_map(|i| {
                let t = i as f32 / n as f32 * std::f32::consts::TAU;
                [Vec3::new(r * t.cos(), 0.0, r * t.sin()), Vec3::new(r * t.cos(), -h, r * t.sin())]
            })
            .collect()
    }

    #[test]
    fn many_sided_hulls_are_cut_into_quad_faced_pieces() {
        use avian3d::parry::shape::ConvexPolyhedron;
        use avian3d::prelude::*;
        // The floor button pad: a 14-sided prism (T-145).
        let pad = prism(14, 0.82, 0.095);
        let pieces = contact_hulls(&pad);
        assert!(pieces.len() > 1);
        let mut volume: f32 = 0.0;
        for p in &pieces {
            let poly = ConvexPolyhedron::from_convex_hull(p).unwrap();
            assert!(poly.faces().iter().all(|f| f.num_vertices_or_edges <= MAX_FACE_VERTICES));
            volume += Collider::convex_hull(p.clone()).unwrap().mass(1.0);
        }
        // Exact: the pieces fill the prism.
        let whole = Collider::convex_hull(pad.clone()).unwrap().mass(1.0);
        assert!((volume - whole).abs() < whole * 1e-3, "{volume} vs {whole}");
        // A box is left alone, also with extra points on its edges.
        assert_eq!(contact_hulls(&prism(4, 0.5, 0.5)).len(), 1);
        let mut boxy = prism(4, 0.5, 0.5);
        boxy.push((boxy[0] + boxy[2]) * 0.5);
        boxy.push((boxy[1] + boxy[3]) * 0.5);
        let whole = contact_hulls(&boxy);
        assert_eq!(whole.len(), 1);
        assert_eq!(whole[0].len(), 8);
    }

    #[test]
    fn grates_and_clips_pass_portal_shots() {
        // handrail_128.mdl is compiled CONTENTS_GRATE (T-130); metal_box solid.
        assert!(contents_shot_passthrough(CONTENTS_GRATE));
        assert!(contents_shot_passthrough(CONTENTS_SOLID | CONTENTS_GRATE));
        assert!(contents_shot_passthrough(CONTENTS_PLAYERCLIP));
        assert!(!contents_shot_passthrough(CONTENTS_SOLID));
        assert!(!contents_shot_passthrough(CONTENTS_WINDOW));
    }

    #[test]
    fn cube_side_polygons_are_squares() {
        for s in cube_sides(32.0) {
            assert_eq!(s.polygon.len(), 4);
            for p in &s.polygon {
                assert!((p.abs().max_element() - 32.0).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn mixed_brush_splits_into_two_classes() {
        let sides = cube_sides(32.0);
        let cols = brush_colliders(
            &sides,
            CONTENTS_SOLID,
            0,
            0,
            DVec3::ZERO,
            &|s, _| s.normal == DVec3::X,
            &|_| (String::new(), None),
        );
        assert_eq!(cols.len(), 2);
        let port = cols.iter().find(|c| c.portalable).unwrap();
        assert_eq!(port.pieces.len(), 1);
        assert_eq!(port.pieces[0].len(), 5); // square + apex
        let non = cols.iter().find(|c| !c.portalable).unwrap();
        assert_eq!(non.pieces.len(), 5);

        let uniform = brush_colliders(&sides, CONTENTS_SOLID, 0, 0, DVec3::ZERO, &|_, _| false, &|_| (String::new(), None));
        assert_eq!(uniform.len(), 1);
        assert_eq!(uniform[0].pieces.len(), 1);
    }
}
