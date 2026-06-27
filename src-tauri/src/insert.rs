use anyhow::Result;
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

/// Writes text to clipboard, simulates Ctrl+V, then restores original clipboard.
pub fn atomic_paste(text: String) -> Result<()> {
    let mut cb = Clipboard::new()?;
    let original = cb.get_text().unwrap_or_default();

    cb.set_text(text)?;

    let mut enigo = Enigo::new(&Settings::default())?;
    enigo.key(Key::Control, Direction::Press)?;
    enigo.key(Key::Unicode('v'), Direction::Click)?;
    enigo.key(Key::Control, Direction::Release)?;

    std::thread::sleep(std::time::Duration::from_millis(60));

    let _ = cb.set_text(original);
    Ok(())
}

/// Copy text to clipboard without simulating paste.
pub fn copy_only(text: String) -> Result<()> {
    let mut cb = Clipboard::new()?;
    cb.set_text(text)?;
    Ok(())
}

/// Check if the foreground window is one that commonly ignores synthetic input.
pub fn foreground_rejects_text() -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() { return true; }
            let mut buf = [0u16; 256];
            let len = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
            if len <= 0 { return false; }
            let title = String::from_utf16_lossy(&buf[..len as usize]);
            let lower = title.to_lowercase();
            return lower.contains("task manager")
                || lower.contains("任务管理器")
                || lower.contains("program manager");
        }
    }
    #[cfg(not(windows))]
    { false }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreground_rejects_text_returns_bool() {
        // Just verify it doesn't panic.
        let _ = foreground_rejects_text();
    }
}