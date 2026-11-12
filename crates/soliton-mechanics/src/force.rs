use crate::body::{Bodies, BodyId};
use crate::constraint::Sketch;
use soliton_core::Vec2;

/// What a force is, for the free-body diagram and the energy ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ForceLabel {
    Gravity,
    Applied,
    Torque,
    Spring,
    Drag,
    Driving,
    Tension,
    Normal,
    Friction,
    Reaction,
}

/// How a force enters the energy ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkClass {
    /// Has a potential; its work is already in the potential energy.
    Conservative,
    /// Removes mechanical energy; its work goes to `dissipated`.
    Dissipative,
    /// Adds or removes energy from outside; its work goes to `external_work`.
    External,
    /// Ideal constraint reaction; does no net work.
    Reaction,
}

impl ForceLabel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gravity => "gravity",
            Self::Applied => "applied",
            Self::Torque => "torque",
            Self::Spring => "spring",
            Self::Drag => "drag",
            Self::Driving => "driving",
            Self::Tension => "tension",
            Self::Normal => "normal",
            Self::Friction => "friction",
            Self::Reaction => "reaction",
        }
    }

    pub fn work_class(self) -> WorkClass {
        match self {
            Self::Gravity | Self::Spring => WorkClass::Conservative,
            Self::Drag | Self::Friction => WorkClass::Dissipative,
            Self::Applied | Self::Torque | Self::Driving => WorkClass::External,
            Self::Tension | Self::Normal | Self::Reaction => WorkClass::Reaction,
        }
    }
}

/// One entry of a free-body diagram: a force at a point, or a pure couple.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForceSample {
    pub body: BodyId,
    pub label: ForceLabel,
    pub force: Vec2,
    pub point: Vec2,
    pub torque: f64,
    /// An impact impulse spread over one step; its energy is accounted at the impact, not through power.
    pub impulsive: bool,
}

/// Accumulated force and torque per body for one evaluation, with the breakdown that produced them.
/// Explicit forces drive the integrator; constraint reactions are recorded separately because they are applied
/// as impulses and must not be kicked a second time.
#[derive(Clone, Debug, Default)]
pub struct Loads {
    pub(crate) forces: Vec<Vec2>,
    pub(crate) torques: Vec<f64>,
    pub(crate) reactions: Vec<Vec2>,
    pub(crate) reaction_torques: Vec<f64>,
    pub(crate) samples: Vec<ForceSample>,
}

impl Loads {
    pub(crate) fn resize(&mut self, bodies: usize) {
        self.forces.resize(bodies, Vec2::ZERO);
        self.torques.resize(bodies, 0.0);
        self.reactions.resize(bodies, Vec2::ZERO);
        self.reaction_torques.resize(bodies, 0.0);
    }

    pub(crate) fn clear(&mut self) {
        self.forces.iter_mut().for_each(|f| *f = Vec2::ZERO);
        self.torques.iter_mut().for_each(|t| *t = 0.0);
        self.reactions.iter_mut().for_each(|f| *f = Vec2::ZERO);
        self.reaction_torques.iter_mut().for_each(|t| *t = 0.0);
        self.samples.clear();
    }

    /// Records a constraint or contact reaction already applied as an impulse.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn react(
        &mut self,
        bodies: &Bodies,
        body: BodyId,
        label: ForceLabel,
        force: Vec2,
        angular: f64,
        point: Vec2,
        impulsive: bool,
    ) {
        let i = body.index();
        self.reactions[i] += force;
        self.reaction_torques[i] += angular;
        let torque = angular - (point - bodies.position(body)).cross(force);
        self.samples.push(ForceSample {
            body,
            label,
            force,
            point,
            torque,
            impulsive,
        });
    }

    /// Applies `force` at a world `point` on `body`.
    pub fn apply(
        &mut self,
        bodies: &Bodies,
        body: BodyId,
        label: ForceLabel,
        force: Vec2,
        point: Vec2,
    ) {
        let i = body.index();
        self.forces[i] += force;
        self.torques[i] += (point - bodies.position(body)).cross(force);
        self.samples.push(ForceSample {
            body,
            label,
            force,
            point,
            torque: 0.0,
            impulsive: false,
        });
    }

    pub fn apply_torque(&mut self, bodies: &Bodies, body: BodyId, label: ForceLabel, torque: f64) {
        self.torques[body.index()] += torque;
        self.samples.push(ForceSample {
            body,
            label,
            force: Vec2::ZERO,
            point: bodies.position(body),
            torque,
            impulsive: false,
        });
    }

    pub fn samples(&self) -> &[ForceSample] {
        &self.samples
    }
}

