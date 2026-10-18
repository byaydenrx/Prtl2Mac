//! Key bindings as Portal 2 keeps them: `bind "key" "command"` lines. The
//! list on Options > Keyboard/Mouse > Customize comes from the game's own
//! `scripts/kb_act.lst` (labels from its `resource/*_english.txt`), and the
//! defaults from its `cfg/config_default.cfg`; built-in copies stand in when
//! a file is missing. The player's own table is saved in settings.cfg and
//! turned into [`p2_core::InputBindings`] for the input systems.
//!
//! Commands read today: movement (`+forward` `+back` `+moveleft`
//! `+moveright` `+jump` `+duck` `+use`) in `p2_movement::input`.
//! TODO: `+attack`/`+attack2` (p2_gun still reads mouse buttons directly),
//! `jpeg`, `save_quick`/`load_quick`, `toggleconsole`, `pause` and the co-op
//! commands are saved and listed but nothing reads them yet.
use std::collections::{BTreeMap, HashMap};

use bevy::prelude::*;
use p2_assets::GameFsRes;
use p2_core::{InputBindings, InputButton};

use crate::settings::Settings;

pub struct BindingsPlugin;

impl Plugin for BindingsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DefaultBinds(BindTable::builtin()))
            .insert_resource(KeyActions(builtin_actions()))
            .init_resource::<InputBindings>()
            .add_systems(Startup, load_game_bindings)
            .add_systems(Update, apply_bindings);
    }
}

/// Key name (Source spelling, lowercase) to command, one command per key.
/// Key names this build doesn't know (`joy1`) are kept as they are.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BindTable(pub BTreeMap<String, String>);

/// The game's default table (`cfg/config_default.cfg`).
#[derive(Resource, Clone, Debug)]
pub struct DefaultBinds(pub BindTable);

/// The rows of the key list, in the game's order.
#[derive(Resource, Clone, Debug)]
pub struct KeyActions(pub Vec<KeyAction>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyAction {
    /// A section title ("Movement").
    Heading(String),
    /// A bindable command and its label.
    Command { command: String, label: String },
}

/// The quoted or bare words of a config line, up to a `//` comment.
fn words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else if c == '"' {
            chars.next();
            out.push(chars.by_ref().take_while(|c| *c != '"').collect());
        } else if c == '/' && line.contains("//") && chars.clone().nth(1) == Some('/') {
            break;
        } else {
            let mut w = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() || c == '"' {
                    break;
                }
                w.push(c);
                chars.next();
            }
            out.push(w);
        }
    }
    out
}

/// `bind "w" "+forward"` (quotes optional) to (`w`, `+forward`).
pub fn parse_bind(line: &str) -> Option<(String, String)> {
    let w = words(line);
    match w.as_slice() {
        [b, key, cmd, ..] if b.eq_ignore_ascii_case("bind") && !key.is_empty() && !cmd.is_empty() => Some((key.to_ascii_lowercase(), cmd.clone())),
        _ => None,
    }
}

impl BindTable {
    /// Every `bind` line of a config file; `unbindall` starts over.
    pub fn from_config(text: &str) -> Self {
        let mut t = BindTable::default();
        for line in text.lines() {
            if words(line).first().is_some_and(|w| w.eq_ignore_ascii_case("unbindall")) {
                t.0.clear();
            } else if let Some((k, c)) = parse_bind(line) {
                t.0.insert(k, c);
            }
        }
        t
    }

    /// Portal 2's PC defaults, for when the game's config_default.cfg can't
    /// be read. Matches the keys the controller used before bindings existed.
    pub fn builtin() -> Self {
        let pairs = [
            ("w", "+forward"),
            ("s", "+back"),
            ("a", "+moveleft"),
            ("d", "+moveright"),
            ("space", "+jump"),
            ("ctrl", "+duck"),
            ("rctrl", "+duck"),
            ("e", "+use"),
            ("mouse1", "+attack"),
            ("mouse2", "+attack2"),
            ("f5", "jpeg"),
            ("f6", "save_quick"),
            ("f9", "load_quick"),
            ("`", "toggleconsole"),
            ("pause", "pause"),
            ("escape", "cancelselect"),
        ];
        BindTable(pairs.into_iter().map(|(k, c)| (k.to_string(), c.to_string())).collect())
    }

    /// The keys bound to `command`, as named in the file.
    pub fn keys_for(&self, command: &str) -> Vec<&str> {
        self.0.iter().filter(|(_, c)| c.eq_ignore_ascii_case(command)).map(|(k, _)| k.as_str()).collect()
    }

    /// Set `key` as the one key for `command`, the way the game's dialog
    /// does: the command's old keys are freed, and the key leaves whatever
    /// it was bound to.
    pub fn rebind(&mut self, command: &str, key: &str) {
        self.clear(command);
        self.0.insert(key.to_ascii_lowercase(), command.to_string());
    }

    /// Unbind every key of `command`.
    pub fn clear(&mut self, command: &str) {
        self.0.retain(|_, c| !c.eq_ignore_ascii_case(command));
    }

