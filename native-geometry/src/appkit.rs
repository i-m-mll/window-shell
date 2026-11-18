//! Measure AppKit controls in the window's logical coordinate space.

use crate::{controls_reservation, union, Rect};
use objc2_app_kit::{NSView, NSWindow, NSWindowButton, NSWindowStyleMask};

pub fn rect_in_window(native: &NSWindow, view: &NSView) -> Result<Rect, &'static str> {
    let owner = view
        .window()
        .ok_or("Native view is not attached to a window")?;
    let screen = owner.convertRectToScreen(view.convertRect_toView(view.bounds(), None));
    let window = native.frame();
    Ok(Rect {
        left: screen.origin.x - window.origin.x,
        top: window.origin.y + window.size.height - screen.origin.y - screen.size.height,
        width: screen.size.width,
        height: screen.size.height,
    })
}

pub fn standard_buttons(native: &NSWindow) -> Result<Vec<Rect>, &'static str> {
    [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ]
    .into_iter()
    .map(|kind| {
        let button = native
            .standardWindowButton(kind)
            .ok_or("Missing native window button")?;
        rect_in_window(native, &button)
    })
    .collect()
}

/// The leading CSS space occupied by the measured controls above an overlay webview.
pub fn overlay_reservation(
    native: &NSWindow,
    webview: &NSView,
    page_zoom: f64,
) -> Result<f64, &'static str> {
    if native.styleMask().contains(NSWindowStyleMask::FullScreen) {
        return Ok(0.0);
    }
    let bounds = union(&standard_buttons(native)?);
    let webview = rect_in_window(native, webview)?;
    Ok(controls_reservation(webview, bounds, page_zoom))
}
