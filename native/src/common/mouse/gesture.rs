#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy)]
enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Default)]
pub(crate) struct Gesture {
    x: f64,
    y: f64,
    axis: Option<Axis>,
    dragged: bool,
    triggered: bool,
}

impl Gesture {
    pub fn movement(&mut self, x: f64, y: f64) -> Option<Direction> {
        if self.triggered || !x.is_finite() || !y.is_finite() {
            return None;
        }
        self.x = (self.x + x).clamp(-16_384.0, 16_384.0);
        self.y = (self.y + y).clamp(-16_384.0, 16_384.0);
        let horizontal = self.x.abs();
        let vertical = self.y.abs();
        self.dragged |= horizontal.hypot(vertical) > 12.0;
        const DOMINANCE: f64 = 1.8;
        if self.axis.is_none() {
            if horizontal >= 48.0 && horizontal >= vertical * DOMINANCE {
                self.axis = Some(Axis::Horizontal);
            } else if vertical >= 48.0 && vertical >= horizontal * DOMINANCE {
                self.axis = Some(Axis::Vertical);
            }
        }
        let direction = match self.axis {
            Some(Axis::Horizontal) if horizontal >= 96.0 && horizontal >= vertical * DOMINANCE => {
                if self.x < 0.0 {
                    Direction::Left
                } else {
                    Direction::Right
                }
            }
            Some(Axis::Vertical) if vertical >= 120.0 && vertical >= horizontal * DOMINANCE => {
                if self.y < 0.0 {
                    Direction::Up
                } else {
                    Direction::Down
                }
            }
            _ => return None,
        };
        self.triggered = true;
        Some(direction)
    }

    pub fn clicked(&self) -> bool {
        !self.dragged && !self.triggered
    }
}
