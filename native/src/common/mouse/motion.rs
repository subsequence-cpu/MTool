use std::{collections::VecDeque, f64::consts::PI};

struct Impulse {
    pixels: f64,
    progress: f64,
}

fn position(progress: f64) -> f64 {
    let progress = progress.clamp(0.0, 1.0);
    const RAMP: f64 = 0.1;
    if progress < RAMP {
        progress - RAMP / PI * (PI * progress / RAMP).sin()
    } else {
        let tail = progress - RAMP;
        RAMP + tail + (1.0 - RAMP) / PI * (PI * tail / (1.0 - RAMP)).sin()
    }
}

#[derive(Default)]
struct Axis {
    impulses: VecDeque<Impulse>,
    fraction: f64,
}

impl Axis {
    fn push(&mut self, pixels: f64) {
        if !pixels.is_finite() || pixels == 0.0 {
            return;
        }
        if self
            .impulses
            .front()
            .is_some_and(|impulse| impulse.pixels * pixels < 0.0)
        {
            *self = Self::default();
        }
        let remaining: f64 = self
            .impulses
            .iter()
            .map(|impulse| impulse.pixels * (1.0 - position(impulse.progress)))
            .sum();
        let pixels = (remaining + pixels).clamp(-10_000.0, 10_000.0) - remaining;
        if self.impulses.len() >= 128 {
            let last = self.impulses.back_mut().unwrap();
            last.pixels = last.pixels * (1.0 - position(last.progress)) + pixels;
            last.progress = 0.0;
        } else {
            self.impulses.push_back(Impulse {
                pixels,
                progress: 0.0,
            });
        }
    }

    fn step(&mut self, progress: f64) -> i32 {
        for impulse in &mut self.impulses {
            let previous = position(impulse.progress);
            impulse.progress = (impulse.progress + progress).min(1.0);
            self.fraction += impulse.pixels * (position(impulse.progress) - previous);
        }
        self.impulses.retain(|impulse| impulse.progress < 1.0);
        let output = self.fraction.round();
        self.fraction -= output;
        output as i32
    }
}

#[derive(Default)]
pub(crate) struct Motion {
    vertical: Axis,
    horizontal: Axis,
}

impl Motion {
    pub fn push(&mut self, vertical: f64, horizontal: f64) {
        self.vertical.push(vertical);
        self.horizontal.push(horizontal);
    }

    pub fn step(&mut self, elapsed_ms: f64, duration_ms: u32) -> (i32, i32) {
        let progress = elapsed_ms.max(0.0) / f64::from(duration_ms.max(1));
        (self.vertical.step(progress), self.horizontal.step(progress))
    }

    pub fn active(&self) -> bool {
        !self.vertical.impulses.is_empty() || !self.horizontal.impulses.is_empty()
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
