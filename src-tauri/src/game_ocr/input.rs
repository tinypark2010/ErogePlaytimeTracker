use crate::models::ScreenshotOcrRegion;
use windows::Win32::Foundation::{POINT, RECT};

const LEFT: u8 = 1;
const RIGHT: u8 = 2;
const ESCAPE: u8 = 4;

#[derive(Clone, Copy)]
pub(super) enum Input {
    Move(POINT),
    LeftDown(POINT),
    LeftUp(POINT),
    RightDown(POINT),
    RightUp,
    EscapeDown,
    EscapeUp,
}

#[derive(Clone)]
pub(super) struct Selection {
    pub bounds: RECT,
    size: (u32, u32),
    start: Option<POINT>,
    end: POINT,
    pending: u8,
    pub closing: bool,
    pub result: Option<ScreenshotOcrRegion>,
}

impl Selection {
    pub fn new(bounds: RECT, size: (u32, u32)) -> Self {
        Self {
            bounds,
            size,
            start: None,
            end: POINT::default(),
            pending: 0,
            closing: false,
            result: None,
        }
    }

    pub fn cancel(&mut self) {
        self.closing = true;
        self.start = None;
        self.result = None;
    }

    pub fn drained(&self) -> bool {
        self.closing && self.pending == 0
    }

    fn contains(&self, p: POINT) -> bool {
        p.x >= self.bounds.left
            && p.x < self.bounds.right
            && p.y >= self.bounds.top
            && p.y < self.bounds.bottom
    }

    fn clamp(&self, p: POINT) -> POINT {
        POINT {
            x: p.x.clamp(self.bounds.left, self.bounds.right),
            y: p.y.clamp(self.bounds.top, self.bounds.bottom),
        }
    }

    // True means this event belongs to selection and must not reach the game.
    // Matching releases remain ours even after cancellation or focus loss.
    pub fn handle(&mut self, input: Input) -> bool {
        match input {
            Input::LeftUp(p) if self.pending & LEFT != 0 => {
                self.pending &= !LEFT;
                if !self.closing {
                    self.end = self.clamp(p);
                    self.result = self.region();
                    self.closing = self.result.is_some();
                    self.start = None;
                }
                true
            }
            Input::RightUp if self.pending & RIGHT != 0 => {
                self.pending &= !RIGHT;
                true
            }
            Input::EscapeUp if self.pending & ESCAPE != 0 => {
                self.pending &= !ESCAPE;
                true
            }
            Input::EscapeDown if !self.closing || self.pending & ESCAPE != 0 => {
                self.pending |= ESCAPE;
                self.cancel();
                true
            }
            Input::LeftDown(p) | Input::RightDown(p) if !self.closing => {
                if !self.contains(p) {
                    self.cancel();
                    return false;
                }
                if matches!(input, Input::LeftDown(_)) {
                    self.pending |= LEFT;
                    self.start = Some(p);
                    self.end = p;
                } else {
                    self.pending |= RIGHT;
                    self.cancel();
                }
                true
            }
            Input::Move(p) if !self.closing && self.start.is_some() => {
                self.end = self.clamp(p);
                // Let Windows move the real cursor; no mouse capture/focus is needed.
                false
            }
            _ => false,
        }
    }

    pub fn rect(&self) -> Option<RECT> {
        let start = self.start?;
        Some(RECT {
            left: start.x.min(self.end.x),
            top: start.y.min(self.end.y),
            right: start.x.max(self.end.x),
            bottom: start.y.max(self.end.y),
        })
    }

