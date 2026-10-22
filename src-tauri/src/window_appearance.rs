//! A native window becomes visible only after its frontend has supplied the
//! saved appearance. This also handles existing, browser-only preferences on
//! upgrade without a second settings store or a visible OS-theme flash.
use std::collections::HashSet;
use std::sync::Mutex;
use tauri::{Manager, Theme, WebviewWindow};

#[derive(Default)]
pub struct WindowAppearance(Mutex<Appearance>);

#[derive(Default)]
struct Appearance {
    theme: Option<Theme>,
    initialized: HashSet<String>,
}

pub fn theme(app: &tauri::AppHandle) -> Theme {
    app.state::<WindowAppearance>().0.lock().ok()
        .and_then(|state| state.theme).unwrap_or(Theme::Light)
}

// Owned-profile integration checks can exercise real WebViews without taking
// over the user's desktop. Release builds never honor this diagnostic flag.
pub fn hidden_test_windows() -> bool {
    cfg!(debug_assertions)
        && std::env::var("SMART_PHOTO_TEST_HIDDEN_WINDOWS").as_deref() == Ok("1")
}

pub fn present(window: &WebviewWindow) -> Result<(), String> {
    if hidden_test_windows() { return Ok(()); }
    let ready = window.state::<WindowAppearance>().0.lock()
        .map_err(|_| "Window appearance lock failed.")?
        .initialized.contains(window.label());
    if ready {
        window.unminimize().map_err(|e| e.to_string())?;
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn remove(app: &tauri::AppHandle, label: &str) {
    if let Ok(mut state) = app.state::<WindowAppearance>().0.lock() {
        state.initialized.remove(label);
    }
}

#[cfg(target_os = "windows")]
fn caption_colors(window: &WebviewWindow, theme: Theme) -> Result<(), String> {
    use windows_sys::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
    };
    // COLORREF is BGR. Match the application's blue-soft/panel-soft palettes;
    // let Windows retain native hit testing, caption buttons and resizing.
    let (background, text): (u32, u32) = match theme {
        Theme::Dark => (0x00443224, 0x00f8f2ed), // #243244, #edf2f8
        _ => (0x00fff1e8, 0x002a2019),          // #e8f1ff, #19202a
    };
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    for (attribute, value) in [(DWMWA_CAPTION_COLOR, background), (DWMWA_TEXT_COLOR, text)] {
        let result = unsafe {
            DwmSetWindowAttribute(hwnd.0 as _, attribute as u32,
                &value as *const u32 as _, std::mem::size_of::<u32>() as u32)
        };
        // Caption color attributes are available on Windows 11. Older Windows
        // retains its native light/dark caption, without a simulated overlay.
        if result == 0x80070057_u32 as i32 { return Ok(()); }
        if result < 0 { return Err(format!("Cannot set native caption color: {result:#x}")); }
    }
    Ok(())
}

#[tauri::command]
pub fn apply_window_appearance(window: WebviewWindow, theme: String) -> Result<(), String> {
    let theme = match theme.as_str() {
        "light" => Theme::Light,
        "dark" => Theme::Dark,
        _ => return Err("Unsupported window theme.".into()),
    };
    // Queue the native theme before ShowWindow. Repeated changes do not reveal
    // a window that the user has subsequently hidden or minimized.
    let applied = window.set_theme(Some(theme)).map_err(|e| e.to_string());
    #[cfg(target_os = "windows")]
    let applied = applied.and_then(|_| caption_colors(&window, theme));
    let first = {
        let appearance = window.state::<WindowAppearance>();
        let mut state = appearance.0.lock().map_err(|_| "Window appearance lock failed.")?;
        state.theme = Some(theme);
        state.initialized.insert(window.label().to_owned())
    };
    if first && !hidden_test_windows() {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }
    applied
}
