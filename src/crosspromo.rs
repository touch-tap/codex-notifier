//! The one place codex-notifier mentions its sister apps: Claude Code Notifier (computer side
//! ccn) for Claude Code, and Antigravity Notifier (computer side antigravity-notifier) for Google
//! Antigravity's `agy`. One of them is suggested only to someone who has that agent and not its
//! notifier already (Claude Code Notifier first when both qualify, never both at once), and only
//! where the user is looking at codex-notifier anyway: the end of every `codex-notifier pair`
//! (with a QR code: the phone is in their hand), a hint in `status` and `doctor`, and at most
//! twice ever in Codex at SessionStart (with a desktop notification), whichever app is named.
//! ccn's and antigravity-notifier's own copies of this module suggest the others the same way.
//!
//! Detection is a few `stat` calls and at most two small file reads: no network.

use rust_i18n::t;
use serde_json::{Value, json};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::config::{self, Config};
use crate::qr;

const DAY: i64 = 24 * 60 * 60;
/// The first SessionStart mention waits this long after pairing
const FIRST_AFTER: i64 = 3 * DAY;
/// The second waits this long after the first
const SECOND_AFTER: i64 = 30 * DAY;
/// SessionStart mentions one of them no more often than this, ever (both together)
const SESSION_MAX: u32 = 2;

/// Claude Code Notifier's addresses. /get sends a phone straight to the App Store or Google Play
/// (and shows a computer both store buttons).
mod claude_code {
    pub const UNIX_COMMAND: &str = "curl -fsSL https://claudecodenotifier.com/install.sh | sh";
    pub const WINDOWS_COMMAND: &str =
        "irm https://claudecodenotifier.com/install.ps1 | iex; ccn pair";
    /// The website, whose buttons take a phone to the App Store or Google Play
    pub const URL: &str = "https://claudecodenotifier.com/?ref=codex-cli";
    /// The QR code and link after `codex-notifier pair`
    pub const PAIR_URL: &str = "https://claudecodenotifier.com/get?ref=codex-pair";
    /// Where a click on the SessionStart notification goes
    pub const NOTIFICATION_URL: &str = "https://claudecodenotifier.com/?ref=codex-notification";
}

/// Antigravity Notifier's addresses. All three are the website itself, never a store link or
/// /get: its phone apps were still in store review when this was written (27 Nov 2026), and the
/// website says where the apps are as soon as they are there.
mod antigravity {
    pub const UNIX_COMMAND: &str = "curl -fsSL https://antigravitynotifier.com/install.sh | sh";
    pub const WINDOWS_COMMAND: &str =
        "irm https://antigravitynotifier.com/install.ps1 | iex; antigravity-notifier pair";
    pub const URL: &str = "https://antigravitynotifier.com/?ref=codex-cli";
    /// The QR code and link after `codex-notifier pair`
    pub const PAIR_URL: &str = "https://antigravitynotifier.com/?ref=codex-pair";
    /// Where a click on the SessionStart notification goes
    pub const NOTIFICATION_URL: &str = "https://antigravitynotifier.com/?ref=codex-notification";
}

/// A sister app that may be suggested. One line installs its CLI, sets up its agent's hooks and
/// pairs: each install script ends in `pair`, whose QR code also offers the app to a phone
/// without it. No Node needed, unlike npm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sister {
    /// Claude Code Notifier, for Claude Code
    ClaudeCode,
    /// Antigravity Notifier, for Google Antigravity (`agy`)
    Antigravity,
}

impl Sister {
    pub const ALL: [Sister; 2] = [Sister::ClaudeCode, Sister::Antigravity];