    fn region(&self) -> Option<ScreenshotOcrRegion> {
        let rect = self.rect()?;
        let (width, height) = (
            self.bounds.right - self.bounds.left,
            self.bounds.bottom - self.bounds.top,
        );
        if width <= 0
            || height <= 0
            || i64::from(rect.right - rect.left) * i64::from(self.size.0) < 2 * i64::from(width)
            || i64::from(rect.bottom - rect.top) * i64::from(self.size.1) < 2 * i64::from(height)
        {
            return None;
        }
        let x = f64::from(rect.left - self.bounds.left) / f64::from(width);
        let y = f64::from(rect.top - self.bounds.top) / f64::from(height);
        Some(ScreenshotOcrRegion {
            x,
            y,
            width: f64::from(rect.right - self.bounds.left) / f64::from(width) - x,
            height: f64::from(rect.bottom - self.bounds.top) / f64::from(height) - y,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn point(x: i32, y: i32) -> POINT {
        POINT { x, y }
    }
    fn selection() -> Selection {
        Selection::new(
            RECT {
                left: -1600,
                top: 100,
                right: 0,
                bottom: 1300,
            },
            (800, 600),
        )
    }

    #[test]
    fn maps_reverse_drag_and_scaled_negative_screen_coordinates_to_source() {
        let mut s = selection();
        assert!(s.handle(Input::LeftDown(point(-400, 1000))));
        assert!(!s.handle(Input::Move(point(-1200, 400))));
        assert_eq!(
            s.rect(),
            Some(RECT {
                left: -1200,
                top: 400,
                right: -400,
                bottom: 1000
            })
        );
        assert!(s.handle(Input::LeftUp(point(-1200, 400))));
        let r = s.result.unwrap();
        assert_eq!((r.x, r.y, r.width, r.height), (0.25, 0.25, 0.5, 0.5));
        assert!(s.drained());
    }

    #[test]
    fn clamps_drag_but_passes_outside_clicks_and_unowned_releases() {
        let mut s = selection();
        assert!(!s.handle(Input::LeftUp(point(-100, 200))));
        assert!(!s.handle(Input::EscapeUp));
        assert!(!s.handle(Input::RightUp));
        assert!(s.handle(Input::LeftDown(point(-1600, 100))));
        assert!(s.handle(Input::LeftUp(point(200, 1500))));
        let r = s.result.unwrap();
        assert_eq!((r.x, r.y, r.width, r.height), (0.0, 0.0, 1.0, 1.0));
        let mut s = selection();
        assert!(!s.handle(Input::LeftDown(point(10, 200))));
        assert!(s.drained());
        assert!(s.result.is_none());
    }

    #[test]
    fn clicks_and_less_than_two_source_pixels_allow_retry() {
        let mut s = selection();
        for end in [point(-1600, 100), point(-1597, 104), point(-1596, 103)] {
            assert!(s.handle(Input::LeftDown(point(-1600, 100))));
            assert!(s.handle(Input::LeftUp(end)));
            assert!(!s.closing);
            assert!(s.result.is_none());
        }
        s.handle(Input::LeftDown(point(-1600, 100)));
        s.handle(Input::LeftUp(point(-1596, 104)));
        assert!(s.drained());
        assert!(s.result.is_some());
    }

    #[test]
    fn escape_cancels_immediately_but_drains_repeat_and_all_owned_releases() {
        let mut s = selection();
        s.handle(Input::LeftDown(point(-1500, 200)));
        s.handle(Input::Move(point(-500, 800)));
        assert!(s.handle(Input::EscapeDown));
        assert!(s.closing);
        assert!(!s.drained());
        assert!(s.rect().is_none());
        assert!(s.handle(Input::EscapeDown));
        assert!(s.handle(Input::EscapeUp));
        assert!(!s.drained());
        assert!(s.handle(Input::LeftUp(point(-500, 800))));
        assert!(s.drained());
        assert!(s.result.is_none());
    }

    #[test]
    fn right_click_and_focus_loss_do_not_leak_releases_or_complete_a_draft() {
        let mut s = selection();
        assert!(s.handle(Input::RightDown(point(-1500, 200))));
        assert!(s.closing);
        assert!(!s.drained());
        assert!(s.handle(Input::RightUp));
        assert!(s.drained());
        let mut s = selection();
        s.handle(Input::LeftDown(point(-1500, 200)));
        s.cancel();
        assert!(s.handle(Input::LeftUp(point(-500, 800))));
        assert!(s.result.is_none());
        assert!(s.drained());
    }
}
