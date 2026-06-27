use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};

pub fn show(app: &AppHandle, phase: &str, text: Option<&str>) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("overlay") {
        w.show()?;
        position_near_cursor(&w)?;
        let _ = app.emit_to("overlay", "overlay-update", serde_json::json!({
            "phase": phase, "text": text,
        }));
    }
    Ok(())
}

pub fn hide(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("overlay") {
        w.hide()?;
    }
    Ok(())
}

fn position_near_cursor(w: &tauri::WebviewWindow) -> tauri::Result<()> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
        unsafe {
            let mut pt = windows_sys::Win32::Foundation::POINT { x: 0, y: 0 };
            if GetCursorPos(&mut pt) != 0 {
                let _ = w.set_position(PhysicalPosition::new(pt.x + 16, pt.y + 24));
            }
        }
    }
    let _ = w;
    Ok(())
}