    /// Telemetry's name for it
    pub fn id(self) -> &'static str {
        match self {
            Sister::ClaudeCode => "ccn",
            Sister::Antigravity => "antigravity",
        }
    }

    /// The app's own name, which every translation keeps
    #[cfg(test)]
    fn app_name(self) -> &'static str {
        match self {
            Sister::ClaudeCode => "Claude Code Notifier",
            Sister::Antigravity => "Antigravity Notifier",
        }
    }

    /// The one line that sets its CLI up on this computer
    pub fn command(self) -> &'static str {
        match (self, cfg!(windows)) {
            (Sister::ClaudeCode, false) => claude_code::UNIX_COMMAND,
            (Sister::ClaudeCode, true) => claude_code::WINDOWS_COMMAND,
            (Sister::Antigravity, false) => antigravity::UNIX_COMMAND,
            (Sister::Antigravity, true) => antigravity::WINDOWS_COMMAND,
        }
    }

    /// The website
    pub fn url(self) -> &'static str {
        match self {
            Sister::ClaudeCode => claude_code::URL,
            Sister::Antigravity => antigravity::URL,
        }
    }

    /// The QR code and link after `codex-notifier pair`
    pub fn pair_url(self) -> &'static str {
        match self {
            Sister::ClaudeCode => claude_code::PAIR_URL,
            Sister::Antigravity => antigravity::PAIR_URL,
        }
    }

    /// Where a click on the SessionStart notification goes
    pub fn notification_url(self) -> &'static str {
        match self {
            Sister::ClaudeCode => claude_code::NOTIFICATION_URL,
            Sister::Antigravity => antigravity::NOTIFICATION_URL,
        }
    }

    /// Its agent is on this computer
    pub fn agent_present(self, places: &Places) -> bool {
        match self {
            Sister::ClaudeCode => claude_present(places),
            Sister::Antigravity => antigravity_present(places),
        }
    }

    /// Its notifier is on this computer already
    pub fn notifier_present(self, places: &Places) -> bool {
        match self {
            Sister::ClaudeCode => ccn_present(places),
            Sister::Antigravity => antigravity_notifier_present(places),
        }
    }

    /// Someone with its agent who doesn't have its notifier yet
    pub fn worth_suggesting(self, places: &Places) -> bool {
        self.agent_present(places) && !self.notifier_present(places)
    }

    /// The line itself, the same everywhere it is shown
    pub fn line(self) -> String {
        let (command, url) = (self.command(), self.url());
        match self {
            Sister::ClaudeCode => t!("cross_promo.line", command = command, url = url),
            Sister::Antigravity => t!("cross_promo_antigravity.line", command = command, url = url),
        }
        .into_owned()
    }

    /// The SessionStart desktop notification's body (its title is this app's name)
    pub fn notification_body(self) -> String {
        match self {
            Sister::ClaudeCode => t!("cross_promo.notification"),
            Sister::Antigravity => t!("cross_promo_antigravity.notification"),
        }
        .into_owned()
    }

    /// Above the QR code after `pair`
    fn pair_title(self) -> String {
        match self {
            Sister::ClaudeCode => t!("cross_promo.pair_title"),
            Sister::Antigravity => t!("cross_promo_antigravity.pair_title"),
        }
        .into_owned()
    }

    fn pair_scan(self) -> String {
        match self {
            Sister::ClaudeCode => t!("cross_promo.pair_scan"),
            Sister::Antigravity => t!("cross_promo_antigravity.pair_scan"),
        }
        .into_owned()
    }

    fn pair_steps(self) -> String {
        match self {
            Sister::ClaudeCode => t!("cross_promo.pair_steps"),
            Sister::Antigravity => t!("cross_promo_antigravity.pair_steps"),
        }
        .into_owned()
    }
}

/// Where the sister agents and their notifiers would live on this computer.
#[derive(Debug, Clone)]
pub struct Places {
    /// Claude Code's config folder: `$CLAUDE_CONFIG_DIR`, else `~/.claude`
    pub claude_dir: PathBuf,
    /// ccn's own folder, found the way ccn finds it (and codex-notifier finds its own)
    pub ccn_dir: PathBuf,
    /// The folder Antigravity shares with the Gemini CLI, found the way antigravity-notifier finds
    /// it: `$ANTIGRAVITY_NOTIFIER_GEMINI_HOME`, else `~/.gemini`
    pub gemini_dir: PathBuf,
    /// antigravity-notifier's own folder, found the way it finds it
    pub antigravity_notifier_dir: PathBuf,
    /// `$PATH`
    pub path: Option<OsString>,
}

