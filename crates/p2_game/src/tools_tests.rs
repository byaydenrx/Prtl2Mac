//! Headless runs of the tool plugins (console, overlays, speedrun) in a bare
//! app: Bevy only checks that systems' queries and resources fit together
//! when the app runs, so this catches what `cargo check` can't. No window,
//! no Portal 2 install (an empty filesystem).
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use p2_core::*;

use crate::campaign::{Campaign, Chapter};
use crate::console::{Cheats, Console, ConsoleSubmit};
use crate::level::{CurrentMap, GameSet, LevelLoaded, LoadLevel};
use crate::settings::Settings;
use crate::speedrun::{PracticeRequest, Speedrun};
use crate::splits::RunState;
use crate::tool_settings::ToolSettings;

/// Tests that point `P2_DATA_DIR` somewhere hold this.
pub static ENV_LOCK: Mutex<()> = Mutex::new(());

fn temp(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("p2-tools-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

/// A mounted filesystem with nothing in it.
fn empty_fs(root: &std::path::Path) -> source_assets::GameFs {
    std::fs::create_dir_all(root.join("platform")).unwrap();
    source_assets::GameFs::mount(source_assets::config::GameConfig::from_root(root)).unwrap()
}

fn chapter(map: &str) -> Chapter {
    Chapter { map: map.into(), title: String::new(), subtitle: String::new(), picture: String::new(), display_on_spawn: false, display_delay: 0.0 }
}

fn map_info(name: &str) -> source_assets::MapInfo {
    source_assets::MapInfo {
        name: name.into(),
        root: Entity::PLACEHOLDER,
        offset: Vec3::ZERO,
        player_start: Transform::IDENTITY,
        arrival: Transform::IDENTITY,
        entities: Vec::new(),
        light_environment: None,
        fog: None,
        skybox: None,
        skyname: None,
        bounds: (Vec3::ZERO, Vec3::ZERO),
        convert_seconds: 0.0,
    }
}

fn app(root: &std::path::Path) -> App {
    let mut app = App::new();
    // A system whose parameters don't fit (a missing resource) fails the test.
    app.set_error_handler(bevy::ecs::error::panic);
    app.add_plugins((MinimalPlugins, bevy::input::InputPlugin, bevy::asset::AssetPlugin::default()))
        .init_asset::<Font>()
        .add_plugins(p2_core::CorePlugin)
        .configure_sets(Update, (GameSet::Load, GameSet::Logic).chain())
        .add_message::<LoadLevel>()
        .add_message::<LevelLoaded>()
        .add_message::<crate::triggers::LevelExit>()
        .add_message::<crate::vscripts::ConsoleCommand>()
        .insert_resource(Settings { tools: ToolSettings { dev_console: true, speedrun_timer: true, practice: true, show_fps: true, show_pos: true, ..default() }, ..default() })
        .insert_resource(Campaign { maps: vec!["a1".into(), "a2".into(), "b1".into()], chapters: vec![chapter("a1"), chapter("b1")], ..default() })
        .insert_resource(p2_assets::GameFsRes(Arc::new(empty_fs(root))))
        .insert_resource(crate::community::CommunityMaps::default())
        .insert_resource(crate::mounts::Mounts::default())
        .insert_resource(crate::ui::Screen::None)
        .insert_resource(p2_room::HarnessArgs::default())
        .init_resource::<crate::health::PlayerHealth>()
        .add_plugins((crate::console::ConsolePlugin, crate::overlays::OverlaysPlugin, crate::speedrun::SpeedrunPlugin));
    app
}

fn enter_map(app: &mut App, name: &str) {
    let info = map_info(name);
    let ents = crate::io::Ents::from_map(&info, &HashMap::new());
    app.world_mut().insert_resource(ents);
    app.world_mut().insert_resource(CurrentMap { info, menu: false, model_bounds: HashMap::new() });
    app.world_mut().write_message(LevelLoaded { map: name.into(), menu: false });
}

fn key(app: &mut App, code: KeyCode, text: &str) {
    app.world_mut().write_message(KeyboardInput {
        key_code: code,
        logical_key: Key::Character(text.into()),
        state: ButtonState::Pressed,
        text: Some(text.into()),
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
}

fn submit(app: &mut App, line: &str) {
    app.world_mut().write_message(ConsoleSubmit(line.into()));
    app.update();
}

fn feet(app: &mut App) -> Vec3 {
    let w = app.world_mut();
    let mut q = w.query_filtered::<&Transform, With<Player>>();
    q.single(w).unwrap().translation
}

#[test]
fn tools_run_together() {
    let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = temp("app");
    std::env::set_var("P2_DATA_DIR", root.join("data"));
    let mut app = app(&root);
    // A few empty frames: every system's parameters are valid.
    for _ in 0..3 {
        app.update();
    }
    {
        let mut c = app.world_mut().commands();
        spawn_player(&mut c, Vec3::ZERO, 0.0);
    }
    app.world_mut().flush();
    enter_map(&mut app, "a1");
    app.update();
    assert_eq!(app.world().resource::<Speedrun>().run.state, RunState::Running);

    // Cheats need sv_cheats.
    submit(&mut app, "setpos 100 200 300");
    assert_eq!(feet(&mut app), Vec3::ZERO);
    submit(&mut app, "sv_cheats 1; setpos 100 200 300; noclip; god; cl_showpos 0");
    assert!((feet(&mut app) - source_to_world(Vec3::new(100.0, 200.0, 300.0))).length() < 1e-4);
    let cheats = app.world().resource::<Cheats>().clone();
    assert!(cheats.sv_cheats && cheats.noclip && cheats.god);
    assert!(!app.world().resource::<Settings>().tools.show_pos);
    app.update();
    assert!(app.world().resource::<crate::health::PlayerHealth>().hp > 1000.0);
    // The run is spoiled by sv_cheats.
    assert_eq!(app.world().resource::<Speedrun>().run.invalid, Some(crate::splits::Invalid::Cheats));
    submit(&mut app, "fire nobody Open; ent_list; getpos; map nothing_here; cmdlist; host_timescale 2");
    assert_eq!(app.world().resource::<Time<Virtual>>().relative_speed(), 2.0);
    submit(&mut app, "sv_cheats 0");
    assert_eq!(app.world().resource::<Time<Virtual>>().relative_speed(), 1.0);
    assert!(!app.world().resource::<Cheats>().noclip);

    // A practice checkpoint goes to the data folder and back.
    app.world_mut().write_message(PracticeRequest::Save(Some("start".into())));
    app.update();
    assert!(root.join("data/practice/a1.txt").is_file());
    submit(&mut app, "sv_cheats 1; setpos 0 0 0");
    app.world_mut().write_message(PracticeRequest::Load(Some("start".into())));
    app.update();
    assert!((feet(&mut app) - source_to_world(Vec3::new(100.0, 200.0, 300.0))).length() < 1e-3);

    // Leaving the map splits and waits for a2.
    app.world_mut().write_message(crate::triggers::LevelExit);
    app.update();
    let run = &app.world().resource::<Speedrun>().run;
    assert_eq!(run.state, RunState::Loading);
    assert_eq!(run.splits.maps.len(), 1);

    // The console key opens it and pauses the world; Esc closes it.
    key(&mut app, KeyCode::Backquote, "`");
    app.update();
    assert!(app.world().resource::<Console>().open);
    assert!(app.world().resource::<Time<Virtual>>().is_paused());
    assert!(app.world().resource::<InputLocked>().0);
    key(&mut app, KeyCode::KeyN, "n");
    key(&mut app, KeyCode::Tab, "\t");
    key(&mut app, KeyCode::Escape, "");
    app.update();
    assert!(!app.world().resource::<Console>().open);
    assert!(!app.world().resource::<Time<Virtual>>().is_paused());

    // Turned off in Options: the key does nothing.
    app.world_mut().resource_mut::<Settings>().tools.dev_console = false;
    key(&mut app, KeyCode::Backquote, "`");
    app.update();
    assert!(!app.world().resource::<Console>().open);
    std::env::remove_var("P2_DATA_DIR");
    let _ = std::fs::remove_dir_all(&root);
}

/// Press a menu button the way scripted tests do, and let the menu rebuild.
fn click(app: &mut App, action: &str) {
    app.world_mut().write_message(crate::ui::MenuClick(action.into()));
    app.update();
    app.update();
}

#[test]
fn options_pages_work() {
    let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = temp("ui");
    std::env::set_var("P2_DATA_DIR", root.join("data"));
    let mut app = app(&root);
    // The menu's view placement casts rays: an empty collider tree will do.
    app.init_resource::<avian3d::collider_tree::ColliderTrees>()
        .init_asset::<Image>()
        .add_message::<crate::audio::PlayGameSound>()
        .insert_resource(crate::Launch::default())
        .add_plugins(crate::ui::UiPlugin)
        .insert_resource(crate::ui::Screen::Main);
    {
        let mut tools = app.world_mut().resource_mut::<Settings>();
        tools.tools = ToolSettings::default();
    }
    for _ in 0..3 {
        app.update();
    }
    click(&mut app, "Options");
    assert!(matches!(*app.world().resource::<crate::ui::Screen>(), crate::ui::Screen::Options { .. }));
    click(&mut app, "Toggle(DevConsole)");
    click(&mut app, "Toggle(ShowFps)");
    click(&mut app, "Toggle(ShowPos)");
    let t = app.world().resource::<Settings>().tools.clone();
    assert!(t.dev_console && t.show_fps && t.show_pos);
    click(&mut app, "SpeedrunOptions");
    click(&mut app, "Toggle(SpeedrunTimer)");
    click(&mut app, "Adjust(SpeedrunMode,1.0)");
    click(&mut app, "Toggle(QuickRestart)");
    click(&mut app, "Adjust(QuickRestartKey,1.0)");
    click(&mut app, "Toggle(Practice)");
    click(&mut app, "Adjust(SaveCheckpointKey,-1.0)");
    click(&mut app, "Adjust(LoadCheckpointKey,1.0)");
    click(&mut app, "Adjust(CheckpointSlot,1.0)");
    let t = app.world().resource::<Settings>().tools.clone();
    assert!(t.speedrun_timer && t.quick_restart && t.practice);
    assert_eq!(t.speedrun_mode, crate::tool_settings::SpeedrunMode::SingleMap);
    assert_eq!((t.quick_restart_key, t.checkpoint_save_key, t.checkpoint_load_key, t.checkpoint_slot), (KeyCode::F7, KeyCode::F6, KeyCode::F9, 2));
    // BACK from the page saves settings.cfg with the new keys.
    click(&mut app, "Back");
    let cfg = std::fs::read_to_string(root.join("data/settings.cfg")).unwrap();
    assert!(cfg.contains("speedrun_timer 1") && cfg.contains("quick_restart_key F7") && cfg.contains("con_enable 1"), "{cfg}");
    assert!(matches!(*app.world().resource::<crate::ui::Screen>(), crate::ui::Screen::Options { .. }));
    // The Mods page with no mod folder, and back.
    click(&mut app, "ModsOptions");
    assert!(!root.join("data/mods").exists());
    click(&mut app, "Back");
    click(&mut app, "Back");
    assert_eq!(*app.world().resource::<crate::ui::Screen>(), crate::ui::Screen::Main);
    // Chapter select with a community map: its tab, then start it.
    std::fs::create_dir_all(root.join("data/maps")).unwrap();
    std::fs::write(root.join("data/maps/my_map.bsp"), b"VBSP").unwrap();
    app.world_mut().resource_mut::<crate::community::CommunityMaps>().folders = vec![root.join("data/maps")];
    click(&mut app, "Chapters");
    click(&mut app, "ChapterTab(true)");
    click(&mut app, "CommunityMap(0)");
    assert_eq!(*app.world().resource::<crate::ui::Screen>(), crate::ui::Screen::None);
    let loads: Vec<String> = app.world_mut().resource_mut::<Messages<LoadLevel>>().drain().map(|l| l.map).collect();
    assert!(loads.iter().any(|m| m.ends_with("my_map.bsp")), "{loads:?}");
    std::env::remove_var("P2_DATA_DIR");
    let _ = std::fs::remove_dir_all(&root);
}