/// Which half-step a force evaluation at `time` serves. Verlet kicks twice per step, once with the loads at the
/// step's start and once with the loads at its end, so a force that switches at a step boundary must know whether
/// it is being asked about the half-step after the boundary or the one before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Half {
    /// The half-step starting at `time`.
    Opening,
    /// The half-step ending at `time`.
    Closing,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Instant {
    pub time: f64,
    pub half: Half,
}

impl Instant {
    pub fn opening(time: f64) -> Self {
        Self {
            time,
            half: Half::Opening,
        }
    }

    pub fn closing(time: f64) -> Self {
        Self {
            time,
            half: Half::Closing,
        }
    }

    /// Whether this half-step lies inside the window `[from, until)`.
    /// Exact for windows whose edges fall on step boundaries.
    pub fn within(self, from: f64, until: f64) -> bool {
        match self.half {
            Half::Opening => self.time >= from && self.time < until,
            Half::Closing => self.time > from && self.time <= until,
        }
    }
}

/// Anything that contributes force and torque to bodies given the current state and time.
pub trait Force {
    fn label(&self) -> ForceLabel;

    fn apply(&self, bodies: &Bodies, instant: Instant, loads: &mut Loads);

    fn depends_on_velocity(&self) -> bool {
        false
    }

    /// True when the force switches on or off at given times, so loads must be re-evaluated at step starts.
    fn has_time_window(&self) -> bool {
        false
    }

    fn potential_energy(&self, _bodies: &Bodies) -> f64 {
        0.0
    }

    fn sketches(&self, _bodies: &Bodies) -> Vec<Sketch> {
        Vec::new()
    }
}

/// Uniform gravity on every dynamic body. Added by [`crate::World::new`].
pub(crate) struct Gravity {
    pub(crate) g: Vec2,
}

impl Force for Gravity {
    fn label(&self) -> ForceLabel {
        ForceLabel::Gravity
    }

    fn apply(&self, bodies: &Bodies, _instant: Instant, loads: &mut Loads) {
        for id in bodies.ids().filter(|&id| bodies.is_dynamic(id)) {
            loads.apply(
                bodies,
                id,
                self.label(),
                self.g * bodies.mass(id),
                bodies.position(id),
            );
        }
    }

    /// Potential with datum at the origin: `-m g · x`.
    fn potential_energy(&self, bodies: &Bodies) -> f64 {
        bodies
            .ids()
            .filter(|&id| bodies.is_dynamic(id))
            .map(|id| -bodies.mass(id) * self.g.dot(bodies.position(id)))
            .sum()
    }
}

/// A constant force on one body, optionally at a point in the body frame and within a time window.
#[derive(Clone, Copy, Debug)]
pub struct Applied {
    body: BodyId,
    force: Vec2,
    local_point: Vec2,
    from: f64,
    until: f64,
}

impl Applied {
    pub fn new(body: BodyId, force: Vec2) -> Self {
        Self {
            body,
            force,
            local_point: Vec2::ZERO,
            from: f64::NEG_INFINITY,
            until: f64::INFINITY,
        }
    }

    pub fn at_local(mut self, point: Vec2) -> Self {
        self.local_point = point;
        self
    }

    pub fn from(mut self, time: f64) -> Self {
        self.from = time;
        self
    }

    pub fn until(mut self, time: f64) -> Self {
        self.until = time;
        self
    }
}

impl Force for Applied {
    fn label(&self) -> ForceLabel {
        ForceLabel::Applied
    }

    fn apply(&self, bodies: &Bodies, instant: Instant, loads: &mut Loads) {
        if instant.within(self.from, self.until) {
            let point = bodies.world_point(self.body, self.local_point);
            loads.apply(bodies, self.body, self.label(), self.force, point);
        }
    }

    fn has_time_window(&self) -> bool {
        self.from.is_finite() || self.until.is_finite()
    }
}

/// A constant couple on one body.
#[derive(Clone, Copy, Debug)]
pub struct Torque {
    body: BodyId,
    torque: f64,
}

impl Torque {
    pub fn new(body: BodyId, torque: f64) -> Self {
        Self { body, torque }
    }
}

impl Force for Torque {
    fn label(&self) -> ForceLabel {
        ForceLabel::Torque
    }

    fn apply(&self, bodies: &Bodies, _instant: Instant, loads: &mut Loads) {
        loads.apply_torque(bodies, self.body, self.label(), self.torque);
    }
}

/// Where a spring or rope ends: a point fixed in the world, or a point riding on a body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Anchor {
    Fixed(Vec2),
    Body { id: BodyId, local: Vec2 },
}

