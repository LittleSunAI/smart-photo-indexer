//! Keep scripts/queues alive while inactive WebViews release disposable browser caches.
use crate::*;
use std::sync::atomic::{AtomicU64, Ordering};
use windows::core::Interface;
use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL};
static LOW: AtomicU64 = AtomicU64::new(0);
static NORMAL: AtomicU64 = AtomicU64::new(0);
static APPLIED: AtomicU64 = AtomicU64::new(0);
static UNSUPPORTED: AtomicU64 = AtomicU64::new(0);

pub fn focus(window: &tauri::Window, active: bool) {
    let Some(webview) = window.app_handle().get_webview_window(window.label()) else { return; };
    if active { NORMAL.fetch_add(1,Ordering::Relaxed); } else { LOW.fetch_add(1,Ordering::Relaxed); }
    let result = webview.with_webview(move |platform| {
        // Tauri invokes this on the owning UI thread. No COM pointer crosses apartments.
        let result = unsafe {
            platform.controller().CoreWebView2()
                .and_then(|core| core.cast::<ICoreWebView2_19>())
                .and_then(|core| core.SetMemoryUsageTargetLevel(COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL(if active { 0 } else { 1 })))
        };
        if result.is_ok() { APPLIED.fetch_add(1,Ordering::Relaxed); }
        else { UNSUPPORTED.fetch_add(1,Ordering::Relaxed); }
    });
    if result.is_err() { UNSUPPORTED.fetch_add(1,Ordering::Relaxed); }
}

#[tauri::command]
pub fn get_webview_memory_statistics() -> serde_json::Value {
    serde_json::json!({"lowRequests":LOW.load(Ordering::Relaxed),"normalRequests":NORMAL.load(Ordering::Relaxed),"applied":APPLIED.load(Ordering::Relaxed),"unsupportedOrClosed":UNSUPPORTED.load(Ordering::Relaxed),"policy":"low-when-inactive-normal-when-focused-no-script-suspension"})
}