    /// What the input systems read: the known keys, by command.
    pub fn to_input(&self) -> InputBindings {
        let mut map: HashMap<String, Vec<InputButton>> = HashMap::new();
        for (k, c) in &self.0 {
            if let Some(b) = InputButton::from_name(k) {
                map.entry(c.to_ascii_lowercase()).or_default().push(b);
            }
        }
        InputBindings(map)
    }

    /// settings.cfg lines: `unbindall` then one `bind` per key, as Source
    /// writes config.cfg.
    pub fn to_lines(&self) -> String {
        let mut s = String::from("unbindall\n");
        for (k, c) in &self.0 {
            s.push_str(&format!("bind \"{k}\" \"{c}\"\n"));
        }
        s
    }
}

/// How a key is shown in the list: `W`, `SPACE`, `MOUSE1`.
pub fn key_label(key: &str) -> String {
    key.to_uppercase()
}

/// The key list from `kb_act.lst`: `"command" "#Token"` lines, where
/// `"blank"` lines are separators or (with a token) section titles.
pub fn parse_kb_act(text: &str, tr: &dyn Fn(&str) -> String) -> Vec<KeyAction> {
    let mut out = Vec::new();
    for line in text.lines() {
        let w = words(line);
        let [cmd, label, ..] = w.as_slice() else { continue };
        if cmd.eq_ignore_ascii_case("blank") {
            if !label.starts_with('=') && !label.is_empty() {
                out.push(KeyAction::Heading(tr(label)));
            }
        } else {
            out.push(KeyAction::Command { command: cmd.clone(), label: tr(label) });
        }
    }
    out
}

/// The built-in key list, in Portal 2's groups.
pub fn builtin_actions() -> Vec<KeyAction> {
    let h = |s: &str| KeyAction::Heading(s.into());
    let c = |cmd: &str, label: &str| KeyAction::Command { command: cmd.into(), label: label.into() };
    vec![
        h("Movement"),
        c("+forward", "Move forward"),
        c("+back", "Move back"),
        c("+moveleft", "Move left (strafe)"),
        c("+moveright", "Move right (strafe)"),
        c("+jump", "Jump"),
        c("+duck", "Duck"),
        c("+use", "Use item"),
        h("Portal gun"),
        c("+attack", "Fire blue portal"),
        c("+attack2", "Fire orange portal"),
        h("Miscellaneous"),
        c("jpeg", "Take screen shot"),
        c("save_quick", "Quick save"),
        c("load_quick", "Quick load"),
        c("pause", "Pause game"),
        c("toggleconsole", "Developer console"),
    ]
}

/// Localized strings from the game's `resource/<name>_english.txt` files
/// (`"lang" { "Tokens" { "Key" "Text" } }`), keyed lowercase.
pub fn read_tokens(text: &str, out: &mut HashMap<String, String>) {
    let kv = source_assets::kv::parse(text);
    let Some((_, root)) = kv.root() else { return };
    let Some(tokens) = root.block("tokens").or_else(|| root.block("Tokens")) else { return };
    for (k, v) in &tokens.entries {
        if let source_assets::kv::KvValue::Str(s) = v {
            out.entry(k.to_ascii_lowercase()).or_insert_with(|| s.clone());
        }
    }
}

/// `#Valve_Jump` to its text; a token the files don't have stays as it is.
fn translate(tokens: &HashMap<String, String>, label: &str) -> String {
    label.strip_prefix('#').and_then(|t| tokens.get(&t.to_ascii_lowercase()).cloned()).unwrap_or_else(|| label.to_string())
}

/// An untranslated `#Valve_Move_Forward` made readable: `Move Forward`.
fn untoken(label: &str) -> String {
    let t = label.trim_start_matches('#');
    let t = ["Valve_", "PORTAL2_", "Portal2_", "GameUI_"].iter().find_map(|p| t.strip_prefix(p)).unwrap_or(t);
    t.replace('_', " ")
}

