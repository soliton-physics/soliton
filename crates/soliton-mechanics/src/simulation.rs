use crate::energy::EnergyLedger;
use crate::event::Event;
use crate::finish::{Finish, FinishWatch, Finished};
use crate::force::Instant;
use crate::integrator::{Integrator, Verlet};
use crate::record::Recorder;
use crate::world::{Power, World};
use soliton_core::{Invariant, Simulate, Vec2};

/// Drives a [`World`] forward in fixed steps and keeps the energy ledger.
pub struct Simulation {
    world: World,
    integrator: Box<dyn Integrator>,
    dt: f64,
    time: f64,
    steps: u64,
    dissipated: f64,
    external_work: f64,
    previous_power: Power,
    events: Vec<Event>,
    finish: FinishWatch,
    finished: Option<Finished>,
}

impl Simulation {
    pub fn new(world: World, dt: f64) -> Self {
        Self::with_integrator(world, Verlet, dt)
    }

    pub fn with_integrator(
        mut world: World,
        integrator: impl Integrator + 'static,
        dt: f64,
    ) -> Self {
        assert!(dt > 0.0, "time step must be positive");
        world.evaluate_loads(Instant::opening(0.0));
        world.initialize_reactions(dt);
        let previous_power = world.power();
        Self {
            world,
            integrator: Box::new(integrator),
            dt,
            time: 0.0,
            steps: 0,
            dissipated: 0.0,
            external_work: 0.0,
            previous_power,
            events: Vec::new(),
            finish: FinishWatch::default(),
            finished: None,
        }
    }

    /// Installs the rules that mark the end of the problem; the first to fire wins.
    pub fn finish_when(&mut self, rules: Vec<Finish>) {
        self.finish = FinishWatch::new(rules);
        self.finished = None;
    }

    pub fn finish_rules(&self) -> &[Finish] {
        self.finish.rules()
    }

    /// Set once the problem has ended; stepping past it is allowed. If everything later comes to rest, this
    /// changes once more to say so.
    pub fn finished(&self) -> Option<Finished> {
        self.finished
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    /// The world, for taking hold of a body between steps (design 008).
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn steps(&self) -> u64 {
        self.steps
    }

    pub fn energy(&self) -> EnergyLedger {
        EnergyLedger {
            kinetic: self.world.kinetic_energy(),
            potential: self.world.potential_energy(),
            dissipated: self.dissipated,
            external_work: self.external_work,
        }
    }

    pub fn momentum(&self) -> Vec2 {
        self.world.momentum()
    }

    pub fn angular_momentum(&self) -> f64 {
        self.world.angular_momentum()
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Steps until `time >= end`, handing every `recorder.every`-th state to the recorder, including the initial one.
    pub fn run_until(&mut self, end: f64, recorder: &mut Recorder) {
        if self.steps == 0 && recorder.is_empty() {
            recorder.observe(self);
        }
        while self.time < end - 0.5 * self.dt {
            self.step();
            if self.steps.is_multiple_of(recorder.interval()) {
                recorder.observe(self);
            }
        }
    }
}

impl Simulate for Simulation {
    fn time(&self) -> f64 {
        self.time
    }

    fn dt(&self) -> f64 {
        self.dt
    }

    fn step(&mut self) {
        if self.world.needs_fresh_loads() {
            self.world.evaluate_loads(Instant::opening(self.time));
            self.previous_power = self.world.power();
        }
        self.integrator.step(&mut self.world, self.time, self.dt);
        self.steps += 1;
        self.time = self.steps as f64 * self.dt;
        let first_new = self.events.len();
        self.events.append(&mut self.world.events);
        if let Some(finished) =
            self.finish
                .observe(&self.world, &self.events[first_new..], self.time)
        {
            self.finished = Some(finished);
        }
        self.dissipated += std::mem::take(&mut self.world.impact_loss);

        let power = self.world.power();
        self.dissipated += 0.5 * (self.previous_power.dissipative + power.dissipative) * self.dt;
        self.external_work += 0.5 * (self.previous_power.external + power.external) * self.dt;
        self.previous_power = power;
    }

    fn invariants(&mut self) -> Vec<Invariant> {
        let momentum = self.momentum();
        vec![
            Invariant {
                name: "energy",
                value: self.energy().conserved(),
            },
            Invariant {
                name: "momentum_x",
                value: momentum.x,
            },
            Invariant {
                name: "momentum_y",
                value: momentum.y,
            },
            Invariant {
                name: "angular_momentum",
                value: self.angular_momentum(),
            },
        ]
    }
}
