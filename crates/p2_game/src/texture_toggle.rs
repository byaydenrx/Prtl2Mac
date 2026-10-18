//! `env_texturetoggle`: `SetTextureIndex N` / `IncrementTextureIndex` set
//! the texture frame index of the brushes, props and overlays its `target`
//! names. Their materials' `ToggleTexture` proxy draws that frame
//! (`source_assets::proxy`), which is how Portal 2's antlines and checkmark
//! indicator lights switch from blue to orange when a puzzle is solved.
//! A `prop_indicator_panel`'s `Check` / `Uncheck` sets the toggle named by
//! its `indicatorlights` to 1 / 0 the same way.
//!
//! Named `info_overlay`s reach the entity list as `info_overlay_accessor`s
//! whose `overlayid` names the overlay ([`SourceOverlay`]).
use std::collections::HashMap;

use bevy::prelude::*;
use source_assets::proxy::{SourceOverlay, TextureFrames};

use crate::io::{EntInput, Ents};
use crate::level::{GameSet, LevelLoaded};

pub struct TextureTogglePlugin;

impl Plugin for TextureTogglePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, texture_toggle_inputs.after(crate::io::run_io).in_set(GameSet::Logic));
    }
}

/// The frame index an input leaves, or `None` for other inputs.
pub fn toggle_index(input: &str, param: &str, current: u32) -> Option<u32> {
    match input {
        // like the engine's atoi: "2", "2.0" and junk-free numbers
        "settextureindex" => Some(source_assets::kv::parse_f32(param).unwrap_or(0.0).max(0.0) as u32),
        "incrementtextureindex" => Some(current.saturating_add(1)),
        _ => None,
    }
}

/// The drawn entities an entity stands for: its spawned brush/prop, or the
/// surfaces of the overlay an `info_overlay_accessor` names.
fn drawn(ents: &Ents, i: usize, overlay: &dyn Fn(i32) -> Vec<Entity>) -> Vec<Entity> {
    let e = &ents.list[i];
    if e.class == "info_overlay_accessor" {
        e.get("overlayid").and_then(|v| v.trim().parse().ok()).map(overlay).unwrap_or_default()
    } else {
        e.entity.into_iter().collect()
    }
}

/// What a toggle switches: the entities its `target` names.
pub fn toggle_targets(ents: &Ents, toggle: usize, activator: Option<usize>, overlay: &dyn Fn(i32) -> Vec<Entity>) -> Vec<Entity> {
    let Some(target) = ents.list[toggle].get("target") else { return Vec::new() };
    ents.find(target, toggle, activator).into_iter().flat_map(|t| drawn(ents, t, overlay)).collect()
}

/// Apply one input to entity `target` (a toggle or an indicator panel).
pub fn handle_input(
    ents: &Ents,
    frames: &mut HashMap<Entity, u32>,
    target: usize,
    input: &str,
    param: &str,
    activator: Option<usize>,
    overlay: &dyn Fn(i32) -> Vec<Entity>,
) {
    let e = &ents.list[target];
    match e.class.as_str() {
        "env_texturetoggle" => {
            for d in toggle_targets(ents, target, activator, overlay) {
                let cur = frames.get(&d).copied().unwrap_or(0);
                if let Some(v) = toggle_index(input, param, cur) {
                    frames.insert(d, v);
                }
            }
        }
        "prop_indicator_panel" => {
            let index = match input {
                "check" => "1",
                "uncheck" => "0",
                _ => return,
            };
            let Some(lights) = e.get("indicatorlights").filter(|n| !n.trim().is_empty()) else { return };
            for t in ents.find(lights, target, activator) {
                if ents.list[t].class == "env_texturetoggle" {
                    handle_input(ents, frames, t, "settextureindex", index, activator, overlay);
                } else {
                    // the panel may name the lights themselves
                    for d in drawn(ents, t, overlay) {
                        frames.insert(d, index.parse().unwrap_or(0));
                    }
                }
            }
        }
        _ => {}
    }
}