fn env_dir(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

impl Places {
    pub fn from_env() -> Places {
        let home = config::home_dir();
        Places {
            claude_dir: env_dir("CLAUDE_CONFIG_DIR").unwrap_or_else(|| home.join(".claude")),
            ccn_dir: tool_dir(&home, "CCN_HOME", "ccn"),
            gemini_dir: env_dir("ANTIGRAVITY_NOTIFIER_GEMINI_HOME")
                .unwrap_or_else(|| home.join(".gemini")),
            antigravity_notifier_dir: tool_dir(
                &home,
                "ANTIGRAVITY_NOTIFIER_HOME",
                "antigravity-notifier",
            ),
            path: std::env::var_os("PATH"),
        }
    }
}

/// A sister CLI's folder: `$<env>`, else `%APPDATA%\<name>` on Windows, else
/// `$XDG_CONFIG_HOME/<name>`, else `~/.config/<name>`. The same order as `config::app_dir`.
fn tool_dir(home: &Path, env: &str, name: &str) -> PathBuf {
    if let Some(dir) = env_dir(env) {
        return dir;
    }
    #[cfg(windows)]
    {
        if let Some(appdata) = env_dir("APPDATA") {
            return appdata.join(name);
        }
    }
    if let Some(xdg) = env_dir("XDG_CONFIG_HOME") {
        return xdg.join(name);
    }
    home.join(".config").join(name)
}

/// An executable called `name` in one of `path`'s folders.
fn on_path(name: &str, path: Option<&OsString>) -> bool {
    let Some(path) = path else {
        return false;
    };
    let names: Vec<String> = if cfg!(windows) {
        // npm puts a .cmd (and a .ps1) shim on Windows
        ["", ".exe", ".cmd", ".bat", ".ps1"]
            .iter()
            .map(|ext| format!("{name}{ext}"))
            .collect()
    } else {
        vec![name.to_string()]
    };
    std::env::split_paths(path).any(|dir| names.iter().any(|n| dir.join(n).is_file()))
}

/// A hooks file has a hook whose command runs one of `programs` (`ccn hook`, `/path/to/ccn hook`,
/// `npx @touchtap/ccn hook`, `C:\...\ccn.exe hook`). Claude Code keeps its hooks under `hooks`
/// in settings.json (`under`); Antigravity's hooks.json is named groups from the top.
fn hooks_run(file: &Path, under: Option<&str>, programs: &[&str]) -> bool {
    let Ok(text) = std::fs::read_to_string(file) else {
        return false;
    };
    let Ok(json) = serde_json::from_str::<Value>(&text) else {
        return false;
    };
    let mut commands = Vec::new();
    collect_commands(under.map_or(&json, |key| &json[key]), &mut commands);
    commands.iter().any(|command| {
        command.split_whitespace().any(|word| {
            let word = word.trim_matches(|c| c == '"' || c == '\'');
            let file = word.rsplit(['/', '\\']).next().unwrap_or(word);
            let file = file.strip_suffix(".exe").unwrap_or(file);
            programs.iter().any(|p| file.eq_ignore_ascii_case(p))
        })
    })
}

fn collect_commands<'a>(value: &'a Value, out: &mut Vec<&'a str>) {
    match value {
        Value::Object(map) => {
            for (key, v) in map {
                match (key.as_str(), v) {
                    ("command", Value::String(s)) => out.push(s),
                    _ => collect_commands(v, out),
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|v| collect_commands(v, out)),
        _ => {}
    }
}

/// Claude Code is on this computer: its config folder exists, or `claude` is on the PATH.
pub fn claude_present(places: &Places) -> bool {
    places.claude_dir.exists() || on_path("claude", places.path.as_ref())
}

/// ccn is already here: its folder exists, `ccn` is on the PATH, or Claude Code's hooks run it.
pub fn ccn_present(places: &Places) -> bool {
    places.ccn_dir.exists()
        || on_path("ccn", places.path.as_ref())
        || hooks_run(
            &places.claude_dir.join("settings.json"),
            Some("hooks"),
            &["ccn"],
        )
}

/// Antigravity is on this computer: `agy` is on the PATH, or `~/.gemini` holds what Antigravity
/// keeps there (`antigravity-cli`, or the `config` folder its hooks.json lives in). `~/.gemini`
/// alone is not enough: the Gemini CLI uses it too.
pub fn antigravity_present(places: &Places) -> bool {
    on_path("agy", places.path.as_ref())
        || places.gemini_dir.join("antigravity-cli").is_dir()
        || places.gemini_dir.join("config").is_dir()
}

/// antigravity-notifier is already here: its folder exists, it is on the PATH, or Antigravity's
/// hooks run it.
pub fn antigravity_notifier_present(places: &Places) -> bool {
    const NAME: &str = "antigravity-notifier";
    places.antigravity_notifier_dir.exists()
        || on_path(NAME, places.path.as_ref())
        || hooks_run(
            &places.gemini_dir.join("config").join("hooks.json"),
            None,
            &[NAME],
        )
}

/// The sister app to suggest here, if any: Claude Code Notifier first when both qualify, so
/// never more than one at a time.
pub fn worth_suggesting(places: &Places) -> Option<Sister> {
    Sister::ALL.into_iter().find(|s| s.worth_suggesting(places))
}

/// The setting is on and `CODEX_NOTIFIER_NO_NUDGE` isn't set.
pub fn enabled(cfg: &Config) -> bool {
    cfg.cross_promo && !crate::local::no_nudge_env()
}

/// What to suggest anywhere other than SessionStart: enabled, and worth suggesting here.
pub fn eligible(cfg: &Config) -> Option<Sister> {
    if enabled(cfg) {
        worth_suggesting(&Places::from_env())
    } else {
        None
    }
}

/// Counts a showing; `place` is pair, session, status or doctor.
fn shown(sister: Sister, place: &str) {
    crate::telemetry::event_detached(
        "cli_cross_promo_shown",
        &[("where", place), ("app", sister.id())],
    );
}

/// Prints the line (after a blank one) when eligible. For `status` and `doctor`.
pub fn print_hint(cfg: &Config, place: &str) {
    if let Some(sister) = eligible(cfg) {
        println!();
        println!("{}", sister.line());
        shown(sister, place);
    }
}

/// After every successful `codex-notifier pair`, when eligible: the phone is in their hand, so a
/// QR code (for the app, or for Antigravity Notifier's website), its link, and the one command
/// for this computer.
pub fn print_after_pair(cfg: &Config) {
    let Some(sister) = eligible(cfg) else {
        return;
    };
    let code = qr::render(sister.pair_url());
    println!();
    println!("{}", sister.pair_title());
    match &code {
        Some(code) => {
            println!("{}", sister.pair_scan());
            println!();
            print!("{code}");
        }
        None => println!("{}", sister.pair_steps()),
    }
    println!("  {}", sister.pair_url());
    println!("  {}", sister.command());
    let qr = if code.is_some() { "yes" } else { "no" };
    crate::telemetry::event_detached(
        "cli_cross_promo_shown",
        &[("where", "pair"), ("qr", qr), ("app", sister.id())],
    );
}

/// Whether SessionStart may show the line: the first time no earlier than 3 days after `since`
/// (pairing), the second 30 days after the first, then never.
pub fn session_due(now: i64, since: i64, last_at: Option<i64>, count: u32) -> bool {
    match count {
        0 => now - since >= FIRST_AFTER,
        c if c < SESSION_MAX => last_at.is_none_or(|last| now - last >= SECOND_AFTER),
        _ => false,
    }
}

/// The `systemMessage` for a paired SessionStart that shows nothing else, when due and eligible,
/// with a desktop notification as well (most people never read Codex's start-up lines) unless
/// desktop notifications are off. Neither goes through `codex_wording`.
pub fn session_message(cfg: &mut Config, now: i64) -> Option<Value> {
    if !enabled(cfg) {
        return None;
    }
    let first_seen = cfg.cross_promo_first_seen_at;
    let found = session_line(cfg, now, &Places::from_env());
    if found.is_some() || cfg.cross_promo_first_seen_at != first_seen {
        let _ = cfg.save();
    }
    let (sister, text) = found?;
    if cfg.desktop_notifications {
        crate::notify::show_link(
            crate::notify::APP_NAME,
            &sister.notification_body(),
            sister.notification_url(),
        );
    }
    crate::logger::log(&format!(
        "{} suggestion shown at SessionStart (desktop notification: {})",
        sister.id(),
        cfg.desktop_notifications
    ));
    shown(sister, "session");
    Some(json!({ "systemMessage": text }))
}

/// The cadence and detection behind `session_message`, without saving: the app and its line when
/// one is due here, with the showing counted in `cfg`.
pub fn session_line(cfg: &mut Config, now: i64, places: &Places) -> Option<(Sister, String)> {
    // Never the same day as the rating request
    if cfg.cross_promo_count >= SESSION_MAX
        || cfg.rate_prompt_shown_at.is_some_and(|at| now - at < DAY)
    {
        return None;
    }
    let since = cfg
        .paired_at
        .or(cfg.registered_at)
        .unwrap_or_else(|| *cfg.cross_promo_first_seen_at.get_or_insert(now));
    if !session_due(now, since, cfg.cross_promo_last_at, cfg.cross_promo_count) {
        return None;
    }
    let sister = worth_suggesting(places)?;
    cfg.cross_promo_count += 1;
    cfg.cross_promo_last_at = Some(now);
    Some((sister, sister.line()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "codex-notifier-crosspromo-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn places(root: &Path) -> Places {
        Places {
            claude_dir: root.join(".claude"),
            ccn_dir: root.join(".config").join("ccn"),
            gemini_dir: root.join(".gemini"),
            antigravity_notifier_dir: root.join(".config").join("antigravity-notifier"),
            path: Some(std::env::join_paths([root.join("bin")]).unwrap()),
        }
    }

    fn exe(root: &Path, name: &str) {
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let file = if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_string()
        };
        fs::write(bin.join(file), "").unwrap();
    }

    #[test]
    fn nothing_installed_suggests_nothing() {
        let root = temp("none");
        let p = places(&root);
        assert!(!claude_present(&p));
        assert!(!ccn_present(&p));
        assert!(!antigravity_present(&p));
        assert!(!antigravity_notifier_present(&p));
        assert_eq!(worth_suggesting(&p), None);
        let none = Places {
            path: None,
            ..p.clone()
        };
        assert_eq!(worth_suggesting(&none), None);
    }

    #[test]
    fn claude_code_without_ccn_is_worth_suggesting() {
        let root = temp("claude-dir");
        fs::create_dir_all(root.join(".claude")).unwrap();
        assert_eq!(worth_suggesting(&places(&root)), Some(Sister::ClaudeCode));

        let root = temp("claude-bin");
        exe(&root, "claude");
        assert!(claude_present(&places(&root)));
        assert_eq!(worth_suggesting(&places(&root)), Some(Sister::ClaudeCode));
    }

    #[test]
    fn ccn_already_there_suggests_nothing() {
        // ccn's folder
        let root = temp("ccn-dir");
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::create_dir_all(root.join(".config").join("ccn")).unwrap();
        assert!(ccn_present(&places(&root)));
        assert_eq!(worth_suggesting(&places(&root)), None);

        // ccn on the PATH
        let root = temp("ccn-bin");
        fs::create_dir_all(root.join(".claude")).unwrap();
        exe(&root, "ccn");
        assert_eq!(worth_suggesting(&places(&root)), None);

        // Claude Code's hooks run ccn
        for command in [
            "ccn hook",
            "/usr/local/bin/ccn hook",
            "\"C:\\Users\\me\\AppData\\Local\\ccn\\ccn.exe\" hook",
            "npx -y @touchtap/ccn hook",
        ] {
            let root = temp("ccn-hooks");
            fs::create_dir_all(root.join(".claude")).unwrap();
            let settings = json!({ "hooks": { "Stop": [ { "hooks": [
                { "type": "command", "command": command }
            ] } ] } });
            fs::write(
                root.join(".claude").join("settings.json"),
                settings.to_string(),
            )
            .unwrap();
            assert_eq!(worth_suggesting(&places(&root)), None, "{command}");
        }
    }

    #[test]
    fn other_hooks_and_bad_settings_do_not_count_as_ccn() {
        let root = temp("other-hooks");
        fs::create_dir_all(root.join(".claude")).unwrap();
        let settings = json!({
            "env": { "command": "ccn" },
            "hooks": { "Stop": [ { "hooks": [
                { "type": "command", "command": "/opt/ccnotes/bin/ccnx --hook" }
            ] } ] }
        });
        let file = root.join(".claude").join("settings.json");
        fs::write(&file, settings.to_string()).unwrap();
        assert_eq!(worth_suggesting(&places(&root)), Some(Sister::ClaudeCode));
        fs::write(&file, "{ not json").unwrap();
        assert_eq!(worth_suggesting(&places(&root)), Some(Sister::ClaudeCode));
    }

    #[test]
    fn antigravity_without_its_notifier_is_worth_suggesting() {
        // `agy` on the PATH
        let root = temp("agy-bin");
        exe(&root, "agy");
        assert!(antigravity_present(&places(&root)));
        assert_eq!(worth_suggesting(&places(&root)), Some(Sister::Antigravity));

        // What Antigravity keeps in ~/.gemini
        for kept in ["antigravity-cli", "config"] {
            let root = temp("agy-dir");
            fs::create_dir_all(root.join(".gemini").join(kept)).unwrap();
            assert_eq!(
                worth_suggesting(&places(&root)),
                Some(Sister::Antigravity),
                "{kept}"
            );
        }
    }

    #[test]
    fn the_gemini_cli_alone_is_not_antigravity() {
        // ~/.gemini is the Gemini CLI's folder too: its own files say nothing about Antigravity
        let root = temp("gemini-cli");
        fs::create_dir_all(root.join(".gemini").join("tmp")).unwrap();
        fs::write(root.join(".gemini").join("settings.json"), "{}").unwrap();
        exe(&root, "gemini");
        assert!(!antigravity_present(&places(&root)));
        assert_eq!(worth_suggesting(&places(&root)), None);
    }

    #[test]
    fn antigravity_notifier_already_there_suggests_nothing() {
        // Its folder
        let root = temp("agn-dir");
        exe(&root, "agy");
        fs::create_dir_all(root.join(".config").join("antigravity-notifier")).unwrap();
        assert!(antigravity_notifier_present(&places(&root)));
        assert_eq!(worth_suggesting(&places(&root)), None);

        // On the PATH
        let root = temp("agn-bin");
        exe(&root, "agy");
        exe(&root, "antigravity-notifier");
        assert_eq!(worth_suggesting(&places(&root)), None);

        // Antigravity's hooks run it: named groups from the top of hooks.json, flat lists and
        // matcher lists
        for command in [
            "antigravity-notifier hook Stop",
            "/opt/homebrew/bin/antigravity-notifier hook Stop",
            "\"C:\\Users\\me\\scoop\\apps\\antigravity-notifier\\current\\antigravity-notifier.exe\" hook Stop",
        ] {
            let root = temp("agn-hooks");
            let config = root.join(".gemini").join("config");
            fs::create_dir_all(&config).unwrap();
            let hooks = json!({ "antigravity-notifier": {
                "Stop": [ { "type": "command", "command": command, "timeout": 700 } ],
                "PreToolUse": [ { "matcher": "*", "hooks": [
                    { "type": "command", "command": command }
                ] } ]
            } });
            fs::write(config.join("hooks.json"), hooks.to_string()).unwrap();
            assert_eq!(worth_suggesting(&places(&root)), None, "{command}");
        }

        // Someone else's hooks there don't count
        let root = temp("agn-other-hooks");
        let config = root.join(".gemini").join("config");
        fs::create_dir_all(&config).unwrap();
        let hooks =
            json!({ "policy": { "Stop": [ { "type": "command", "command": "/x/policy.sh" } ] } });
        fs::write(config.join("hooks.json"), hooks.to_string()).unwrap();
        assert_eq!(worth_suggesting(&places(&root)), Some(Sister::Antigravity));
    }

    #[test]
    fn claude_code_comes_first_then_antigravity_and_never_both() {
        // Both agents, neither notifier: Claude Code Notifier
        let root = temp("both");
        fs::create_dir_all(root.join(".claude")).unwrap();
        exe(&root, "agy");
        assert_eq!(worth_suggesting(&places(&root)), Some(Sister::ClaudeCode));
        // ccn already there: Antigravity Notifier instead
        fs::create_dir_all(root.join(".config").join("ccn")).unwrap();
        assert_eq!(worth_suggesting(&places(&root)), Some(Sister::Antigravity));

        // The SessionStart cap is shared: two showings in all, whichever app they named
        let p = places(&root);
        let paired = 1_700_000_000;
        let mut cfg = Config::default();
        cfg.paired_at = Some(paired);
        let (sister, line) = session_line(&mut cfg, paired + 3 * DAY, &p).unwrap();
        assert_eq!(sister, Sister::Antigravity);
        assert_eq!(line, Sister::Antigravity.line());
        assert!(line.contains("Antigravity Notifier") && line.contains("antigravitynotifier.com"));
        assert!(!line.contains("Claude"), "{line}");
        assert!(session_line(&mut cfg, paired + 33 * DAY, &p).is_some());
        assert_eq!(session_line(&mut cfg, paired + 400 * DAY, &p), None);
    }

    #[test]
    fn session_shows_it_after_three_days_then_thirty_then_never() {
        let paired = 1_700_000_000;
        let at = |days: i64| paired + days * DAY;
        assert!(!session_due(at(0), paired, None, 0));
        assert!(!session_due(at(2), paired, None, 0));
        assert!(session_due(at(3), paired, None, 0));
        assert!(session_due(at(40), paired, None, 0));
        // Second: 30 days after the first, not after pairing
        assert!(!session_due(at(32), paired, Some(at(3)), 1));
        assert!(session_due(at(33), paired, Some(at(3)), 1));
        // Never a third
        assert!(!session_due(at(400), paired, Some(at(33)), 2));
        assert!(!session_due(at(400), paired, Some(at(33)), 9));
    }

    #[test]
    fn session_counts_showings_and_stops_at_two() {
        let root = temp("session");
        fs::create_dir_all(root.join(".claude")).unwrap();
        let p = places(&root);
        let paired = 1_700_000_000;
        let shown = Some((Sister::ClaudeCode, Sister::ClaudeCode.line()));
        let mut cfg = Config::default();
        cfg.paired_at = Some(paired);
        assert_eq!(session_line(&mut cfg, paired + DAY, &p), None);
        assert_eq!(session_line(&mut cfg, paired + 3 * DAY, &p), shown);
        assert_eq!(cfg.cross_promo_count, 1);
        assert_eq!(cfg.cross_promo_last_at, Some(paired + 3 * DAY));
        assert_eq!(session_line(&mut cfg, paired + 4 * DAY, &p), None);
        assert_eq!(session_line(&mut cfg, paired + 33 * DAY, &p), shown);
        assert_eq!(session_line(&mut cfg, paired + 400 * DAY, &p), None);
        assert_eq!(cfg.cross_promo_count, 2);
        assert_eq!(cfg.cross_promo_first_seen_at, None);

        // No pairing time: counted from the first SessionStart that looks
        let mut cfg = Config::default();
        assert_eq!(session_line(&mut cfg, paired, &p), None);
        assert_eq!(cfg.cross_promo_first_seen_at, Some(paired));
        assert_eq!(session_line(&mut cfg, paired + 2 * DAY, &p), None);
        assert_eq!(session_line(&mut cfg, paired + 3 * DAY, &p), shown);

        // Not worth suggesting: nothing shown, nothing counted
        let bare = temp("session-bare");
        let mut cfg = Config::default();
        cfg.paired_at = Some(paired);
        assert_eq!(
            session_line(&mut cfg, paired + 9 * DAY, &places(&bare)),
            None
        );
        assert_eq!(cfg.cross_promo_count, 0);
    }

    #[test]
    fn the_line_keeps_its_command_url_and_claude_code() {
        let s = Sister::ClaudeCode;
        assert_eq!(
            s.line(),
            format!(
                "Claude Code detected — get Claude Code approvals and questions on your phone too, with the Claude Code Notifier app (iPhone and Android). Set up this computer with one command: {}  ·  https://claudecodenotifier.com/?ref=codex-cli",
                s.command()
            )
        );
        assert!(s.command().contains("claudecodenotifier.com/install."));
        assert!(!s.command().contains("npm"));
        assert_eq!(
            s.pair_url(),
            "https://claudecodenotifier.com/get?ref=codex-pair"
        );
        assert!(qr::render(s.pair_url()).is_some());
    }

    /// Antigravity Notifier's phone apps were in store review when this shipped: everything the
    /// suggestion links to is the website, which no store link or /get redirect can stand in for.
    #[test]
    fn the_antigravity_line_points_at_the_website_and_never_at_a_store() {
        let s = Sister::Antigravity;
        assert_eq!(
            s.line(),
            format!(
                "Antigravity detected — get Antigravity approvals and questions on your phone too, with the Antigravity Notifier app (iPhone and Android). Set up this computer with one command: {}  ·  https://antigravitynotifier.com/?ref=codex-cli",
                s.command()
            )
        );
        assert!(s.command().contains("antigravitynotifier.com/install."));
        assert!(!s.command().contains("npm"));
        assert_eq!(s.url(), "https://antigravitynotifier.com/?ref=codex-cli");
        assert_eq!(
            s.pair_url(),
            "https://antigravitynotifier.com/?ref=codex-pair"
        );
        assert_eq!(
            s.notification_url(),
            "https://antigravitynotifier.com/?ref=codex-notification"
        );
        for url in [s.url(), s.pair_url(), s.notification_url()] {
            assert!(
                url.starts_with("https://antigravitynotifier.com/?ref="),
                "{url}"
            );
            for store in ["apple.com", "google.com", "/get", "/rate"] {
                assert!(!url.contains(store), "{url}");
            }
        }
        assert!(qr::render(s.pair_url()).is_some());
    }

    /// line, notification, pair_title, pair_scan, pair_steps in `lang`
    fn texts(sister: Sister, lang: &str) -> [String; 5] {
        let (command, url) = (sister.command(), sister.url());
        let all = match sister {
            Sister::ClaudeCode => [
                rust_i18n::t!(
                    "cross_promo.line",
                    locale = lang,
                    command = command,
                    url = url
                ),
                rust_i18n::t!("cross_promo.notification", locale = lang),
                rust_i18n::t!("cross_promo.pair_title", locale = lang),
                rust_i18n::t!("cross_promo.pair_scan", locale = lang),
                rust_i18n::t!("cross_promo.pair_steps", locale = lang),
            ],
            Sister::Antigravity => [
                rust_i18n::t!(
                    "cross_promo_antigravity.line",
                    locale = lang,
                    command = command,
                    url = url
                ),
                rust_i18n::t!("cross_promo_antigravity.notification", locale = lang),
                rust_i18n::t!("cross_promo_antigravity.pair_title", locale = lang),
                rust_i18n::t!("cross_promo_antigravity.pair_scan", locale = lang),
                rust_i18n::t!("cross_promo_antigravity.pair_steps", locale = lang),
            ],
        };
        all.map(|t| t.into_owned())
    }

    #[test]
    fn every_translation_names_the_app_and_no_one_platform() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
        for sister in Sister::ALL {
            // The other sister's name never appears, nor this app's own agent
            let other = match sister {
                Sister::ClaudeCode => "Antigravity",
                Sister::Antigravity => "Claude",
            };
            for lang in config::LOCALES {
                let texts = texts(sister, lang);
                let line = &texts[0];
                assert!(
                    line.contains(sister.command()) && line.contains(sister.url()),
                    "{lang}: {line}"
                );
                for text in &texts[..3] {
                    assert!(text.contains(sister.app_name()), "{lang}: {text}");
                }
                for text in &texts {
                    for word in ["Mac", "macOS", "Codex", other] {
                        assert!(!text.contains(word), "{lang}: {text}");
                    }
                    // A missing translation comes back as its key
                    assert!(!text.contains("cross_promo"), "{lang}: {text}");
                }
                assert!(dir.join(format!("{lang}.yml")).exists());
            }
        }
    }
}
