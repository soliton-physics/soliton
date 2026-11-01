use crate::Invariant;

/// Anything that advances in fixed time steps and can report its conserved quantities.
pub trait Simulate {
    fn time(&self) -> f64;

    fn dt(&self) -> f64;

    fn step(&mut self);

    fn advance(&mut self, steps: usize) {
        for _ in 0..steps {
            self.step();
        }
    }

    fn invariants(&mut self) -> Vec<Invariant>;
}
