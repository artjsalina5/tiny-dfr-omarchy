#![allow(dead_code)]

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HyprlandWorkspace {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HyprlandWindow {
    pub address: String,
    pub mapped: bool,
    pub hidden: bool,
    pub at: [i32; 2],
    pub size: [i32; 2],
    #[serde(rename = "workspace")]
    pub workspace: HyprlandWorkspace,
    pub floating: bool,
    #[serde(default)]
    pub pseudo: bool,
    pub monitor: i32,
    #[serde(default)]
    pub content_type: String,
    #[serde(default)]
    pub over_fullscreen: bool,
    #[serde(default)]
    pub stable_id: String,
    #[serde(rename = "class")]
    pub class: String,
    pub title: String,
    #[serde(rename = "initialClass")]
    pub initial_class: String,
    #[serde(rename = "initialTitle")]
    pub initial_title: String,
    pub pid: i32,
    pub xwayland: bool,
    pub pinned: bool,
    pub fullscreen: i32,
    #[serde(rename = "fullscreenClient")]
    pub fullscreen_client: i32,
    pub grouped: Vec<String>,
    pub tags: Vec<String>,
    pub swallowing: String,
    #[serde(rename = "focusHistoryID")]
    pub focus_history_id: i32,
    #[serde(rename = "inhibitingIdle")]
    pub inhibiting_idle: bool,
    #[serde(rename = "xdgTag")]
    pub xdg_tag: String,
    #[serde(rename = "xdgDescription")]
    pub xdg_description: String,
}

#[derive(Debug, Clone, Default)]
pub struct ActiveWindowInfo {
    pub address: String,
    pub title: String,
    pub class: String,
    pub initial_title: String,
    pub initial_class: String,
}

impl ActiveWindowInfo {
    pub fn from_hyprland_window(window: HyprlandWindow) -> Self {
        Self {
            address: window.address,
            title: window.title,
            class: window.class,
            initial_title: window.initial_title,
            initial_class: window.initial_class,
        }
    }

    pub fn field(&self, key: &str) -> &str {
        match key {
            "title" => &self.title,
            "class" => &self.class,
            "initialTitle" => &self.initial_title,
            "initialClass" => &self.initial_class,
            "address" => &self.address,
            _ => &self.title,
        }
    }

    // Backward-compatible API used by main.rs
    pub fn get_text_by_button_title(&self, button_title: &str) -> String {
        self.field(button_title).to_string()
    }

    // Use this where the display area is byte-limited.
    pub fn get_text_by_button_title_limited(&self, button_title: &str, max_bytes: usize) -> String {
        ellipsize_utf8_bytes(self.field(button_title), max_bytes)
    }

    pub fn field_ellipsized_chars(&self, key: &str, max_chars: usize) -> String {
        ellipsize_chars(self.field(key), max_chars)
    }

    pub fn field_ellipsized_bytes(&self, key: &str, max_bytes: usize) -> String {
        ellipsize_utf8_bytes(self.field(key), max_bytes)
    }

    pub fn app_icon_name(&self) -> String {
        format!("app-{}", self.class)
    }

    // Backward-compatible API used by main.rs
    pub fn get_app_icon_name(&self) -> String {
        self.app_icon_name()
    }
}

#[derive(Debug, Clone)]
pub struct HyprlandSockets {
    pub command: PathBuf,
    pub events: PathBuf,
}

impl HyprlandSockets {
    pub fn discover() -> Result<Self> {
        if let Some(sockets) = Self::from_env() {
            return Ok(sockets);
        }

        if let Some(sockets) = Self::scan_xdg_runtime_dir()? {
            return Ok(sockets);
        }

        if let Some(sockets) = Self::scan_tmp_hypr()? {
            return Ok(sockets);
        }

        if let Some(sockets) = Self::scan_run_user_hypr()? {
            return Ok(sockets);
        }

        Err(anyhow!("could not locate Hyprland IPC sockets"))
    }

    fn from_env() -> Option<Self> {
        let runtime = env::var_os("XDG_RUNTIME_DIR")?;
        let his = env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;

        let base = PathBuf::from(runtime).join("hypr").join(his);
        let command = base.join(".socket.sock");
        let events = base.join(".socket2.sock");

        if command.exists() && events.exists() {
            Some(Self { command, events })
        } else {
            None
        }
    }

    fn scan_xdg_runtime_dir() -> Result<Option<Self>> {
        let runtime = match env::var_os("XDG_RUNTIME_DIR") {
            Some(v) => PathBuf::from(v),
            None => return Ok(None),
        };

        let hypr_dir = runtime.join("hypr");
        if !hypr_dir.exists() {
            return Ok(None);
        }

        for entry in std::fs::read_dir(&hypr_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let command = path.join(".socket.sock");
            let events = path.join(".socket2.sock");

            if command.exists() && events.exists() {
                return Ok(Some(Self { command, events }));
            }
        }

        Ok(None)
    }

    fn scan_tmp_hypr() -> Result<Option<Self>> {
        let hypr_dir = PathBuf::from("/tmp/hypr");
        if !hypr_dir.exists() {
            return Ok(None);
        }

        for entry in std::fs::read_dir(&hypr_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let command = path.join(".socket.sock");
            let events = path.join(".socket2.sock");

            if command.exists() && events.exists() {
                return Ok(Some(Self { command, events }));
            }
        }

        Ok(None)
    }

    fn scan_run_user_hypr() -> Result<Option<Self>> {
        let run_user = PathBuf::from("/run/user");
        if !run_user.exists() {
            return Ok(None);
        }

        for user_entry in std::fs::read_dir(&run_user)? {
            let user_entry = user_entry?;
            let user_path = user_entry.path();
            if !user_path.is_dir() {
                continue;
            }

            let hypr_dir = user_path.join("hypr");
            if !hypr_dir.exists() {
                continue;
            }

            for entry in std::fs::read_dir(&hypr_dir)? {
                let entry = entry?;
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                let command = path.join(".socket.sock");
                let events = path.join(".socket2.sock");

                if command.exists() && events.exists() {
                    return Ok(Some(Self { command, events }));
                }
            }
        }

        Ok(None)
    }
}

#[derive(Debug, Clone)]
pub struct HyprlandIpc {
    sockets: HyprlandSockets,
}

impl HyprlandIpc {
    pub fn new() -> Result<Self> {
        Ok(Self {
            sockets: HyprlandSockets::discover()?,
        })
    }

    pub fn send_command(&self, command: &str) -> Result<String> {
        let mut stream = UnixStream::connect(&self.sockets.command).with_context(|| {
            format!(
                "failed to connect to Hyprland command socket {}",
                self.sockets.command.display()
            )
        })?;

        stream
            .write_all(command.as_bytes())
            .context("failed to write command to Hyprland socket")?;

        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .context("failed to read Hyprland response")?;

        Ok(response)
    }

    pub fn get_active_window(&self) -> Result<HyprlandWindow> {
        let response = self.send_command("j/activewindow")?;
        serde_json::from_str(&response).context("failed to parse j/activewindow response")
    }

    pub fn get_clients(&self) -> Result<Vec<HyprlandWindow>> {
        let response = self.send_command("j/clients")?;
        serde_json::from_str(&response).context("failed to parse j/clients response")
    }

    pub fn start_event_listener(&self) {
        let sockets = self.sockets.clone();
        thread::spawn(move || {
            event_listener_loop(sockets);
        });
    }
}

#[derive(Debug, Clone)]
enum HyprEvent {
    ActiveWindowV2 { address: String },
    WindowTitleV2 { address: String, title: String },
    ActiveWindow { class: String, title: String },
    Unknown,
}

fn parse_event_line(line: &str) -> HyprEvent {
    if let Some(data) = line.strip_prefix("activewindowv2>>") {
        return HyprEvent::ActiveWindowV2 {
            address: data.to_string(),
        };
    }

    if let Some(data) = line.strip_prefix("windowtitlev2>>") {
        let mut parts = data.splitn(2, ',');
        let address = parts.next().unwrap_or_default().to_string();
        let title = parts.next().unwrap_or_default().to_string();
        return HyprEvent::WindowTitleV2 { address, title };
    }

    if let Some(data) = line.strip_prefix("activewindow>>") {
        let mut parts = data.splitn(2, ',');
        let class = parts.next().unwrap_or_default().to_string();
        let title = parts.next().unwrap_or_default().to_string();
        return HyprEvent::ActiveWindow { class, title };
    }

    HyprEvent::Unknown
}

#[derive(Debug, Default)]
struct SharedState {
    active: Option<ActiveWindowInfo>,
    cache_updated: bool,
    listener_started: bool,
    last_refresh: Option<Instant>,
}

static SHARED_STATE: LazyLock<Arc<Mutex<SharedState>>> =
    LazyLock::new(|| Arc::new(Mutex::new(SharedState::default())));

#[derive(Debug)]
struct RateLimitState {
    last_failure_time: Option<Instant>,
    current_backoff: Duration,
}

impl Default for RateLimitState {
    fn default() -> Self {
        Self {
            last_failure_time: None,
            current_backoff: Duration::from_secs(2),
        }
    }
}

static RATE_LIMIT_STATE: LazyLock<Arc<Mutex<RateLimitState>>> =
    LazyLock::new(|| Arc::new(Mutex::new(RateLimitState::default())));

fn event_listener_loop(initial_sockets: HyprlandSockets) {
    let mut current_sockets = initial_sockets;

    loop {
        let stream = match UnixStream::connect(&current_sockets.events) {
            Ok(stream) => stream,
            Err(_) => {
                thread::sleep(Duration::from_secs(2));

                if let Ok(new_sockets) = HyprlandSockets::discover() {
                    current_sockets = new_sockets;
                }

                continue;
            }
        };

        let reader = BufReader::new(stream);

        for line in reader.lines() {
            let line = match line {
                Ok(v) => v,
                Err(_) => break,
            };

            handle_event_line(&current_sockets, &line);
        }

        thread::sleep(Duration::from_millis(250));

        if let Ok(new_sockets) = HyprlandSockets::discover() {
            current_sockets = new_sockets;
        }
    }
}

fn handle_event_line(sockets: &HyprlandSockets, line: &str) {
    match parse_event_line(line) {
        HyprEvent::ActiveWindowV2 { address } => {
            let _ = address;
            let _ = refresh_active_window_from_ipc(sockets);
        }
        HyprEvent::WindowTitleV2 { address, title } => {
            if let Ok(mut state) = SHARED_STATE.lock() {
                if let Some(active) = state.active.as_mut() {
                    if active.address == address {
                        active.title = title;
                        state.cache_updated = true;
                    }
                }
            }
        }
        HyprEvent::ActiveWindow { class, title } => {
            if let Ok(mut state) = SHARED_STATE.lock() {
                if let Some(active) = state.active.as_mut() {
                    active.class = class;
                    active.title = title;
                    state.cache_updated = true;
                }
            }
        }
        HyprEvent::Unknown => {}
    }
}

fn refresh_active_window_from_ipc(sockets: &HyprlandSockets) -> Result<()> {
    let ipc = HyprlandIpc {
        sockets: sockets.clone(),
    };

    let window = ipc.get_active_window()?;
    let info = ActiveWindowInfo::from_hyprland_window(window);

    if let Ok(mut state) = SHARED_STATE.lock() {
        state.active = Some(info);
        state.cache_updated = true;
        state.last_refresh = Some(Instant::now());
    }

    Ok(())
}

pub fn get_active_window_info() -> Result<ActiveWindowInfo> {
    // Check rate limiting before attempting IPC connection
    {
        let rate_limit = RATE_LIMIT_STATE
            .lock()
            .map_err(|_| anyhow!("rate limit state poisoned"))?;

        if let Some(last_failure) = rate_limit.last_failure_time {
            let elapsed = last_failure.elapsed();
            if elapsed < rate_limit.current_backoff {
                return Err(anyhow!(
                    "Hyprland not available (rate limited, retry in {}s)",
                    (rate_limit.current_backoff - elapsed).as_secs()
                ));
            }
        }
    }

    let ipc = match HyprlandIpc::new() {
        Ok(ipc) => {
            // Success: reset backoff state
            if let Ok(mut rate_limit) = RATE_LIMIT_STATE.lock() {
                rate_limit.last_failure_time = None;
                rate_limit.current_backoff = Duration::from_secs(2);
            }
            ipc
        }
        Err(e) => {
            // Failure: update backoff state with exponential increase
            if let Ok(mut rate_limit) = RATE_LIMIT_STATE.lock() {
                rate_limit.last_failure_time = Some(Instant::now());
                // Exponential backoff: 2s → 4s → 8s → 16s → 30s (capped)
                let new_backoff = rate_limit.current_backoff * 2;
                rate_limit.current_backoff = new_backoff.min(Duration::from_secs(30));
            }
            return Err(e);
        }
    };

    {
        let mut state = SHARED_STATE
            .lock()
            .map_err(|_| anyhow!("shared state poisoned"))?;

        if !state.listener_started {
            ipc.start_event_listener();
            state.listener_started = true;
        }

        if let Some(active) = state.active.clone() {
            return Ok(active);
        }
    }

    let window = ipc.get_active_window()?;
    let info = ActiveWindowInfo::from_hyprland_window(window);

    if let Ok(mut state) = SHARED_STATE.lock() {
        state.active = Some(info.clone());
        state.cache_updated = true;
        state.last_refresh = Some(Instant::now());
    }

    Ok(info)
}

pub fn check_and_reset_cache_updated() -> bool {
    if let Ok(mut state) = SHARED_STATE.lock() {
        let updated = state.cache_updated;
        state.cache_updated = false;
        updated
    } else {
        false
    }
}

pub fn ellipsize_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }

    if max_chars == 0 {
        return String::new();
    }

    if max_chars == 1 {
        return "…".to_string();
    }

    let head: String = s.chars().take(max_chars - 1).collect();
    format!("{head}…")
}