impl Anchor {
    pub fn body(id: BodyId) -> Self {
        Self::Body {
            id,
            local: Vec2::ZERO,
        }
    }

    pub fn world_point(self, bodies: &Bodies) -> Vec2 {
        match self {
            Self::Fixed(p) => p,
            Self::Body { id, local } => bodies.world_point(id, local),
        }
    }

    fn apply(self, bodies: &Bodies, label: ForceLabel, force: Vec2, loads: &mut Loads) {
        if let Self::Body { id, .. } = self {
            if bodies.is_dynamic(id) {
                loads.apply(bodies, id, label, force, self.world_point(bodies));
            }
        }
    }
}

/// Hooke's law between two anchors: `F = -k (ℓ - ℓ₀)` along the line joining them.
#[derive(Clone, Copy, Debug)]
pub struct Spring {
    a: Anchor,
    b: Anchor,
    stiffness: f64,
    rest_length: f64,
}

impl Spring {
    pub fn new(a: Anchor, b: Anchor, stiffness: f64, rest_length: f64) -> Self {
        Self {
            a,
            b,
            stiffness,
            rest_length,
        }
    }

    fn extension(&self, bodies: &Bodies) -> (Vec2, f64) {
        let separation = self.b.world_point(bodies) - self.a.world_point(bodies);
        let length = separation.length();
        (separation.normalized(), length - self.rest_length)
    }
}

impl Force for Spring {
    fn label(&self) -> ForceLabel {
        ForceLabel::Spring
    }

    fn apply(&self, bodies: &Bodies, _instant: Instant, loads: &mut Loads) {
        let (direction, extension) = self.extension(bodies);
        let on_b = direction * (-self.stiffness * extension);
        self.a.apply(bodies, self.label(), -on_b, loads);
        self.b.apply(bodies, self.label(), on_b, loads);
    }

    fn potential_energy(&self, bodies: &Bodies) -> f64 {
        let (_, extension) = self.extension(bodies);
        0.5 * self.stiffness * extension * extension
    }

    fn sketches(&self, bodies: &Bodies) -> Vec<Sketch> {
        vec![Sketch::Spring(
            self.a.world_point(bodies),
            self.b.world_point(bodies),
        )]
    }
}

/// `F = -b v`.
#[derive(Clone, Copy, Debug)]
pub struct LinearDrag {
    body: BodyId,
    coefficient: f64,
}

impl LinearDrag {
    pub fn new(body: BodyId, coefficient: f64) -> Self {
        Self { body, coefficient }
    }
}

impl Force for LinearDrag {
    fn label(&self) -> ForceLabel {
        ForceLabel::Drag
    }

    fn apply(&self, bodies: &Bodies, _instant: Instant, loads: &mut Loads) {
        let force = bodies.velocity(self.body) * (-self.coefficient);
        loads.apply(
            bodies,
            self.body,
            self.label(),
            force,
            bodies.position(self.body),
        );
    }

    fn depends_on_velocity(&self) -> bool {
        true
    }
}

/// `F = -c |v| v`.
#[derive(Clone, Copy, Debug)]
pub struct QuadraticDrag {
    body: BodyId,
    coefficient: f64,
}

impl QuadraticDrag {
    pub fn new(body: BodyId, coefficient: f64) -> Self {
        Self { body, coefficient }
    }
}

impl Force for QuadraticDrag {
    fn label(&self) -> ForceLabel {
        ForceLabel::Drag
    }

    fn apply(&self, bodies: &Bodies, _instant: Instant, loads: &mut Loads) {
        let v = bodies.velocity(self.body);
        loads.apply(
            bodies,
            self.body,
            self.label(),
            v * (-self.coefficient * v.length()),
            bodies.position(self.body),
        );
    }

    fn depends_on_velocity(&self) -> bool {
        true
    }
}

/// `F = F₀ cos(ω t + φ)` on one body.
#[derive(Clone, Copy, Debug)]
pub struct Driving {
    body: BodyId,
    amplitude: Vec2,
    angular_frequency: f64,
    phase: f64,
}

impl Driving {
    pub fn new(body: BodyId, amplitude: Vec2, angular_frequency: f64) -> Self {
        Self {
            body,
            amplitude,
            angular_frequency,
            phase: 0.0,
        }
    }

    pub fn phase(mut self, phase: f64) -> Self {
        self.phase = phase;
        self
    }
}

impl Force for Driving {
    fn label(&self) -> ForceLabel {
        ForceLabel::Driving
    }

