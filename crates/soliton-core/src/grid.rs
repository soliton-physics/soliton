use std::f64::consts::TAU;

/// Uniform periodic grid on `[-length/2, length/2)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeriodicGrid {
    points: usize,
    length: f64,
}

impl PeriodicGrid {
    pub fn new(points: usize, length: f64) -> Self {
        assert!(
            points >= 2 && points.is_multiple_of(2),
            "grid needs an even number of points >= 2"
        );
        assert!(length > 0.0, "grid length must be positive");
        Self { points, length }
    }

    pub fn points(&self) -> usize {
        self.points
    }

    pub fn length(&self) -> f64 {
        self.length
    }

    pub fn spacing(&self) -> f64 {
        self.length / self.points as f64
    }

    pub fn positions(&self) -> Vec<f64> {
        let dx = self.spacing();
        let origin = -0.5 * self.length;
        (0..self.points).map(|j| origin + j as f64 * dx).collect()
    }

    /// Angular wavenumbers in FFT order: `0, 1, ..., n/2-1, -n/2, ..., -1` times `2π/L`.
    pub fn wavenumbers(&self) -> Vec<f64> {
        let scale = TAU / self.length;
        let half = (self.points / 2) as i64;
        (0..self.points as i64)
            .map(|j| if j < half { j } else { j - self.points as i64 })
            .map(|m| m as f64 * scale)
            .collect()
    }
}
