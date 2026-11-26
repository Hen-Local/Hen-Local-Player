// Window geometry is changed only on the native UI thread. Interactive dragging
// uses the platform's sizing mechanism, never a stream of webview resize events.
use anyhow::{Context, Result};
use std::cell::RefCell;
use tauri::{LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

type Size = LogicalSize<f64>;

#[derive(Default)]
struct AspectState {
    desired: Option<f64>,
    home: Option<(Size, PhysicalPosition<i32>)>,
    pending: bool,
}

thread_local! {
    static STATE: RefCell<AspectState> = RefCell::default();
}

pub fn set(window: WebviewWindow, ratio: Option<f64>) -> tauri::Result<()> {
    window.clone().run_on_main_thread(move || {
        STATE.with_borrow_mut(|state| {
            state.desired = ratio;
            state.pending = state.home.is_some() || ratio.is_some();
        });
        retry(&window);
    })
}

pub fn reset() {
    STATE.with_borrow_mut(|state| *state = AspectState::default());
}

pub fn pending() -> bool {
    STATE.with(|state| state.try_borrow().is_ok_and(|s| s.pending))
}

pub fn retry(window: &WebviewWindow) {
    STATE.with(|state| {
        // A native size change can synchronously deliver another resize event.
        let Ok(mut state) = state.try_borrow_mut() else {
            return;
        };
        if state.pending {
            if let Err(error) = apply(window, &mut state) {
                state.pending = false;
                tracing::warn!(%error, "Could not fit the window to the video");
            }
        }
    });
}

fn fitted(ratio: f64, area: f64, minimum: Size, available: Size) -> Size {
    let max_height = available.height.min(available.width / ratio);
    // Retain the normal minimum, with enough room for the compact transport
    // on narrow or very wide videos whenever the monitor can accommodate it.
    let min_height = minimum
        .height
        .min(minimum.width / ratio)
        .max(minimum.width.min(320.) / ratio)
        .max(minimum.height.min(180.))
        .min(max_height);
    let height = (area / ratio).sqrt().clamp(min_height, max_height);
    Size::new(height * ratio, height)
}

fn apply(window: &WebviewWindow, state: &mut AspectState) -> Result<()> {
    if window.is_fullscreen()? || window.is_minimized()? {
        return Ok(());
    }
    #[cfg(windows)]
    if window.is_maximized()? {
        // Windows owns the maximized rectangle. Apply to the restored window.
        return Ok(());
    }
    let screen = window
        .current_monitor()?
        .context("No monitor for player window")?;
    let scale = window.scale_factor()?;
    let work = screen.work_area();
    let current = window.inner_size()?.to_logical::<f64>(scale);
    let frame = window.outer_size()?.to_logical::<f64>(scale);
    let pos = window.outer_position()?;
    let decoration = Size::new(
        (frame.width - current.width).max(0.),
        (frame.height - current.height).max(0.),
    );
    let available = Size::new(
        (work.size.width as f64 / scale - decoration.width).max(1.),
        (work.size.height as f64 / scale - decoration.height).max(1.),
    );
    let config = window
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == window.label())
        .context("Missing main window configuration")?;
    let home_min = Size::new(
        config.min_width.unwrap_or(0.),
        config.min_height.unwrap_or(0.),
    );
    let (target, minimum, origin) = if let Some(ratio) = state.desired {
        state.home.get_or_insert((current, pos));
        let size = fitted(ratio, current.width * current.height, home_min, available);
        let minimum = fitted(ratio, 0., home_min, available);
        let origin = PhysicalPosition::new(
            pos.x + ((current.width - size.width) * scale / 2.).round() as i32,
            pos.y + ((current.height - size.height) * scale / 2.).round() as i32,
        );
        (size, minimum, origin)
    } else if let Some((home, pos)) = state.home {
        (
            Size::new(
                home.width.min(available.width),
                home.height.min(available.height),
            ),
            home_min,
            pos,
        )
    } else {
        state.pending = false;
        return Ok(());
    };

    let size: PhysicalSize<u32> = target.to_physical(scale);
    let outer: PhysicalSize<u32> = Size::new(
        target.width + decoration.width,
        target.height + decoration.height,
    )
    .to_physical(scale);
    let x = origin.x.clamp(
        work.position.x,
        work.position.x + work.size.width.saturating_sub(outer.width) as i32,
    );
    let y = origin.y.clamp(
        work.position.y,
        work.position.y + work.size.height.saturating_sub(outer.height) as i32,
    );

    #[cfg(target_os = "macos")]
    macos::constrain(window, state.desired)?;
    #[cfg(windows)]
    windows::constrain(window, state.desired)?;
    window.set_min_size(Some(minimum))?;
    if window.inner_size()? != size {
        window.set_size(size)?;
    }
    let position = PhysicalPosition::new(x, y);
    if window.outer_position()? != position {
        window.set_position(position)?;
    }
    if state.desired.is_none() {
        state.home = None;
    }
    state.pending = false;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_the_visible_workspace_for_each_video_shape() {
        let minimum = Size::new(450., 300.);
        let monitor = Size::new(1280., 700.);
        for ratio in [16. / 9., 9. / 16., 1., 4. / 3., 3., 0.2] {
            let chosen = fitted(ratio, 720. * 430., minimum, monitor);
            assert!(chosen.width <= monitor.width && chosen.height <= monitor.height);
            assert!((chosen.width / chosen.height - ratio).abs() < 1e-8);
            let smallest = fitted(ratio, 0., minimum, monitor);
            assert!(smallest.width <= monitor.width && smallest.height <= monitor.height);
            assert!(smallest.width >= 320. || smallest.height == monitor.height);
            assert!(smallest.height >= 180. || smallest.width == monitor.width);
        }
        assert_eq!(fitted(16. / 9., 0., minimum, monitor).width, 450.);
        assert_eq!(fitted(9. / 16., 0., minimum, monitor).width, 320.);
    }
}
