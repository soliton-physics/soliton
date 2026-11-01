/// Where the energy is. `conserved()` must stay constant to the integrator's accuracy.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EnergyLedger {
    pub kinetic: f64,
    pub potential: f64,
    /// Cumulative energy removed by drag, friction and inelastic impacts.
    pub dissipated: f64,
    /// Cumulative work done on the system by applied forces, torques and driving forces.
    pub external_work: f64,
}

impl EnergyLedger {
    pub fn mechanical(&self) -> f64 {
        self.kinetic + self.potential
    }

    pub fn conserved(&self) -> f64 {
        self.kinetic + self.potential + self.dissipated - self.external_work
    }
}
