//! The native half of the window shell: how a host launches, where its window controls go, and the root attributes
//! through which it reports them to the page.

#[cfg(all(target_os = "macos", feature = "appkit"))]
pub mod appkit;

use serde::{Deserialize, Serialize};
use std::{env, ffi::OsStr, fmt, sync::OnceLock};

/// How a native host presents itself, read once per process from [`Launch::VARIABLE`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Launch {
    /// An ordinary launch: the application activates and the window it presents becomes key.
    Foreground,
    /// A launch for verification beside someone's work: windows are shown and ordered front, but the application never
    /// activates and no window becomes key, so keyboard focus stays where it was.
    Background,
}

impl Launch {
    /// `WINDOW_SHELL_BACKGROUND=1` selects [`Launch::Background`]; unset selects [`Launch::Foreground`].
    pub const VARIABLE: &'static str = "WINDOW_SHELL_BACKGROUND";

    /// The process's launch mode. Any value of [`Launch::VARIABLE`] other than `1` is a launcher defect and panics.
    pub fn current() -> Self {
        static CURRENT: OnceLock<Launch> = OnceLock::new();
        *CURRENT.get_or_init(|| {
            Self::parse(env::var_os(Self::VARIABLE).as_deref()).unwrap_or_else(|value| {
                panic!("{} must be unset or 1, found {value:?}", Self::VARIABLE)
            })
        })
    }

    fn parse(value: Option<&OsStr>) -> Result<Self, String> {
        match value {
            None => Ok(Self::Foreground),
            Some(value) if value == "1" => Ok(Self::Background),
            Some(value) => Err(value.to_string_lossy().into_owned()),
        }
    }

    /// Whether the windows this process creates or presents may take keyboard focus.
    pub fn takes_focus(self) -> bool {
        self == Self::Foreground
    }
}

/// The title bar's height in CSS pixels, read from `--shell-header-height` in `browser/tokens.css`.
mod tokens {
    include!(concat!(env!("OUT_DIR"), "/tokens.rs"));
}
pub use tokens::HEADER_HEIGHT;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

/// Where the window controls are, as the page lays out from it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum WindowControls {
    /// The controls are drawn over the page's title bar and occupy `reservation` CSS pixels at its leading edge.
    Overlay { reservation: f64 },
    /// The window has no controls over the page (fullscreen).
    None,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Deviation {
    pub header: Rect,
    pub before: Vec<Rect>,
    pub after: Vec<Rect>,
    pub desired: Vec<(f64, f64)>,
    pub center_error: f64,
    pub left_error: f64,
    pub tolerance: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum PlacementError {
    /// AppKit did not provide a view the placement needs.
    Measurement { reason: &'static str },
    /// The controls were moved but are not where the placement put them.
    OffCentre(Box<Deviation>),
}

impl fmt::Display for PlacementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Measurement { reason } => f.write_str(reason),
            Self::OffCentre(deviation) => write!(
                f,
                "window controls are off centre after placement: centre error {}, left error {}, tolerance {}",
                deviation.center_error, deviation.left_error, deviation.tolerance
            ),
        }
    }
}

impl std::error::Error for PlacementError {}

pub fn union(rects: &[Rect]) -> Rect {
    let left = rects
        .iter()
        .map(|rect| rect.left)
        .fold(f64::INFINITY, f64::min);
    let top = rects
        .iter()
        .map(|rect| rect.top)
        .fold(f64::INFINITY, f64::min);
    let right = rects
        .iter()
        .map(|rect| rect.left + rect.width)
        .fold(f64::NEG_INFINITY, f64::max);
    let bottom = rects
        .iter()
        .map(|rect| rect.top + rect.height)
        .fold(f64::NEG_INFINITY, f64::max);
    Rect {
        left,
        top,
        width: right - left,
        height: bottom - top,
    }
}

/// The title bar in window logical points, for a webview at `webview` drawn at `scale` points per CSS pixel.
pub fn header(webview: Rect, scale: f64) -> Rect {
    Rect {
        height: HEADER_HEIGHT * scale,
        ..webview
    }
}

/// The origins that centre each button vertically in `header` and inset the group by the same padding from its leading edge.
pub fn button_placement(header: Rect, buttons: &[Rect]) -> Vec<(f64, f64)> {
    let group = union(buttons);
    let padding = (header.height - group.height) / 2.0;
    assert!(
        padding >= 0.0,
        "native controls exceed the frame header height"
    );
    buttons
        .iter()
        .map(|button| {
            (
                header.left + padding + button.left - group.left,
                header.top + (header.height - button.height) / 2.0,
            )
        })
        .collect()
}

