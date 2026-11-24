#[cfg(target_os = "macos")]
extern "C" {
    fn hlp_set_player_fullscreen(webview: *mut std::ffi::c_void, enabled: bool);
}

#[tauri::command]
pub fn set_player_fullscreen(window: tauri::WebviewWindow, enabled: bool) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    {
        window.with_webview(move |webview| {
            // SAFETY: Tauri supplies this WKWebView on the main thread. The
            // Objective-C helper holds only a weak reference to it.
            unsafe { hlp_set_player_fullscreen(webview.inner(), enabled) }
        })
    }
    #[cfg(not(target_os = "macos"))]
    window.set_fullscreen(enabled)
}
