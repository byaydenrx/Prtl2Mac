//! Portal 2 on the p2 crates: the real levels from the player's own install,
//! played with the Portal 2 player, portal gun and portals, wired up by the
//! levels' own entity I/O, with menus, music and saves.
//!
//! `portal2` opens the main menu; `portal2 MAP` starts that map directly.
//! Harness flags (`--screenshot`, `--frames`, `--shot`, `--exit-after`,
//! `--size`) come from `p2_room::HarnessArgs`.
use avian3d::prelude::*;
use bevy::prelude::*;
use p2_core::*;
use source_assets::{SourceMapPlugin, SourceRenderSettings};

mod ai;
mod anim_props;
mod audio;
mod autopilot;
mod bindings;
mod bridges;
mod campaign;
mod captions;
mod community;
mod console;
mod exposure;
mod movies;
mod health;
mod io;
mod lasers;
mod level;
mod map_particles;
mod mods_page;
mod mounts;
mod movers;
mod options;
mod overlays;
mod paint;
mod paint_props;
mod portals;
mod projected_lights;
mod props;
mod saves;
mod scenes;
mod settings;
mod speedrun;
mod splits;
mod sprites;
mod tanks;
mod tool_settings;
#[cfg(test)]
mod tools_tests;
mod templates;
mod texture_toggle;
mod tractor;
mod triggers;
mod turrets;
mod ui;
mod viewcontrol;
mod vscripts;

/// What the command line asked for.
#[derive(Resource, Clone, Debug, Default)]
pub struct Launch {
    /// Start this map instead of the main menu.
    pub map: Option<String>,
}

fn main() {
    // Portal 2 is required: check before any window opens.
    let game = p2_assets::find_portal2().unwrap_or_else(|msg| {
        eprintln!("{msg}");
        std::process::exit(1);
    });
    // Enabled mods ahead of the install, community maps behind it.
    let (game, source_config, mounts, community) = mounts::mount(game);
    let args = p2_room::HarnessArgs::from_env();
    let launch = Launch { map: args.rest.iter().find(|a| !a.starts_with("--")).cloned() };
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(args.window_plugin("Portal 2")))
        .insert_resource(P2Settings { set_fixed_timestep: true })
        .insert_resource(p2_assets::GameFsRes(std::sync::Arc::new(game)))
        .insert_resource(SourceRenderSettings::default())
        .insert_resource(source_assets::valve_render::ValveExposure(source_assets::DEFAULT_LIGHT_SCALE))
        .add_plugins(PhysicsPlugins::default().with_collision_hooks::<props::PortalHooks>())
        .add_plugins(SourceMapPlugin { config: source_config })
        .add_plugins(mounts::MountsPlugin { mounts, community })
        .add_plugins(p2_portals::PortalPlugins)
        .insert_resource(if launch.map.is_some() { ui::Screen::None } else { ui::Screen::Main })
        .insert_resource(launch)
        .add_message::<io::EntInput>()
        .add_message::<io::FireOutput>()
        .add_systems(Startup, campaign::load_campaign)
        .add_systems(Update, io::run_io.in_set(level::GameSet::Logic))
        .add_plugins((
            settings::SettingsPlugin,
            level::LevelPlugin,
            movers::MoversPlugin,
            portals::MapPortalsPlugin,
            props::PropsPlugin,
            triggers::TriggersPlugin,
            audio::GameAudioPlugin,
            autopilot::AutopilotPlugin,
            scenes::ScenesPlugin,
            ai::AiPlugin,
            viewcontrol::ViewControlPlugin,
            anim_props::AnimPropsPlugin,
            captions::CaptionsPlugin,
        ))
        .add_plugins(bindings::BindingsPlugin)
        .add_plugins((tanks::TanksPlugin, sprites::SpritesPlugin))
        .add_plugins((
            templates::TemplatesPlugin,
            health::HealthPlugin,
            lasers::LasersPlugin,
            bridges::BridgesPlugin,
            tractor::TractorPlugin,
            vscripts::VScriptPlugin,
            exposure::ExposurePlugin,
            paint::PaintPlugin,
            paint_props::PaintPropsPlugin,
            map_particles::MapParticlesPlugin,
            texture_toggle::TextureTogglePlugin,
            movies::MoviesPlugin,
            projected_lights::ProjectedLightsPlugin,
        ))
        // Developer, speedrun and modding tools (all off until turned on in Options).
        .add_plugins((console::ConsolePlugin, overlays::OverlaysPlugin, speedrun::SpeedrunPlugin))
        .add_plugins(saves::SavesPlugin)
        .insert_resource(args)
        .add_plugins(p2_room::HarnessPlugin);
    // UI after the Screen resource above (its own default is the main menu).
    app.add_plugins(UiPluginKeepScreen).run();
}

/// [`ui::UiPlugin`] without overwriting the launch screen.
struct UiPluginKeepScreen;

impl Plugin for UiPluginKeepScreen {
    fn build(&self, app: &mut App) {
        let screen = *app.world().resource::<ui::Screen>();
        app.add_plugins(ui::UiPlugin);
        app.insert_resource(screen);
    }
}