/// Checks that the buttons as measured after placement are at `desired` within `tolerance`.
pub fn verify(
    header: Rect,
    before: Vec<Rect>,
    after: Vec<Rect>,
    desired: Vec<(f64, f64)>,
    tolerance: f64,
) -> Result<(), PlacementError> {
    let center_error = after
        .iter()
        .map(|rect| (rect.top + rect.height / 2.0 - header.top - header.height / 2.0).abs())
        .fold(0.0, f64::max);
    let left_error = after
        .iter()
        .zip(&desired)
        .map(|(rect, &(left, _))| (rect.left - left).abs())
        .fold(0.0, f64::max);
    if center_error <= tolerance && left_error <= tolerance {
        return Ok(());
    }
    Err(PlacementError::OffCentre(Box::new(Deviation {
        header,
        before,
        after,
        desired,
        center_error,
        left_error,
        tolerance,
    })))
}

pub fn controls_reservation(header: Rect, bounds: Rect, scale: f64) -> f64 {
    let left_padding = bounds.left - header.left;
    (bounds.left + bounds.width + left_padding * 1.5 - header.left) / scale
}

pub fn map_css(rect: Rect, webview: Rect, offset: (f64, f64), scale: f64) -> Rect {
    Rect {
        left: webview.left + (rect.left - offset.0) * scale,
        top: webview.top + (rect.top - offset.1) * scale,
        width: rect.width * scale,
        height: rect.height * scale,
    }
}

fn on_root(body: &str) -> String {
    format!(
        "(() => {{ const apply = (root) => {{ {body} }}; \
         if (document.documentElement) apply(document.documentElement); \
         else new MutationObserver((_, observer) => {{ \
           if (!document.documentElement) return; \
           observer.disconnect(); apply(document.documentElement); \
         }}).observe(document, {{ childList: true }}); }})();"
    )
}

/// Marks the page as hosted natively. Install it to run before the page's own scripts.
pub fn host_identity_script() -> String {
    on_root("root.setAttribute('data-shell-host', 'native');")
}

/// Brings the root's `data-shell-window-controls` and `--shell-window-controls-reservation` to `controls`.
pub fn window_controls_script(controls: WindowControls) -> String {
    let (kind, reservation) = match controls {
        WindowControls::Overlay { reservation } => ("overlay", reservation),
        WindowControls::None => ("none", 0.0),
    };
    assert!(
        reservation.is_finite(),
        "window controls reservation is not finite"
    );
    on_root(&format!(
        "root.setAttribute('data-shell-window-controls', '{kind}'); \
         root.style.setProperty('--shell-window-controls-reservation', '{reservation}px');"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(left: f64, top: f64, width: f64, height: f64) -> Rect {
        Rect {
            left,
            top,
            width,
            height,
        }
    }

    #[test]
    fn centers_native_buttons_and_reserves_css_space_for_the_measured_group() {
        let header = rect(16.0, 11.0, 900.0, 42.0);
        let controls = [
            rect(58.0, 15.0, 12.0, 12.0),
            rect(78.0, 15.0, 12.0, 12.0),
            rect(98.0, 15.0, 12.0, 12.0),
        ];
        let origins = button_placement(header, &controls);
        assert_eq!(origins, [(31.0, 26.0), (51.0, 26.0), (71.0, 26.0)]);
        let placed: Vec<_> = origins
            .iter()
            .zip(controls)
            .map(|(&(left, top), button)| Rect {
                left,
                top,
                ..button
            })
            .collect();
        assert_eq!(button_placement(header, &placed), origins);
        assert_eq!(
            verify(
                header,
                controls.to_vec(),
                placed.clone(),
                origins.clone(),
                0.25
            ),
            Ok(())
        );
        assert_eq!(controls_reservation(header, union(&placed), 2.0), 44.75);
    }

    #[test]
    fn placement_that_did_not_take_effect_fails_verification() {
        let header = rect(0.0, 0.0, 900.0, 42.0);
        let controls = vec![rect(20.0, 4.0, 14.0, 16.0), rect(40.0, 4.0, 14.0, 16.0)];
        let origins = button_placement(header, &controls);
        let Err(PlacementError::OffCentre(deviation)) =
            verify(header, controls.clone(), controls, origins, 0.25)
        else {
            panic!("unmoved controls passed verification");
        };
        assert_eq!(deviation.center_error, 9.0);
        assert_eq!(deviation.left_error, 7.0);
    }

    #[test]
    fn only_an_explicit_one_selects_the_background_launch() {
        assert_eq!(Launch::parse(None), Ok(Launch::Foreground));
        assert_eq!(Launch::parse(Some(OsStr::new("1"))), Ok(Launch::Background));
        for value in ["", "0", "true"] {
            assert!(Launch::parse(Some(OsStr::new(value))).is_err());
        }
    }

    #[test]
    fn maps_offset_and_scaled_css_to_window_logical_points() {
        assert_eq!(
            map_css(
                rect(30.0, 18.0, 10.0, 12.0),
                rect(100.0, 20.0, 900.0, 500.0),
                (8.0, 4.0),
                1.5,
            ),
            rect(133.0, 41.0, 15.0, 18.0),
        );
    }
}
