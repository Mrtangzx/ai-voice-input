use std::sync::atomic::{AtomicBool, Ordering};

static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn request_stop() { STOP_REQUESTED.store(true, Ordering::SeqCst); }
pub fn reset_stop() { STOP_REQUESTED.store(false, Ordering::SeqCst); }
pub fn should_stop() -> bool { STOP_REQUESTED.load(Ordering::SeqCst) }

pub fn parse_hotkey(s: &str) -> Option<tauri_plugin_global_shortcut::Shortcut> {
    use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};
    let mut parts = s.split('+').map(str::trim).collect::<Vec<_>>();
    let key = parts.pop()?.to_lowercase();
    let code = match key.as_str() {
        "space" => Code::Space,
        "v" => Code::KeyV,
        "t" => Code::KeyT,
        "z" => Code::KeyZ,
        "enter" | "return" => Code::Enter,
        _ => return None,
    };
    let mut mods = Modifiers::empty();
    for p in &parts {
        match p.to_lowercase().as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" => mods |= Modifiers::ALT,
            "super" | "win" | "meta" => mods |= Modifiers::META,
            _ => return None,
        }
    }
    Some(Shortcut::new(Some(mods), code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_default_hotkey() {
        let sc = parse_hotkey("Ctrl+Shift+Space").unwrap();
        // Just verify it parses without panic; structural check requires deeper API
        let _ = sc;
    }

    #[test]
    fn parse_alternative_hotkey() {
        assert!(parse_hotkey("Ctrl+Alt+V").is_some());
    }

    #[test]
    fn parse_unknown_key_returns_none() {
        assert!(parse_hotkey("Ctrl+Unknown").is_none());
    }

    #[test]
    fn stop_flag_round_trip() {
        reset_stop();
        assert!(!should_stop());
        request_stop();
        assert!(should_stop());
        reset_stop();
        assert!(!should_stop());
    }
}