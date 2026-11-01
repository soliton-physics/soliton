use crate::equation::{Invariant, SemiLinear};
use crate::grid::PeriodicGrid;
use crate::spectral::SpectralSpace;
use crate::stepper::{Etdrk4, Stepper};
use num_complex::Complex64;

/// Drives a [`SemiLinear`] equation forward in time on a [`PeriodicGrid`].
pub struct Simulation<E: SemiLinear, S: Stepper = Etdrk4> {
    equation: E,
    stepper: S,
    space: SpectralSpace,
    spectrum: Vec<Complex64>,
    time: f64,
}

impl<E: SemiLinear> Simulation<E, Etdrk4> {
    /// Creates a simulation with the ETDRK4 stepper and a zero initial state.
    pub fn new(grid: PeriodicGrid, equation: E, dt: f64) -> Self {
        let space = SpectralSpace::new(grid);
        let stepper = Etdrk4::new(&equation, &space, dt);
        Self::with_stepper(grid, equation, stepper)
    }
}

impl<E: SemiLinear, S: Stepper> Simulation<E, S> {
    pub fn with_stepper(grid: PeriodicGrid, equation: E, stepper: S) -> Self {
        Self {
            equation,
            stepper,
            space: SpectralSpace::new(grid),
            spectrum: vec![Complex64::default(); grid.points()],
            time: 0.0,
        }
    }

    pub fn grid(&self) -> &PeriodicGrid {
        self.space.grid()
    }

    pub fn equation(&self) -> &E {
        &self.equation
    }

    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn dt(&self) -> f64 {
        self.stepper.dt()
    }

    pub fn spectrum(&self) -> &[Complex64] {
        &self.spectrum
    }

    /// Current state in physical space.
    pub fn state(&mut self) -> Vec<Complex64> {
        let mut field = self.spectrum.clone();
        self.space.to_physical(&mut field);
        field
    }

    /// Replaces the state with the given physical field.
    pub fn set_state(&mut self, field: &[Complex64]) {
        assert_eq!(
            field.len(),
            self.spectrum.len(),
            "field length must match the grid"
        );
        self.spectrum.copy_from_slice(field);
        self.space.to_spectral(&mut self.spectrum);
    }

    /// Replaces the state with `profile(x)` sampled on the grid.
    pub fn set_profile(&mut self, profile: impl Fn(f64) -> Complex64) {
        let field: Vec<Complex64> = self.grid().positions().into_iter().map(profile).collect();
        self.set_state(&field);
    }

    /// Adds `profile(x)` to the current state.
    pub fn add_profile(&mut self, profile: impl Fn(f64) -> Complex64) {
        let mut field = self.state();
        for (value, x) in field.iter_mut().zip(self.grid().positions()) {
            *value += profile(x);
        }
        self.set_state(&field);
    }

    pub fn reset_time(&mut self) {
        self.time = 0.0;
    }

    pub fn step(&mut self) {
        self.stepper
            .step(&self.equation, &mut self.space, &mut self.spectrum);
        self.time += self.stepper.dt();
    }

    pub fn advance(&mut self, steps: usize) {
        for _ in 0..steps {
            self.step();
        }
    }

    pub fn invariants(&mut self) -> Vec<Invariant> {
        self.equation.invariants(&mut self.space, &self.spectrum)
    }
}
