//! Opens one shell window, places its controls, prints what happened as one JSON line, and quits after a few seconds.
//!
//! `cargo run --features appkit --example probe -- [seconds]`. Run it with `WINDOW_SHELL_BACKGROUND=1` beside someone's
//! work: the window is shown and can be captured by its `window` number, and keyboard focus stays where it was.

use block2::RcBlock;
use objc2::{rc::Retained, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSAutoresizingMaskOptions, NSBackingStoreType, NSWindow, NSWindowStyleMask,
    NSWindowTitleVisibility,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSTimer};
use objc2_web_kit::{WKWebView, WKWebViewConfiguration};
use std::{cell::RefCell, rc::Rc};
use window_shell_geometry::{
    appkit::{activation_policy, observe_controls, present},
    Launch,
};

fn main() {
    let seconds: f64 = std::env::args().nth(1).map_or(4.0, |value| {
        value.parse().expect("seconds must be a number")
    });
    let main = MainThreadMarker::new().expect("the probe runs on the main thread");
    let app = NSApplication::sharedApplication(main);
    app.setActivationPolicy(activation_policy());
    let frame = NSRect::new(NSPoint::new(200.0, 200.0), NSSize::new(900.0, 620.0));
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(main),
            frame,
            NSWindowStyleMask::Titled
                | NSWindowStyleMask::Closable
                | NSWindowStyleMask::Miniaturizable
                | NSWindowStyleMask::Resizable
                | NSWindowStyleMask::FullSizeContentView,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe { window.setReleasedWhenClosed(false) };
    window.setTitlebarAppearsTransparent(true);
    window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
    let content = window
        .contentView()
        .expect("a titled window has a content view");
    let webview = unsafe {
        WKWebView::initWithFrame_configuration(
            WKWebView::alloc(main),
            content.bounds(),
            &WKWebViewConfiguration::new(main),
        )
    };
    webview.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    content.addSubview(&webview);
    present(main, &window);
    let outcomes: Rc<RefCell<Vec<String>>> = Rc::default();
    let record = outcomes.clone();
    let observer = observe_controls(main, &window, &webview, move |outcome| {
        record.borrow_mut().push(match outcome {
            Ok(controls) => serde_json::to_string(&controls).unwrap(),
            Err(error) => format!("{{\"error\":{}}}", serde_json::to_string(&error).unwrap()),
        });
    });
    let report = RcBlock::new(move |_: std::ptr::NonNull<NSTimer>| {
        println!(
            "{{\"launch\":\"{:?}\",\"window\":{},\"pid\":{},\"key\":{},\"visible\":{},\"controls\":[{}]}}",
            Launch::current(),
            window.windowNumber(),
            std::process::id(),
            window.isKeyWindow(),
            window.isVisible(),
            outcomes.borrow().join(","),
        );
        window.close();
        NSApplication::sharedApplication(main).terminate(None);
    });
    let _timer: Retained<NSTimer> =
        unsafe { NSTimer::scheduledTimerWithTimeInterval_repeats_block(seconds, false, &report) };
    app.run();
    drop(observer);
}
