use crate::models::{BarEdge, MediaSession, Rect, SystemTheme, WindowInfo};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

#[cfg(not(windows))]
mod stub;
#[cfg(not(windows))]
pub use stub::*;

/// Shared helper to test if a point falls inside any interactive widget/bar rectangle.
#[allow(dead_code)]
pub fn point_in_rects(px: f64, py: f64, rects: &[Rect]) -> bool {
    rects.iter().any(|r| r.contains(px, py))
}

#[allow(dead_code)]
pub fn _type_assertions(
    _edge: BarEdge,
    _session: Option<MediaSession>,
    _theme: SystemTheme,
    _wins: Vec<WindowInfo>,
) {
}
