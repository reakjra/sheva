use std::f32::consts::PI;

const TAPS: usize = 128;

pub struct Decimator {
    factor: usize,
    taps: [f32; TAPS],
    history: [[f32; 2]; TAPS],
    head: usize,
    phase: usize,
}

impl Decimator {
    pub fn new(input_rate: u32, output_rate: u32) -> Self {
        let factor = (input_rate / output_rate) as usize;
        let cutoff = 0.45 / factor as f32;
        let center = (TAPS - 1) as f32 / 2.0;

        let mut taps = [0.0; TAPS];
        for (n, tap) in taps.iter_mut().enumerate() {
            let x = n as f32 - center;
            let sinc = if x == 0.0 {
                2.0 * cutoff
            } else {
                (2.0 * PI * cutoff * x).sin() / (PI * x)
            };
            let blackman = 0.42 - 0.5 * (2.0 * PI * n as f32 / (TAPS - 1) as f32).cos()
                + 0.08 * (4.0 * PI * n as f32 / (TAPS - 1) as f32).cos();
            *tap = sinc * blackman;
        }
        let gain: f32 = taps.iter().sum();
        taps.iter_mut().for_each(|tap| *tap /= gain);

        Self {
            factor,
            taps,
            history: [[0.0; 2]; TAPS],
            head: 0,
            phase: 0,
        }
    }

    pub fn push(&mut self, frame: [f32; 2]) -> Option<[f32; 2]> {
        self.history[self.head] = frame;
        self.head = (self.head + 1) % TAPS;
        self.phase += 1;
        if self.phase < self.factor {
            return None;
        }
        self.phase = 0;

        let mut out = [0.0; 2];
        for (i, tap) in self.taps.iter().enumerate() {
            let [left, right] = self.history[(self.head + i) % TAPS];
            out[0] += left * tap;
            out[1] += right * tap;
        }
        Some(out)
    }
}