/// UTF-16 (with BOM) or UTF-8 text.
pub fn decode_text(b: &[u8]) -> String {
    if b.len() >= 2 && b[0] == 0xFF && b[1] == 0xFE {
        let units: Vec<u16> = b[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(b).into_owned()
    }
}

fn load_game_bindings(game: Res<GameFsRes>, mut defaults: ResMut<DefaultBinds>, mut actions: ResMut<KeyActions>) {
    if let Some(b) = game.read("cfg/config_default.cfg") {
        let t = BindTable::from_config(&decode_text(&b));
        if !t.0.is_empty() {
            info!("portal2: {} default key bindings from cfg/config_default.cfg", t.0.len());
            defaults.0 = t;
        }
    }
    let mut tokens = HashMap::new();
    for f in ["resource/portal2_english.txt", "resource/basemodui_english.txt", "resource/gameui_english.txt", "resource/valve_english.txt"] {
        if let Some(b) = game.read(f) {
            read_tokens(&decode_text(&b), &mut tokens);
        }
    }
    if let Some(b) = game.read("scripts/kb_act.lst") {
        // A label the game's files don't have falls back to ours for that command.
        let builtin = builtin_actions();
        let ours = |cmd: &str| builtin.iter().find_map(|a| match a {
            KeyAction::Command { command, label } if command == cmd => Some(label.clone()),
            _ => None,
        });
        let mut list = parse_kb_act(&decode_text(&b), &|l| translate(&tokens, l));
        for a in &mut list {
            match a {
                KeyAction::Command { command, label } if label.starts_with('#') => *label = ours(command).unwrap_or_else(|| untoken(label)),
                KeyAction::Heading(h) if h.starts_with('#') => *h = untoken(h),
                _ => {}
            }
        }
        if list.iter().any(|a| matches!(a, KeyAction::Command { .. })) {
            info!("portal2: {} key list rows from scripts/kb_act.lst", list.len());
            actions.0 = list;
        }
    }
}

/// The player's table (or the defaults) into what the input systems read.
fn apply_bindings(settings: Res<Settings>, defaults: Res<DefaultBinds>, mut input: ResMut<InputBindings>) {
    if !settings.is_changed() && !defaults.is_changed() {
        return;
    }
    let want = settings.binds.as_ref().unwrap_or(&defaults.0).to_input();
    if *input != want {
        *input = want;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_lines() {
        assert_eq!(parse_bind(r#"bind "W" "+forward""#), Some(("w".into(), "+forward".into())));
        assert_eq!(parse_bind("bind space +jump // comment"), Some(("space".into(), "+jump".into())));
        assert_eq!(parse_bind(r#"bind "`" "toggleconsole""#), Some(("`".into(), "toggleconsole".into())));
        assert_eq!(parse_bind(r#"bind "/" "+voicerecord""#), Some(("/".into(), "+voicerecord".into())));
        assert_eq!(parse_bind("bind w"), None);
        assert_eq!(parse_bind("sensitivity 3"), None);
    }

    #[test]
    fn config_reads_binds_and_unbindall() {
        let t = BindTable::from_config("bind x +jump\nunbindall\nsensitivity \"3\"\nbind \"SPACE\" \"+jump\"\nbind \"CTRL\" \"+duck\"\n");
        assert_eq!(t.0.len(), 2);
        assert_eq!(t.keys_for("+jump"), vec!["space"]);
        assert_eq!(BindTable::from_config(&t.to_lines()), t);
    }

    #[test]
    fn builtin_defaults_are_todays_keys() {
        assert_eq!(BindTable::builtin().to_input().0.get("+duck").map(Vec::len), Some(2));
        for cmd in ["+forward", "+back", "+moveleft", "+moveright", "+jump", "+duck", "+use"] {
            let mut a = BindTable::builtin().to_input().buttons(cmd).to_vec();
            let mut b = InputBindings::default().buttons(cmd).to_vec();
            a.sort_by_key(|x| format!("{x:?}"));
            b.sort_by_key(|x| format!("{x:?}"));
            assert_eq!(a, b, "{cmd}");
        }
    }

    #[test]
    fn rebinding_moves_keys() {
        let mut t = BindTable::builtin();
        // Space goes from jump to use; use's old key E is freed; jump has none.
        t.rebind("+use", "SPACE");
        assert_eq!(t.keys_for("+use"), vec!["space"]);
        assert!(t.keys_for("+jump").is_empty());
        assert!(!t.0.contains_key("e"));
        t.clear("+duck");
        assert!(t.keys_for("+duck").is_empty());
        // Unknown key names survive.
        t.0.insert("joy1".into(), "+jump".into());
        assert_eq!(BindTable::from_config(&t.to_lines()), t);
        assert!(t.to_input().buttons("+jump").is_empty());
    }

    #[test]
    fn kb_act_list() {
        let text = "\"blank\"\t\"=========================\"\n\"blank\"\t\"#Valve_Movement_Title\"\n\"blank\"\t\"=========================\"\n\"+forward\"\t\"#Valve_Move_Forward\"\n\"+jump\"\t\"Jump!\"\n";
        let mut tokens = HashMap::new();
        tokens.insert("valve_movement_title".to_string(), "Movement".to_string());
        tokens.insert("valve_move_forward".to_string(), "Move forward".to_string());
        let list = parse_kb_act(text, &|l| translate(&tokens, l));
        assert_eq!(
            list,
            vec![
                KeyAction::Heading("Movement".into()),
                KeyAction::Command { command: "+forward".into(), label: "Move forward".into() },
                KeyAction::Command { command: "+jump".into(), label: "Jump!".into() },
            ]
        );
    }

    #[test]
    fn token_files() {
        let mut out = HashMap::new();
        read_tokens("\"lang\"\n{\n\"Language\" \"English\"\n\"Tokens\"\n{\n\"Valve_Jump\" \"Jump\"\n}\n}\n", &mut out);
        assert_eq!(out.get("valve_jump").map(String::as_str), Some("Jump"));
        assert_eq!(translate(&out, "#Valve_Jump"), "Jump");
        assert_eq!(translate(&out, "#Valve_Missing_Thing"), "#Valve_Missing_Thing");
        assert_eq!(untoken("#Valve_Move_Forward"), "Move Forward");
    }
}
