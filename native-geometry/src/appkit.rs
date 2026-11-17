//! Present AppKit windows under the process's [`Launch`] mode, and measure and place their window controls in the
//! window's logical coordinate space.

use crate::{
    button_placement, controls_reservation, header, union, verify, Launch, PlacementError, Rect,
    WindowControls,
};
use block2::RcBlock;
use objc2::{
    rc::Retained,
    runtime::{NSObjectProtocol, ProtocolObject},
    MainThreadMarker, Message,
};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationOptions, NSApplicationActivationPolicy,
    NSRunningApplication, NSView, NSWindow, NSWindowButton,
    NSWindowDidChangeBackingPropertiesNotification, NSWindowDidEnterFullScreenNotification,
    NSWindowDidExitFullScreenNotification, NSWindowDidResizeNotification,
    NSWindowDidUpdateNotification, NSWindowStyleMask, NSWorkspaceOpenConfiguration,
};
use objc2_foundation::{NSNotification, NSNotificationCenter};
use objc2_web_kit::WKWebView;
use std::{cell::Cell, cell::RefCell, ptr::NonNull, rc::Rc};

const BUTTONS: [NSWindowButton; 3] = [
    NSWindowButton::CloseButton,
    NSWindowButton::MiniaturizeButton,
    NSWindowButton::ZoomButton,
];

/// The activation policy a host launches with. A background launch is prohibited from activating, which also stops
/// AppKit, LaunchServices and windowing toolkits from activating it on its behalf.
pub fn activation_policy() -> NSApplicationActivationPolicy {
    match Launch::current() {
        Launch::Foreground => NSApplicationActivationPolicy::Regular,
        Launch::Background => NSApplicationActivationPolicy::Prohibited,
    }
}

/// Brings `window` to the front, restoring it if it is minimized. In the foreground the window becomes key and the
/// application activates; in the background the window is ordered front without either.
pub fn present(main: MainThreadMarker, window: &NSWindow) {
    if window.isMiniaturized() {
        window.deminiaturize(None);
    }
    match Launch::current() {
        Launch::Foreground => {
            window.makeKeyAndOrderFront(None);
            #[allow(deprecated)]
            NSApplication::sharedApplication(main).activateIgnoringOtherApps(true);
        }
        Launch::Background => window.orderFrontRegardless(),
    }
}

/// Activates another running application, or in the background leaves it where it is.
pub fn activate(application: &NSRunningApplication) {
    if Launch::current().takes_focus() {
        application.activateWithOptions(NSApplicationActivationOptions::empty());
    }
}

/// A configuration for opening an application that activates it only in the foreground.
pub fn open_configuration() -> Retained<NSWorkspaceOpenConfiguration> {
    let configuration = NSWorkspaceOpenConfiguration::configuration();
    configuration.setActivates(Launch::current().takes_focus());
    configuration
}

fn missing(reason: &'static str) -> PlacementError {
    PlacementError::Measurement { reason }
}

pub fn rect_in_window(native: &NSWindow, view: &NSView) -> Result<Rect, PlacementError> {
    let owner = view
        .window()
        .ok_or(missing("Native view is not attached to a window"))?;
    let screen = owner.convertRectToScreen(view.convertRect_toView(view.bounds(), None));
    let window = native.frame();
    Ok(Rect {
        left: screen.origin.x - window.origin.x,
        top: window.origin.y + window.size.height - screen.origin.y - screen.size.height,
        width: screen.size.width,
        height: screen.size.height,
    })
}

pub fn standard_buttons(native: &NSWindow) -> Result<Vec<Rect>, PlacementError> {
    BUTTONS
        .into_iter()
        .map(|kind| {
            let button = native
                .standardWindowButton(kind)
                .ok_or(missing("Missing native window button"))?;
            rect_in_window(native, &button)
        })
        .collect()
}

