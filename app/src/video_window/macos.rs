use objc2_app_kit::NSWindow;
use objc2_foundation::NSSize;
use tauri::WebviewWindow;

pub(super) fn constrain(window: &WebviewWindow, ratio: Option<f64>) -> tauri::Result<()> {
    // Called on AppKit's main thread while Tauri owns the window.
    let native = unsafe { &*window.ns_window()?.cast::<NSWindow>() };
    if let Some(ratio) = ratio {
        // The webview extends under the overlay title bar and fills the frame.
        native.setAspectRatio(NSSize::new(ratio, 1.));
    } else {
        // Setting increments cancels AppKit's mutual-exclusive aspect constraint.
        native.setResizeIncrements(NSSize::new(1., 1.));
    }
    Ok(())
}
