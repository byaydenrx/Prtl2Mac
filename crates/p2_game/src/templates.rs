//! Entity templates: `point_template` and `env_entity_maker`.
//!
//! A template's entities (`Template01`..`Template16`) are taken out of the
//! level when it loads and kept as prototypes. `ForceSpawn` makes a fresh copy
//! of each one: on the template it reappears where it was placed; on an
//! entity maker it appears at the maker, placed relative to it the way it
//! was placed relative to the template. Cube droppers run on this: each new
//! cube is a copy, and the old one fizzles when the next drops.
use avian3d::prelude::*;
use bevy::prelude::*;

use crate::io::{EntInput, Ents, FireOutput};
use crate::level::LevelLoaded;
use crate::props::Prop;

pub struct TemplatesPlugin;

impl Plugin for TemplatesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Templates>().add_systems(
            Update,
            (take_prototypes.after(crate::props::make_props_dynamic), spawn_inputs)
                .chain()
                .after(crate::io::run_io)
                .in_set(crate::level::GameSet::Logic),
        );
    }
}

/// Template entity index -> the prototype entity indices it spawns.
#[derive(Resource, Default)]
struct Templates(std::collections::HashMap<usize, Vec<usize>>);

/// The `TemplateNN` entities of a `point_template`.
fn members(ents: &Ents, t: usize) -> Vec<usize> {
    (1..=16)
        .filter_map(|n| ents.list[t].get(&format!("template{n:02}")))
        .flat_map(|name| ents.find(name, t, None))
        .filter(|&i| i != t)
        .collect()
}

/// On load: every templated entity leaves the level (hidden, no physics,
/// gets no inputs) unless the template keeps it (spawnflag 1).
fn take_prototypes(
    mut commands: Commands,
    mut loaded: MessageReader<LevelLoaded>,
    ents: Option<ResMut<Ents>>,
    mut templates: ResMut<Templates>,
    children: Query<&Children>,
    colliders: Query<(), With<Collider>>,
) {
    if loaded.read().count() == 0 {
        return;
    }
    templates.0.clear();
    let Some(mut ents) = ents else { return };
    for t in 0..ents.list.len() {
        if ents.list[t].class != "point_template" {
            continue;
        }
        let m = members(&ents, t);
        let keep = ents.list[t].spawnflags & 1 != 0;
        if !keep {
            for &i in &m {
                ents.list[i].killed = true;
                if let Some(e) = ents.list[i].entity {
                    commands.entity(e).insert((Visibility::Hidden, RigidBodyDisabled));
                    for c in children.iter_descendants(e).chain(std::iter::once(e)) {
                        if colliders.contains(c) {
                            commands.entity(c).insert(ColliderDisabled);
                        }
                    }
                }
            }
        }
        templates.0.insert(t, m);
    }
    debug!("portal2: {} entity templates", templates.0.len());
}

fn spawn_inputs(
    mut commands: Commands,
    mut inputs: MessageReader<EntInput>,
    ents: Option<ResMut<Ents>>,
    templates: Res<Templates>,
    props: Query<&Prop>,
    mut fire: MessageWriter<FireOutput>,
) {
    let Some(mut ents) = ents else {
        inputs.clear();
        return;
    };
    for msg in inputs.read() {
        let class = ents.list[msg.target].class.clone();
        if !matches!(msg.input.as_str(), "forcespawn" | "forcespawnatentityorigin") {
            continue;
        }
        let (template, place) = match class.as_str() {
            "point_template" => (Some(msg.target), None),
            "env_entity_maker" => {
                let name = ents.list[msg.target].get("entitytemplate").unwrap_or("").to_string();
                let t = ents.find(&name, msg.target, None).into_iter().find(|&i| ents.list[i].class == "point_template");
                let mut at = ents.list[msg.target].transform;
                if msg.input == "forcespawnatentityorigin" {
                    if let Some(&o) = ents.find(&msg.param, msg.target, msg.activator).first() {
                        at = ents.list[o].transform;
                    }
                }
                (t, Some(at))
            }
            _ => continue,
        };
        let Some(t) = template else {
            fire.write(FireOutput { ent: msg.target, event: "OnEntityFailedSpawn".into(), activator: msg.activator });
            continue;
        };
        let origin = ents.list[t].transform;
        for &proto in templates.0.get(&t).map(Vec::as_slice).unwrap_or(&[]) {
            let placed = ents.list[proto].transform;
            let tf = match place {
                // Keep its pose relative to the template, moved to the maker.
                Some(at) => {
                    let rel = origin.compute_affine().inverse() * placed.compute_affine();
                    Transform::from_matrix(at.to_matrix() * Mat4::from(rel)).with_scale(placed.scale)
                }
                None => placed,
            };
            let mut ent = ents.list[proto].clone();
            ent.killed = false;
            ent.enabled = !ent.get("startdisabled").is_some_and(|v| v.trim() == "1");
            ent.bounds = ent.bounds.map(|(lo, hi)| (lo - placed.translation + tf.translation, hi - placed.translation + tf.translation));
            ent.transform = tf;
            let idx = ents.list.len();
            if let Some(e) = ents.list[proto].entity {
                let copy = commands
                    .entity(e)
                    .clone_and_spawn_with_opt_out(|b| {
                        b.linked_cloning(true).deny::<(RigidBodyDisabled, ColliderDisabled, ColliderOf, RigidBodyColliders)>();
                    })
                    .id();
                commands.entity(copy).insert((tf, Visibility::Inherited));
                if let Ok(p) = props.get(e) {
                    commands.entity(copy).insert((
                        Prop { ent: idx, half: p.half },
                        Position(tf.translation),
                        Rotation(tf.rotation),
                        LinearVelocity::ZERO,
                        AngularVelocity::ZERO,
                    ));
                }
                ent.entity = Some(copy);
            }
            ents.add(ent);
            debug!("portal2: spawned {} {:?} from template {:?}", ents.list[idx].class, ents.list[idx].name, ents.list[t].name);
        }
        fire.write(FireOutput { ent: msg.target, event: "OnEntitySpawned".into(), activator: msg.activator });
        if place.is_some() {
            fire.write(FireOutput { ent: t, event: "OnEntitySpawned".into(), activator: msg.activator });
        }
    }
}