pub fn truncate_utf8_bytes(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }

    let mut end = 0;
    for (idx, _) in s.char_indices() {
        if idx <= max_bytes {
            end = idx;
        } else {
            break;
        }
    }

    s[..end].to_string()
}

pub fn ellipsize_utf8_bytes(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }

    if max_bytes == 0 {
        return String::new();
    }

    let ellipsis = "…";
    let reserve = ellipsis.len();

    if max_bytes <= reserve {
        return truncate_utf8_bytes(ellipsis, max_bytes);
    }

    let head = truncate_utf8_bytes(s, max_bytes - reserve);
    format!("{head}{ellipsis}")
}

fn normalized_key_name(name: &str) -> String {
    name.trim()
        .replace('-', "_")
        .replace('+', "_")
        .replace(' ', "_")
        .to_uppercase()
}

fn parse_key_name(name: &str) -> Option<input_linux::Key> {
    use input_linux::Key;

    let n = normalized_key_name(name);

    let key = match n.as_str() {
        // modifiers
        "CTRL" | "CONTROL" | "LCTRL" | "LEFTCTRL" | "LEFTCONTROL" => Key::LeftCtrl,
        "RCTRL" | "RIGHTCTRL" | "RIGHTCONTROL" => Key::RightCtrl,
        "SHIFT" | "LSHIFT" | "LEFTSHIFT" => Key::LeftShift,
        "RSHIFT" | "RIGHTSHIFT" => Key::RightShift,
        "ALT" | "LALT" | "LEFTALT" => Key::LeftAlt,
        "RALT" | "RIGHTALT" | "ALTGR" => Key::RightAlt,
        "META" | "CMD" | "SUPER" | "WIN" | "LEFTMETA" | "LMETA" | "LEFTSUPER" => Key::LeftMeta,
        "RMETA" | "RIGHTMETA" | "RIGHTSUPER" | "RSUPER" | "RCMD" => Key::RightMeta,

        // letters
        "A" => Key::A,
        "B" => Key::B,
        "C" => Key::C,
        "D" => Key::D,
        "E" => Key::E,
        "F" => Key::F,
        "G" => Key::G,
        "H" => Key::H,
        "I" => Key::I,
        "J" => Key::J,
        "K" => Key::K,
        "L" => Key::L,
        "M" => Key::M,
        "N" => Key::N,
        "O" => Key::O,
        "P" => Key::P,
        "Q" => Key::Q,
        "R" => Key::R,
        "S" => Key::S,
        "T" => Key::T,
        "U" => Key::U,
        "V" => Key::V,
        "W" => Key::W,
        "X" => Key::X,
        "Y" => Key::Y,
        "Z" => Key::Z,

        // numbers
        "0" | "NUM0" => Key::Num0,
        "1" | "NUM1" => Key::Num1,
        "2" | "NUM2" => Key::Num2,
        "3" | "NUM3" => Key::Num3,
        "4" | "NUM4" => Key::Num4,
        "5" | "NUM5" => Key::Num5,
        "6" | "NUM6" => Key::Num6,
        "7" | "NUM7" => Key::Num7,
        "8" | "NUM8" => Key::Num8,
        "9" | "NUM9" => Key::Num9,

        // function keys
        "F1" => Key::F1,
        "F2" => Key::F2,
        "F3" => Key::F3,
        "F4" => Key::F4,
        "F5" => Key::F5,
        "F6" => Key::F6,
        "F7" => Key::F7,
        "F8" => Key::F8,
        "F9" => Key::F9,
        "F10" => Key::F10,
        "F11" => Key::F11,
        "F12" => Key::F12,
        "F13" => Key::F13,
        "F14" => Key::F14,
        "F15" => Key::F15,
        "F16" => Key::F16,
        "F17" => Key::F17,
        "F18" => Key::F18,
        "F19" => Key::F19,
        "F20" => Key::F20,
        "F21" => Key::F21,
        "F22" => Key::F22,
        "F23" => Key::F23,
        "F24" => Key::F24,

        // navigation/editing
        "ENTER" | "RETURN" => Key::Enter,
        "ESC" | "ESCAPE" => Key::Esc,
        "TAB" => Key::Tab,
        "SPACE" | "SPACEBAR" => Key::Space,
        "BACKSPACE" => Key::Backspace,
        "DELETE" | "DEL" => Key::Delete,
        "INSERT" | "INS" => Key::Insert,
        "HOME" => Key::Home,
        "END" => Key::End,
        "PAGEUP" | "PGUP" => Key::PageUp,
        "PAGEDOWN" | "PGDN" | "PGDOWN" => Key::PageDown,
        "UP" | "UPARROW" => Key::Up,
        "DOWN" | "DOWNARROW" => Key::Down,
        "LEFT" | "LEFTARROW" => Key::Left,
        "RIGHT" | "RIGHTARROW" => Key::Right,

        // punctuation / symbols
        "MINUS" | "DASH" => Key::Minus,
        "EQUAL" | "EQUALS" => Key::Equal,
        "COMMA" => Key::Comma,
        "DOT" | "PERIOD" => Key::Dot,
        "SLASH" | "FORWARDSLASH" => Key::Slash,
        "BACKSLASH" => Key::Backslash,
        "SEMICOLON" => Key::Semicolon,
        "APOSTROPHE" | "QUOTE" => Key::Apostrophe,
        "GRAVE" | "BACKTICK" => Key::Grave,
        "LEFTBRACE" | "LBRACE" | "LEFTBRACKET" | "LBRACKET" => Key::LeftBrace,
        "RIGHTBRACE" | "RBRACE" | "RIGHTBRACKET" | "RBRACKET" => Key::RightBrace,

        // keypad
        "KP0" | "NUMPAD0" => Key::Kp0,
        "KP1" | "NUMPAD1" => Key::Kp1,
        "KP2" | "NUMPAD2" => Key::Kp2,
        "KP3" | "NUMPAD3" => Key::Kp3,
        "KP4" | "NUMPAD4" => Key::Kp4,
        "KP5" | "NUMPAD5" => Key::Kp5,
        "KP6" | "NUMPAD6" => Key::Kp6,
        "KP7" | "NUMPAD7" => Key::Kp7,
        "KP8" | "NUMPAD8" => Key::Kp8,
        "KP9" | "NUMPAD9" => Key::Kp9,
        "KPPLUS" | "NUMPADPLUS" => Key::KpPlus,
        "KPMINUS" | "NUMPADMINUS" => Key::KpMinus,
        "KPASTERISK" | "NUMPADASTERISK" | "KPSTAR" => Key::KpAsterisk,
        "KPSLASH" | "NUMPADSLASH" => Key::KpSlash,
        "KPDOT" | "NUMPADDOT" | "KPDECIMAL" => Key::KpDot,
        "KPENTER" | "NUMPADENTER" => Key::KpEnter,

        // lock / system
        "CAPSLOCK" => Key::CapsLock,
        "NUMLOCK" => Key::NumLock,
        "SCROLLLOCK" => Key::ScrollLock,
        "PAUSE" => Key::Pause,
        "MENU" => Key::Menu,

        // print screen / sysrq
        "PRTSCR" | "PRTSC" | "PRINTSCREEN" | "SYSRQ" => Key::Sysrq,
        "PRINT" | "ACPRINT" => Key::Print,

        // media / misc
        "MUTE" | "VOLUMEMUTE" => Key::Mute,
        "VOLUMEDOWN" => Key::VolumeDown,
        "VOLUMEUP" => Key::VolumeUp,
        "PLAYPAUSE" => Key::PlayPause,
        "PLAY" => Key::Play,
        "PAUSECD" => Key::PauseCD,
        "STOPCD" | "STOP" => Key::StopCD,
        "NEXTSONG" | "NEXTTRACK" => Key::NextSong,
        "PREVIOUSSONG" | "PREVSONG" | "PREVTRACK" => Key::PreviousSong,

        // brightness / illumination
        "BRIGHTNESSUP" => Key::BrightnessUp,
        "BRIGHTNESSDOWN" => Key::BrightnessDown,
        "ILLUMUP" | "KBDILLUMUP" | "KEYBOARDLIGHTUP" => Key::IllumUp,
        "ILLUMDOWN" | "KBDILLUMDOWN" | "KEYBOARDLIGHTDOWN" => Key::IllumDown,
        "ILLUMTOGGLE" | "KBDILLUMTOGGLE" | "KEYBOARDLIGHTTOGGLE" => Key::IllumToggle,

        _ => return None,
    };

    Some(key)
}

pub fn parse_key_combos(action: &str) -> Vec<input_linux::Key> {
    if !action.starts_with("KeyCombos_") {
        return Vec::new();
    }

    let combo_part = &action["KeyCombos_".len()..];
    combo_part.split('_').filter_map(parse_key_name).collect()
}