fn texture_toggle_inputs(
    mut inputs: MessageReader<EntInput>,
    mut loaded: MessageReader<LevelLoaded>,
    ents: Option<Res<Ents>>,
    mut frames: ResMut<TextureFrames>,
    overlays: Query<(Entity, &SourceOverlay)>,
) {
    if loaded.read().count() > 0 {
        frames.0.clear();
    }
    let Some(ents) = ents else {
        inputs.clear();
        return;
    };
    let overlay = |id: i32| overlays.iter().filter(|(_, o)| o.id == id).map(|(e, _)| e).collect::<Vec<_>>();
    for msg in inputs.read() {
        if matches!(ents.list.get(msg.target).map(|e| e.class.as_str()), Some("env_texturetoggle" | "prop_indicator_panel")) {
            handle_input(&ents, &mut frames.0, msg.target, &msg.input, &msg.param, msg.activator, &overlay);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::GameEnt;

    fn ent(class: &str, name: &str, kv: &[(&str, &str)], entity: Option<u64>) -> GameEnt {
        let mut e = GameEnt::logic(class, name, kv.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect());
        e.entity = entity.map(|b| Entity::from_bits(b | 1 << 32));
        e
    }

    fn id(b: u64) -> Entity {
        Entity::from_bits(b | 1 << 32)
    }

    #[test]
    fn index_inputs() {
        assert_eq!(toggle_index("settextureindex", "1", 0), Some(1));
        assert_eq!(toggle_index("settextureindex", "0", 5), Some(0));
        assert_eq!(toggle_index("settextureindex", "", 5), Some(0));
        assert_eq!(toggle_index("settextureindex", "-3", 5), Some(0));
        assert_eq!(toggle_index("incrementtextureindex", "", 1), Some(2));
        assert_eq!(toggle_index("enable", "", 1), None);
    }

    #[test]
    fn toggles_brushes_props_and_overlays() {
        let mut ents = Ents::default();
        ents.add(ent("player", "!player", &[], None));
        let toggle = ents.add(ent("env_texturetoggle", "lights_toggle", &[("target", "lights*")], None));
        ents.add(ent("func_brush", "lights_sign", &[], Some(7)));
        ents.add(ent("info_overlay_accessor", "lights_floor", &[("overlayid", "12")], None));
        ents.add(ent("prop_dynamic", "other", &[], Some(9)));
        let overlay = |oid: i32| if oid == 12 { vec![id(20), id(21)] } else { Vec::new() };
        let mut frames = HashMap::new();

        handle_input(&ents, &mut frames, toggle, "settextureindex", "1", None, &overlay);
        assert_eq!(frames.get(&id(7)), Some(&1));
        assert_eq!(frames.get(&id(20)), Some(&1));
        assert_eq!(frames.get(&id(21)), Some(&1));
        assert_eq!(frames.get(&id(9)), None, "not a target");

        handle_input(&ents, &mut frames, toggle, "incrementtextureindex", "", None, &overlay);
        assert_eq!(frames.get(&id(7)), Some(&2));
        handle_input(&ents, &mut frames, toggle, "settextureindex", "0", None, &overlay);
        assert_eq!(frames.get(&id(20)), Some(&0));
        // other inputs leave it alone
        handle_input(&ents, &mut frames, toggle, "kill", "", None, &overlay);
        assert_eq!(frames.get(&id(7)), Some(&0));
    }

    #[test]
    fn indicator_panel_checks_its_lights() {
        let mut ents = Ents::default();
        ents.add(ent("player", "!player", &[], None));
        ents.add(ent("env_texturetoggle", "door_lights", &[("target", "door_overlay")], None));
        ents.add(ent("info_overlay_accessor", "door_overlay", &[("overlayid", "3")], None));
        let panel = ents.add(ent("prop_indicator_panel", "panel", &[("indicatorlights", "door_lights")], None));
        let overlay = |oid: i32| if oid == 3 { vec![id(30)] } else { Vec::new() };
        let mut frames = HashMap::new();
        handle_input(&ents, &mut frames, panel, "check", "", None, &overlay);
        assert_eq!(frames.get(&id(30)), Some(&1));
        handle_input(&ents, &mut frames, panel, "uncheck", "", None, &overlay);
        assert_eq!(frames.get(&id(30)), Some(&0));
        handle_input(&ents, &mut frames, panel, "start", "", None, &overlay);
        assert_eq!(frames.get(&id(30)), Some(&0));
    }
}
