use num_complex::Complex64;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

/// In-place complex FFT pair with the inverse normalised by `1/n`.
pub struct Fourier {
    forward: Arc<dyn Fft<f64>>,
    inverse: Arc<dyn Fft<f64>>,
    scratch: Vec<Complex64>,
    len: usize,
}

impl Fourier {
    pub fn new(len: usize) -> Self {
        let mut planner = FftPlanner::new();
        let forward = planner.plan_fft_forward(len);
        let inverse = planner.plan_fft_inverse(len);
        let scratch_len = forward
            .get_inplace_scratch_len()
            .max(inverse.get_inplace_scratch_len());
        Self {
            forward,
            inverse,
            scratch: vec![Complex64::default(); scratch_len],
            len,
        }
    }

    pub fn forward(&mut self, data: &mut [Complex64]) {
        self.forward.process_with_scratch(data, &mut self.scratch);
    }

    pub fn inverse(&mut self, data: &mut [Complex64]) {
        self.inverse.process_with_scratch(data, &mut self.scratch);
        let norm = 1.0 / self.len as f64;
        data.iter_mut().for_each(|z| *z *= norm);
    }
}
