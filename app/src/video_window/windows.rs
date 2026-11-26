use anyhow::{ensure, Result};
use tauri::WebviewWindow;
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    UI::{
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::{
            GetClientRect, GetWindowRect, WMSZ_BOTTOMLEFT, WMSZ_BOTTOMRIGHT, WMSZ_LEFT, WMSZ_RIGHT,
            WMSZ_TOPLEFT, WMSZ_TOPRIGHT, WM_NCDESTROY, WM_SIZING,
        },
    },
};

// Storing the ratio itself in the callback reference data avoids allocating
// a pointer whose lifetime would have to track the HWND.
const SUBCLASS: usize = 1;
pub(super) fn constrain(window: &WebviewWindow, ratio: Option<f64>) -> Result<()> {
    let hwnd = window.hwnd()?.0;
    // Both installation and removal run on the thread that owns the window.
    unsafe {
        if let Some(ratio) = ratio {
            ensure!(
                SetWindowSubclass(
                    hwnd,
                    Some(size_message),
                    SUBCLASS,
                    (ratio as f32).to_bits() as usize
                ) != 0,
                "Could not constrain the playback window"
            );
        } else {
            RemoveWindowSubclass(hwnd, Some(size_message), SUBCLASS);
        }
    }
    Ok(())
}

unsafe extern "system" fn size_message(
    hwnd: HWND,
    msg: u32,
    edge: WPARAM,
    data: LPARAM,
    id: usize,
    reference: usize,
) -> LRESULT {
    if msg == WM_NCDESTROY {
        unsafe {
            RemoveWindowSubclass(hwnd, Some(size_message), id);
        }
    }
    if msg == WM_SIZING && data != 0 {
        let mut frame = RECT::default();
        let mut client = RECT::default();
        if unsafe { GetWindowRect(hwnd, &mut frame) != 0 && GetClientRect(hwnd, &mut client) != 0 }
        {
            let ratio = f32::from_bits(reference as u32) as f64;
            let rect = unsafe { &mut *(data as *mut RECT) };
            // Both rectangles are in physical pixels at the window's current DPI.
            let inset_w = frame.right - frame.left - (client.right - client.left);
            let inset_h = frame.bottom - frame.top - (client.bottom - client.top);
            let w = (rect.right - rect.left - inset_w).max(1) as f64;
            let h = (rect.bottom - rect.top - inset_h).max(1) as f64;
            let edge = edge as u32;
            let corner = matches!(
                edge,
                WMSZ_TOPLEFT | WMSZ_TOPRIGHT | WMSZ_BOTTOMLEFT | WMSZ_BOTTOMRIGHT
            );
            let by_width = matches!(edge, WMSZ_LEFT | WMSZ_RIGHT)
                || (corner
                    && (w - (client.right - client.left) as f64).abs()
                        >= (h - (client.bottom - client.top) as f64).abs() * ratio);
            if by_width {
                let height = (w / ratio).round() as i32 + inset_h;
                if matches!(edge, WMSZ_TOPLEFT | WMSZ_TOPRIGHT) {
                    rect.top = rect.bottom - height;
                } else {
                    rect.bottom = rect.top + height;
                }
            } else {
                let width = (h * ratio).round() as i32 + inset_w;
                if matches!(edge, WMSZ_TOPLEFT | WMSZ_BOTTOMLEFT) {
                    rect.left = rect.right - width;
                } else {
                    rect.right = rect.left + width;
                }
            }
            return 1;
        }
    }
    unsafe { DefSubclassProc(hwnd, msg, edge, data) }
}
