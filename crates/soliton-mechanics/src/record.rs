use crate::body::BodyId;
use crate::energy::EnergyLedger;
use crate::force::ForceLabel;
use crate::simulation::Simulation;
use soliton_core::{Simulate, Vec2};

/// Time series of one body, one `Vec` per quantity.
#[derive(Clone, Debug, Default)]
pub struct BodyRecord {
    pub position: Vec<Vec2>,
    pub angle: Vec<f64>,
    pub velocity: Vec<Vec2>,
    pub angular_velocity: Vec<f64>,
    pub acceleration: Vec<Vec2>,
    pub angular_acceleration: Vec<f64>,
}

#[derive(Clone, Debug, Default)]
struct ForceTable {
    sample: Vec<u32>,
    body: Vec<u32>,
    label: Vec<ForceLabel>,
    force: Vec<Vec2>,
}

/// Samples a simulation every `n` steps into columnar storage.
#[derive(Clone, Debug)]
pub struct Recorder {
    every: u64,
    pub time: Vec<f64>,
    pub energy: Vec<EnergyLedger>,
    pub momentum: Vec<Vec2>,
    pub angular_momentum: Vec<f64>,
    bodies: Vec<BodyRecord>,
    forces: ForceTable,
}

impl Recorder {
    pub fn every(steps: u64) -> Self {
        assert!(steps > 0, "record interval must be at least one step");
        Self {
            every: steps,
            time: Vec::new(),
            energy: Vec::new(),
            momentum: Vec::new(),
            angular_momentum: Vec::new(),
            bodies: Vec::new(),
            forces: ForceTable::default(),
        }
    }

    pub(crate) fn interval(&self) -> u64 {
        self.every
    }

    /// Steps between samples.
    pub fn interval_public(&self) -> u64 {
        self.every
    }

    pub fn samples(&self) -> usize {
        self.time.len()
    }

    pub fn is_empty(&self) -> bool {
        self.time.is_empty()
    }

    pub fn body(&self, id: BodyId) -> &BodyRecord {
        &self.bodies[id.index()]
    }

    /// The net force with `label` on `id` at each sample.
    pub fn force(&self, id: BodyId, label: ForceLabel) -> Vec<Vec2> {
        let mut out = vec![Vec2::ZERO; self.samples()];
        for k in 0..self.forces.sample.len() {
            if self.forces.body[k] == id.0 && self.forces.label[k] == label {
                out[self.forces.sample[k] as usize] += self.forces.force[k];
            }
        }
        out
    }

    /// The same by the name a force goes by in a free-body diagram, which may cover more than one label:
    /// `electric` is the force from charges and from fields alike. `None` for a name no force has.
    pub fn force_named(&self, id: BodyId, name: &str) -> Option<Vec<Vec2>> {
        let labels: Vec<ForceLabel> = ForceLabel::ALL
            .into_iter()
            .filter(|l| l.as_str() == name)
            .collect();
        if labels.is_empty() {
            return None;
        }
        let mut out = vec![Vec2::ZERO; self.samples()];
        for label in labels {
            for (sum, force) in out.iter_mut().zip(self.force(id, label)) {
                *sum += force;
            }
        }
        Some(out)
    }

    pub fn observe(&mut self, sim: &Simulation) {
        let world = sim.world();
        let bodies = world.bodies();
        if self.bodies.len() < bodies.len() {
            self.bodies.resize_with(bodies.len(), BodyRecord::default);
        }
        let sample = self.time.len() as u32;
        self.time.push(sim.time());
        self.energy.push(sim.energy());
        self.momentum.push(sim.momentum());
        self.angular_momentum.push(sim.angular_momentum());
        for id in bodies.ids() {
            let record = &mut self.bodies[id.index()];
            record.position.push(bodies.position(id));
            record.angle.push(bodies.angle(id));
            record.velocity.push(bodies.velocity(id));
            record.angular_velocity.push(bodies.angular_velocity(id));
            record.acceleration.push(world.acceleration(id));
            record
                .angular_acceleration
                .push(world.angular_acceleration(id));
        }
        for s in world.force_samples() {
            self.forces.sample.push(sample);
            self.forces.body.push(s.body.0);
            self.forces.label.push(s.label);
            self.forces.force.push(s.force);
        }
    }
}