/// Centres the standard buttons in the title bar above `webview`, verifies they are there, and reports the result.
pub fn place_controls(
    native: &NSWindow,
    webview: &WKWebView,
) -> Result<WindowControls, PlacementError> {
    native.layoutIfNeeded();
    if native.styleMask().contains(NSWindowStyleMask::FullScreen) {
        return Ok(WindowControls::None);
    }
    let scale = unsafe { webview.pageZoom() };
    let header = header(rect_in_window(native, webview)?, scale);
    let before = standard_buttons(native)?;
    let desired = button_placement(header, &before);
    let tolerance = 0.5 / native.backingScaleFactor();
    let window = native.frame();
    for ((kind, rect), &(left, top)) in BUTTONS.into_iter().zip(&before).zip(&desired) {
        if (rect.left - left).abs() <= tolerance && (rect.top - top).abs() <= tolerance {
            continue;
        }
        let button = native
            .standardWindowButton(kind)
            .ok_or(missing("Missing native window button"))?;
        let parent =
            unsafe { button.superview() }.ok_or(missing("Missing native button parent"))?;
        let owner = parent
            .window()
            .ok_or(missing("Native button parent has no window"))?;
        let mut screen = button.bounds();
        screen.origin.x = window.origin.x + left;
        screen.origin.y = window.origin.y + window.size.height - top - rect.height;
        let local = parent.convertRect_fromView(owner.convertRectFromScreen(screen), None);
        button.setFrameOrigin(local.origin);
    }
    let after = standard_buttons(native)?;
    let bounds = union(&after);
    verify(header, before, after, desired, tolerance)?;
    Ok(WindowControls::Overlay {
        reservation: controls_reservation(header, bounds, scale),
    })
}

type Report = dyn Fn(Result<WindowControls, PlacementError>);

struct Placer {
    native: Retained<NSWindow>,
    webview: Retained<WKWebView>,
    report: Box<Report>,
    placing: Cell<bool>,
    last: RefCell<Option<Result<WindowControls, PlacementError>>>,
}

impl Placer {
    fn place(&self) {
        if self.placing.replace(true) {
            return;
        }
        let outcome = place_controls(&self.native, &self.webview);
        self.placing.set(false);
        let changed = match (&*self.last.borrow(), &outcome) {
            (Some(Ok(previous)), Ok(current)) => previous != current,
            (Some(Err(_)), Err(_)) => false,
            _ => true,
        };
        if changed {
            self.last.replace(Some(outcome.clone()));
            (self.report)(outcome);
        }
    }
}

/// Keeps a window's controls placed for as long as it lives; dropping it stops the observation.
pub struct ControlsObserver {
    placer: Rc<Placer>,
    center: Retained<NSNotificationCenter>,
    tokens: Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>>,
}

impl ControlsObserver {
    /// The latest outcome, for a host to deliver again to a page that has just loaded.
    pub fn current(&self) -> Result<WindowControls, PlacementError> {
        self.placer
            .last
            .borrow()
            .clone()
            .expect("the observer places the controls when it is created")
    }
}

impl Drop for ControlsObserver {
    fn drop(&mut self) {
        for token in &self.tokens {
            unsafe { self.center.removeObserver(token.as_ref()) };
        }
    }
}

/// Places the controls now and again whenever the window updates, resizes, changes scale, or enters or leaves fullscreen.
///
/// `report` receives the first outcome and every change of outcome after it: a new `WindowControls`, or the first
/// failure after a success. It runs on the main thread, inside the AppKit notification that caused it.
pub fn observe_controls(
    _main: MainThreadMarker,
    native: &NSWindow,
    webview: &WKWebView,
    report: impl Fn(Result<WindowControls, PlacementError>) + 'static,
) -> ControlsObserver {
    let placer = Rc::new(Placer {
        native: native.retain(),
        webview: webview.retain(),
        report: Box::new(report),
        placing: Cell::new(false),
        last: RefCell::new(None),
    });
    placer.place();
    let center = NSNotificationCenter::defaultCenter();
    let tokens = unsafe {
        [
            NSWindowDidUpdateNotification,
            NSWindowDidResizeNotification,
            NSWindowDidChangeBackingPropertiesNotification,
            NSWindowDidEnterFullScreenNotification,
            NSWindowDidExitFullScreenNotification,
        ]
    }
    .into_iter()
    .map(|name| {
        let placer = placer.clone();
        let callback = RcBlock::new(move |_: NonNull<NSNotification>| placer.place());
        unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(name),
                Some(native),
                None,
                &callback,
            )
        }
    })
    .collect();
    ControlsObserver {
        placer,
        center,
        tokens,
    }
}