    fn apply(&self, bodies: &Bodies, instant: Instant, loads: &mut Loads) {
        let force = self.amplitude * (self.angular_frequency * instant.time + self.phase).cos();
        loads.apply(
            bodies,
            self.body,
            self.label(),
            force,
            bodies.position(self.body),
        );
    }
}

/// A hand on a body (design 008): a critically damped spring from a point of the body to a target that follows the
/// pointer. It is an ordinary external force, so it appears in the free-body diagram as `applied` and the energy
/// ledger books what it does as external work.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grab {
    pub body: BodyId,
    /// The held point in the body's frame.
    pub local: Vec2,
    pub target: Vec2,
}

/// Natural frequency of the hand's spring, rad/s, at ordinary time steps: stiff enough to follow the pointer closely.
pub(crate) const GRAB_FREQUENCY: f64 = 30.0;
/// The largest `ω dt` the spring is allowed, so a scene stepped in seconds (an orbit) gets a correspondingly
/// slower hand instead of an unstable one.
pub(crate) const GRAB_STEP_LIMIT: f64 = 0.5;
/// The most the hand can accelerate a body, m/s²: 1.5 m of stretch at the ordinary frequency. A pointer flung
/// across the screen cannot launch a body at any speed.
const GRAB_ACCELERATION: f64 = 1.5 * GRAB_FREQUENCY * GRAB_FREQUENCY;

impl Grab {
    pub(crate) fn apply(&self, bodies: &Bodies, loads: &mut Loads, frequency: f64) {
        let mass = bodies.mass(self.body);
        let point = bodies.world_point(self.body, self.local);
        let mut stretch = self.target - point;
        let length = stretch.length();
        let reach = GRAB_ACCELERATION / (frequency * frequency);
        if length > reach {
            stretch = stretch * (reach / length);
        }
        let stiffness = mass * frequency * frequency;
        let damping = 2.0 * mass * frequency;
        let force = stretch * stiffness - bodies.point_velocity(self.body, point) * damping;
        loads.apply(bodies, self.body, ForceLabel::Applied, force, point);
    }
}

/// Newton's law of gravitation, `F = G m₁ m₂ / r²` along the line joining the two (design 009). Every pair of
/// dynamic bodies attracts, and so does each fixed source, a mass at a point that is not itself moved: the planet
/// a satellite goes round. It reports as `gravity` and has a potential, `−G m₁ m₂ / r`.
#[derive(Clone, Debug)]
pub struct Gravitation {
    constant: f64,
    sources: Vec<(Vec2, f64)>,
}

impl Gravitation {
    pub fn new(constant: f64) -> Self {
        Self {
            constant,
            sources: Vec::new(),
        }
    }

    /// Adds a fixed mass at a point.
    pub fn source(mut self, at: Vec2, mass: f64) -> Self {
        self.sources.push((at, mass));
        self
    }

    fn movers(bodies: &Bodies) -> impl Iterator<Item = BodyId> + '_ {
        bodies.ids().filter(|&id| bodies.is_dynamic(id))
    }
}

/// The attraction on a mass at `at` from a mass at `from`, per unit of the product `G m₁ m₂`.
/// Coincident points attract with nothing rather than with infinity.
fn inverse_square(at: Vec2, from: Vec2) -> Vec2 {
    let separation = from - at;
    let distance = separation.length();
    if distance == 0.0 {
        Vec2::ZERO
    } else {
        separation / (distance * distance * distance)
    }
}

impl Force for Gravitation {
    fn label(&self) -> ForceLabel {
        ForceLabel::Gravity
    }

    fn apply(&self, bodies: &Bodies, _instant: Instant, loads: &mut Loads) {
        for id in Self::movers(bodies) {
            let (at, mass) = (bodies.position(id), bodies.mass(id));
            let mut pull = Vec2::ZERO;
            for &(from, source_mass) in &self.sources {
                pull += inverse_square(at, from) * source_mass;
            }
            for other in Self::movers(bodies).filter(|&other| other != id) {
                pull += inverse_square(at, bodies.position(other)) * bodies.mass(other);
            }
            loads.apply(bodies, id, self.label(), pull * (self.constant * mass), at);
        }
    }

    fn potential_energy(&self, bodies: &Bodies) -> f64 {
        let mut energy = 0.0;
        let movers: Vec<BodyId> = Self::movers(bodies).collect();
        for (k, &id) in movers.iter().enumerate() {
            let (at, mass) = (bodies.position(id), bodies.mass(id));
            for &(from, source_mass) in &self.sources {
                energy -= mass * source_mass / (from - at).length();
            }
            for &other in &movers[k + 1..] {
                energy -= mass * bodies.mass(other) / (bodies.position(other) - at).length();
            }
        }
        self.constant * energy
    }
}
