use block2::{RcBlock, StackBlock};
use objc2_app_kit::{NSAppearance, NSColor, NSColorSpace, NSSystemColorsDidChangeNotification};
use objc2_foundation::{NSNotificationCenter, NSOperationQueue};
use std::cell::Cell;

pub fn current(appearance: &NSAppearance) -> Option<String> {
    let rgb = Cell::new(None);
    appearance.performAsCurrentDrawingAppearance(&StackBlock::new(|| {
        rgb.set(
            NSColor::controlAccentColor().colorUsingColorSpace(&NSColorSpace::sRGBColorSpace()),
        );
    }));
    let color = rgb.take()?;
    let byte = |component: f64| (component * 255.).round() as u8;
    let r = byte(color.redComponent());
    let g = byte(color.greenComponent());
    let b = byte(color.blueComponent());
    Some(format!("#{r:02x}{g:02x}{b:02x}"))
}

/// Register once for the app lifetime, including periods with no open window.
pub fn observe(on_change: impl Fn() + Send + Sync + 'static) -> impl FnOnce() {
    let center = NSNotificationCenter::defaultCenter();
    let callback = RcBlock::new(move |_| on_change());
    // The notification has no sender. Resolve the dynamic color on the UI thread.
    let token = unsafe {
        center.addObserverForName_object_queue_usingBlock(
            Some(NSSystemColorsDidChangeNotification),
            None,
            Some(&NSOperationQueue::mainQueue()),
            &callback,
        )
    };
    // Use the same registration token while the notification center is still alive.
    move || unsafe { center.removeObserver((*token).as_ref()) }
}
