//! Window-control placement and conversion between logical points and CSS pixels.

#[cfg(all(target_os = "macos", feature = "appkit"))]
pub mod appkit;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

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

pub fn button_placement(header: Rect, buttons: &[Rect]) -> (Vec<(f64, f64)>, f64) {
    let group = union(buttons);
    let padding = (header.height - group.height) / 2.0;
    assert!(
        padding >= 0.0,
        "native controls exceed the frame header height"
    );
    let origins = buttons
        .iter()
        .map(|button| {
            (
                header.left + padding + button.left - group.left,
                header.top + (header.height - button.height) / 2.0,
            )
        })
        .collect();
    (origins, (group.left - header.left - padding).abs())
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
        let (origins, left_error) = button_placement(header, &controls);
        assert_eq!(origins, [(31.0, 26.0), (51.0, 26.0), (71.0, 26.0)]);
        assert_eq!(left_error, 27.0);
        let placed: Vec<_> = origins
            .iter()
            .zip(controls)
            .map(|(&(left, top), button)| Rect {
                left,
                top,
                ..button
            })
            .collect();
        assert_eq!(button_placement(header, &placed), (origins, 0.0));
        assert_eq!(controls_reservation(header, union(&placed), 2.0), 44.75);
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
