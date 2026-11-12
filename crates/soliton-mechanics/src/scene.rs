//! Scenes as data (design 004): a versioned JSON description of a world that the playground, the notebook and the
//! scenario catalogue all share. Building a scene validates it and produces the live `World`.

use crate::body::{Body, BodyId};
use crate::constraint::{FixedAngle, PinJoint, Rod, Slot};
use crate::finish::{Finish, Trigger};
use crate::force::{
    Anchor, Applied, Driving, Gravitation, LinearDrag, QuadraticDrag, Spring, Torque,
};
use crate::rope::{Rope, RopeNode, Wrap};
use crate::scenario::{Scene, View};
use crate::shape::{Circle, HalfPlane, Polygon, Shape};
use crate::world::World;
use serde::{Deserialize, Serialize};
use soliton_core::Vec2;
use std::collections::HashMap;
use std::fmt;

pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct SceneError(pub String);

impl fmt::Display for SceneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SceneError {}

fn err<T>(message: impl Into<String>) -> Result<T, SceneError> {
    Err(SceneError(message.into()))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneFile {
    pub schema: u32,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub syllabus: Option<SyllabusSpec>,
    pub world: WorldSpec,
    pub bodies: Vec<BodySpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forces: Vec<ForceSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<ConstraintSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignore_collisions: Vec<[String; 2]>,
    /// The body the free-body diagram and graphs show by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<String>,
    /// Textbook answers for this scene's parameters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub answers: Vec<Answer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<View>,
    /// Where the problem ends; the first rule to fire wins. Front ends pause there.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub finish: Vec<FinishSpec>,
    /// Seconds of the simulation a front end should show per second of watching, when real time would be far
    /// too slow: an orbit takes ninety minutes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playback: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinishSpec {
    pub when: TriggerSpec,
    /// Seconds to keep going after the trigger, so a collision can be watched to separation.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub hold: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerSpec {
    Contact([String; 2]),
    Separation([String; 2]),
    /// Every moving body still for this many seconds.
    Rest(f64),
    Time(f64),
}

impl FinishSpec {
    pub fn contact(a: &str, b: &str) -> Self {
        Self::new(TriggerSpec::Contact([a.to_string(), b.to_string()]))
    }

    pub fn separation(a: &str, b: &str) -> Self {
        Self::new(TriggerSpec::Separation([a.to_string(), b.to_string()]))
    }

    pub fn rest(seconds: f64) -> Self {
        Self::new(TriggerSpec::Rest(seconds))
    }

    pub fn time(seconds: f64) -> Self {
        Self::new(TriggerSpec::Time(seconds))
    }

    fn new(when: TriggerSpec) -> Self {
        Self { when, hold: 0.0 }
    }

    pub fn hold(mut self, seconds: f64) -> Self {
        self.hold = seconds;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SyllabusSpec {
    pub class: u8,
    pub chapter: String,
    pub topic: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldSpec {
    pub gravity: [f64; 2],
    pub dt: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Answer {
    pub name: String,
    pub value: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KindSpec {
    Particle,
    Rigid,
    Static,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShapeSpec {
    Circle(f64),
    Rectangle([f64; 2]),
    Polygon(Vec<[f64; 2]>),
    Halfplane { normal: [f64; 2], offset: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodySpec {
    pub id: String,
    pub kind: KindSpec,
    pub shape: ShapeSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mass: Option<f64>,
    #[serde(default, skip_serializing_if = "is_zero2")]
    pub position: [f64; 2],
    #[serde(default, skip_serializing_if = "is_zero")]
    pub angle: f64,
    #[serde(default, skip_serializing_if = "is_zero2")]
    pub velocity: [f64; 2],
    #[serde(default, skip_serializing_if = "is_zero")]
    pub angular_velocity: f64,
    /// `[static, kinetic]` coefficients.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub friction: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restitution: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inertia: Option<f64>,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

fn is_false(v: &bool) -> bool {
    !*v
}

fn is_zero2(v: &[f64; 2]) -> bool {
    *v == [0.0, 0.0]
}

impl BodySpec {
    pub fn particle(id: &str, mass: f64) -> Self {
        Self::new(id, KindSpec::Particle, ShapeSpec::Circle(0.0)).mass(mass)
    }

    pub fn rigid(id: &str, mass: f64, shape: ShapeSpec) -> Self {
        Self::new(id, KindSpec::Rigid, shape).mass(mass)
    }

    pub fn fixed(id: &str, shape: ShapeSpec) -> Self {
        Self::new(id, KindSpec::Static, shape)
    }

    fn new(id: &str, kind: KindSpec, shape: ShapeSpec) -> Self {
        Self {
            id: id.to_string(),
            kind,
            shape,
            mass: None,
            position: [0.0; 2],
            angle: 0.0,
            velocity: [0.0; 2],
            angular_velocity: 0.0,
            friction: None,
            restitution: None,
            inertia: None,
        }
    }

    pub fn mass(mut self, mass: f64) -> Self {
        self.mass = Some(mass);
        self
    }

    pub fn shape(mut self, shape: ShapeSpec) -> Self {
        self.shape = shape;
        self
    }

    pub fn at(mut self, x: f64, y: f64) -> Self {
        self.position = [x, y];
        self
    }

    pub fn angle(mut self, angle: f64) -> Self {
        self.angle = angle;
        self
    }

    pub fn velocity(mut self, x: f64, y: f64) -> Self {
        self.velocity = [x, y];
        self
    }

    pub fn friction(mut self, static_friction: f64, kinetic_friction: f64) -> Self {
        self.friction = Some([static_friction, kinetic_friction]);
        self
    }

    pub fn restitution(mut self, restitution: f64) -> Self {
        self.restitution = Some(restitution);
        self
    }

    pub fn inertia(mut self, inertia: f64) -> Self {
        self.inertia = Some(inertia);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnchorSpec {
    Fixed([f64; 2]),
    Body {
        id: String,
        #[serde(default, skip_serializing_if = "is_zero2")]
        local: [f64; 2],
    },
}

impl AnchorSpec {
    pub fn body(id: &str) -> Self {
        Self::Body {
            id: id.to_string(),
            local: [0.0; 2],
        }
    }

    pub fn body_at(id: &str, local: [f64; 2]) -> Self {
        Self::Body {
            id: id.to_string(),
            local,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForceSpec {
    Applied {
        body: String,
        force: [f64; 2],
        #[serde(default, skip_serializing_if = "is_zero2")]
        point: [f64; 2],
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        until: Option<f64>,
    },
    Torque {
        body: String,
        torque: f64,
    },
    Spring {
        a: AnchorSpec,
        b: AnchorSpec,
        stiffness: f64,
        /// Defaults to the distance between the anchors at the start.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rest_length: Option<f64>,
        /// A spring with a free end: it pushes when squeezed and does nothing when stretched.
        #[serde(default, skip_serializing_if = "is_false")]
        pushes_only: bool,
    },
    LinearDrag {
        body: String,
        coefficient: f64,
    },
    QuadraticDrag {
        body: String,
        coefficient: f64,
    },
    Driving {
        body: String,
        amplitude: [f64; 2],
        angular_frequency: f64,
        #[serde(default, skip_serializing_if = "is_zero")]
        phase: f64,
    },
    /// Newton's gravitation between every pair of moving bodies, and from each fixed mass.
    Gravitation {
        constant: f64,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        fixed: Vec<SourceSpec>,
    },
}

/// A mass at a point that attracts but is not moved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceSpec {
    pub at: [f64; 2],
    pub mass: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WrapSpec {
    Clockwise,
    CounterClockwise,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RopeNodeSpec {
    Attach {
        body: String,
        #[serde(default, skip_serializing_if = "is_zero2")]
        local: [f64; 2],
    },
    Peg([f64; 2]),
    Pulley {
        center: [f64; 2],
        radius: f64,
        wrap: WrapSpec,
    },
    /// A rigid body the rope wraps without slipping; its rotation follows the rope.
    Wheel {
        body: String,
        radius: f64,
        wrap: WrapSpec,
    },
}

impl RopeNodeSpec {
    pub fn attach(body: &str) -> Self {
        Self::Attach {
            body: body.to_string(),
            local: [0.0; 2],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintSpec {
    Pin {
        a: AnchorSpec,
        b: AnchorSpec,
    },
    Rod {
        a: AnchorSpec,
        b: AnchorSpec,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        length: Option<f64>,
    },
    Slot {
        body: String,
        #[serde(default, skip_serializing_if = "is_zero2")]
        local: [f64; 2],
        origin: [f64; 2],
        direction: [f64; 2],
    },
    FixedAngle {
        body: String,
        angle: f64,
    },
    Rope {
        nodes: Vec<RopeNodeSpec>,
        /// Defaults to the path length at the start: a taut rope.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        length: Option<f64>,
    },
}

fn v(p: [f64; 2]) -> Vec2 {
    Vec2::new(p[0], p[1])
}

impl SceneFile {
    pub fn new(title: &str, gravity: [f64; 2], dt: f64) -> Self {
        Self {
            schema: SCHEMA,
            title: title.to_string(),
            description: None,
            syllabus: None,
            world: WorldSpec { gravity, dt },
            bodies: Vec::new(),
            forces: Vec::new(),
            constraints: Vec::new(),
            ignore_collisions: Vec::new(),
            focus: None,
            answers: Vec::new(),
            view: None,
            finish: Vec::new(),
            playback: None,
        }
    }

    pub fn from_json(text: &str) -> Result<Self, SceneError> {
        let scene: Self = serde_json::from_str(text)
            .map_err(|e| SceneError(format!("invalid scene JSON: {e}")))?;
        if scene.schema != SCHEMA {
            return err(format!(
                "unsupported scene schema {} (this engine reads {SCHEMA})",
                scene.schema
            ));
        }
        Ok(scene)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("scene serialises")
    }

    pub fn body(mut self, body: BodySpec) -> Self {
        self.bodies.push(body);
        self
    }

    pub fn force(mut self, force: ForceSpec) -> Self {
        self.forces.push(force);
        self
    }

    pub fn constraint(mut self, constraint: ConstraintSpec) -> Self {
        self.constraints.push(constraint);
        self
    }

    pub fn focus(mut self, id: &str) -> Self {
        self.focus = Some(id.to_string());
        self
    }

    pub fn answer(mut self, name: &str, value: f64) -> Self {
        self.answers.push(Answer {
            name: name.to_string(),
            value,
        });
        self
    }

    pub fn view(mut self, center: [f64; 2], half_height: f64) -> Self {
        self.view = Some(View {
            center,
            half_height,
            half_width: None,
        });
        self
    }

    /// A view that must fit a box, not just a height.
    pub fn frame(mut self, center: [f64; 2], half_width: f64, half_height: f64) -> Self {
        self.view = Some(View {
            center,
            half_height,
            half_width: Some(half_width),
        });
        self
    }

    pub fn playback(mut self, rate: f64) -> Self {
        self.playback = Some(rate);
        self
    }

    pub fn finish(mut self, rule: FinishSpec) -> Self {
        self.finish.push(rule);
        self
    }

    pub fn describe(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn syllabus(mut self, class: u8, chapter: &str, topic: &str) -> Self {
        self.syllabus = Some(SyllabusSpec {
            class,
            chapter: chapter.to_string(),
            topic: topic.to_string(),
        });
        self
    }

    /// Validates the description and builds the live world.
    pub fn build(&self) -> Result<Scene, SceneError> {
        self.build_explicit().map(|(scene, _)| scene)
    }

    /// The scene as it stands part-way through a run: the same world with every body where `world` has it now,
    /// and with everything that was measured at the start (a taut rope's length, a spring's rest length) written
    /// out, so the lengths do not change with the positions. Times are counted from `time`: force windows and
    /// time rules shift, and those already past are dropped. What is not carried over is the solver's memory of
    /// which contacts were sliding, so the first step may differ in the last digits.
    pub fn snapshot(&self, world: &World, time: f64) -> Result<SceneFile, SceneError> {
        let (_, mut file) = self.build_explicit()?;
        let bodies = world.bodies();
        for (index, spec) in file.constraints.iter_mut().enumerate() {
            if let ConstraintSpec::Rope { length, .. } = spec {
                if let Some(now) = world.constraint(index).length_from_here(bodies) {
                    *length = Some(now);
                }
            }
        }
        if bodies.len() != file.bodies.len() {
            return err("the state does not belong to this scene");
        }
        for (spec, id) in file.bodies.iter_mut().zip(bodies.ids()) {
            if !bodies.is_dynamic(id) {
                continue;
            }
            let (position, angle) = (bodies.position(id), bodies.angle(id));
            let origin = position - centroid_offset(&spec.shape).rotated(angle);
            spec.position = [origin.x, origin.y];
            spec.angle = angle;
            spec.velocity = [bodies.velocity(id).x, bodies.velocity(id).y];
            spec.angular_velocity = bodies.angular_velocity(id);
        }
        file.forces.retain_mut(|force| match force {
            ForceSpec::Applied { from, until, .. } => {
                *from = from.map(|t| t - time).filter(|&t| t > 0.0);
                *until = until.map(|t| t - time);
                until.is_none_or(|t| t > 0.0)
            }
            ForceSpec::Driving {
                angular_frequency,
                phase,
                ..
            } => {
                *phase += *angular_frequency * time;
                true
            }
            _ => true,
        });
        file.finish.retain_mut(|rule| match &mut rule.when {
            TriggerSpec::Time(end) => {
                *end -= time;
                *end > 0.0
            }
            _ => true,
        });
        Ok(file)
    }

    /// Builds the world and, beside it, this description with every length measured at the start written out.
    fn build_explicit(&self) -> Result<(Scene, SceneFile), SceneError> {
        let mut explicit = self.clone();
        if self.schema != SCHEMA {
            return err(format!("unsupported scene schema {}", self.schema));
        }
        if self.world.dt.is_nan() || self.world.dt <= 0.0 {
            return err("world.dt must be positive");
        }
        if self.bodies.is_empty() {
            return err("a scene needs at least one body");
        }
        if self
            .playback
            .is_some_and(|rate| rate.is_nan() || rate <= 0.0)
        {
            return err("playback must be positive");
        }
        let mut world = World::new(v(self.world.gravity));
        let mut ids: HashMap<&str, BodyId> = HashMap::new();
        let mut labels = Vec::new();
        for spec in &self.bodies {
            if ids.contains_key(spec.id.as_str()) {
                return err(format!("duplicate body id `{}`", spec.id));
            }
            let id = world.add(build_body(spec)?);
            ids.insert(spec.id.as_str(), id);
            labels.push(spec.id.clone());
        }
        let lookup = |name: &str| -> Result<BodyId, SceneError> {
            ids.get(name)
                .copied()
                .ok_or_else(|| SceneError(format!("unknown body `{name}`")))
        };
        let anchor = |a: &AnchorSpec| -> Result<Anchor, SceneError> {
            Ok(match a {
                AnchorSpec::Fixed(p) => Anchor::Fixed(v(*p)),
                AnchorSpec::Body { id, local } => Anchor::Body {
                    id: lookup(id)?,
                    local: v(*local),
                },
            })
        };

        for (index, spec) in self.forces.iter().enumerate() {
            match spec {
                ForceSpec::Applied {
                    body,
                    force,
                    point,
                    from,
                    until,
                } => {
                    let mut applied = Applied::new(lookup(body)?, v(*force)).at_local(v(*point));
                    if let Some(t) = from {
                        applied = applied.from(*t);
                    }
                    if let Some(t) = until {
                        applied = applied.until(*t);
                    }
                    world.add_force(applied);
                }
                ForceSpec::Torque { body, torque } => {
                    world.add_force(Torque::new(lookup(body)?, *torque))
                }
                ForceSpec::Spring {
                    a,
                    b,
                    stiffness,
                    rest_length,
                    pushes_only,
                } => {
                    let (a, b) = (anchor(a)?, anchor(b)?);
                    let rest = rest_length.unwrap_or_else(|| {
                        (b.world_point(world.bodies()) - a.world_point(world.bodies())).length()
                    });
                    if let ForceSpec::Spring { rest_length, .. } = &mut explicit.forces[index] {
                        *rest_length = Some(rest);
                    }
                    let spring = Spring::new(a, b, *stiffness, rest);
                    world.add_force(if *pushes_only {
                        spring.pushing_only()
                    } else {
                        spring
                    });
                }
                ForceSpec::LinearDrag { body, coefficient } => {
                    world.add_force(LinearDrag::new(lookup(body)?, *coefficient))
                }
                ForceSpec::QuadraticDrag { body, coefficient } => {
                    world.add_force(QuadraticDrag::new(lookup(body)?, *coefficient))
                }
                ForceSpec::Driving {
                    body,
                    amplitude,
                    angular_frequency,
                    phase,
                } => world.add_force(
                    Driving::new(lookup(body)?, v(*amplitude), *angular_frequency).phase(*phase),
                ),
                ForceSpec::Gravitation { constant, fixed } => {
                    let mut gravitation = Gravitation::new(*constant);
                    for source in fixed {
                        if source.mass.is_nan() || source.mass <= 0.0 {
                            return err("gravitation: a fixed mass must be positive");
                        }
                        gravitation = gravitation.source(v(source.at), source.mass);
                    }
                    world.add_force(gravitation);
                }
            }
        }

        for (index, spec) in self.constraints.iter().enumerate() {
            match spec {
                ConstraintSpec::Pin { a, b } => match (anchor(a)?, anchor(b)?) {
                    (Anchor::Body { id, local }, Anchor::Fixed(p))
                    | (Anchor::Fixed(p), Anchor::Body { id, local }) => {
                        world.add_constraint(PinJoint::to_world(id, local, p));
                    }
                    (Anchor::Body { id: a, local: la }, Anchor::Body { id: b, local: lb }) => {
                        world.add_constraint(PinJoint::between(a, la, b, lb));
                    }
                    (Anchor::Fixed(_), Anchor::Fixed(_)) => {
                        return err("a pin needs at least one body")
                    }
                },
                ConstraintSpec::Rod { a, b, length } => {
                    let (a, b) = (anchor(a)?, anchor(b)?);
                    let length = length.unwrap_or_else(|| {
                        (b.world_point(world.bodies()) - a.world_point(world.bodies())).length()
                    });
                    if let ConstraintSpec::Rod {
                        length: written, ..
                    } = &mut explicit.constraints[index]
                    {
                        *written = Some(length);
                    }
                    world.add_constraint(Rod::new(a, b, length));
                }
                ConstraintSpec::Slot {
                    body,
                    local,
                    origin,
                    direction,
                } => {
                    world.add_constraint(Slot::new(
                        lookup(body)?,
                        v(*local),
                        v(*origin),
                        v(*direction),
                    ));
                }
                ConstraintSpec::FixedAngle { body, angle } => {
                    world.add_constraint(FixedAngle::new(lookup(body)?, *angle));
                }
                ConstraintSpec::Rope { nodes, length } => {
                    let mut built = Vec::with_capacity(nodes.len());
                    for node in nodes {
                        built.push(match node {
                            RopeNodeSpec::Attach { body, local } => RopeNode::Attach {
                                body: lookup(body)?,
                                local: v(*local),
                            },
                            RopeNodeSpec::Peg(p) => RopeNode::Peg(v(*p)),
                            RopeNodeSpec::Wheel { body, radius, wrap } => RopeNode::Wheel {
                                body: lookup(body)?,
                                radius: *radius,
                                wrap: match wrap {
                                    WrapSpec::Clockwise => Wrap::Clockwise,
                                    WrapSpec::CounterClockwise => Wrap::CounterClockwise,
                                },
                            },
                            RopeNodeSpec::Pulley {
                                center,
                                radius,
                                wrap,
                            } => RopeNode::Pulley {
                                center: v(*center),
                                radius: *radius,
                                wrap: match wrap {
                                    WrapSpec::Clockwise => Wrap::Clockwise,
                                    WrapSpec::CounterClockwise => Wrap::CounterClockwise,
                                },
                            },
                        });
                    }
                    let rope = match length {
                        Some(length) => {
                            Rope::taut(built, world.bodies()).with_length(world.bodies(), *length)
                        }
                        None => Rope::taut(built, world.bodies()),
                    };
                    if let ConstraintSpec::Rope {
                        length: written, ..
                    } = &mut explicit.constraints[index]
                    {
                        *written = Some(rope.length());
                    }
                    world.add_constraint(rope);
                }
            }
        }

        for [a, b] in &self.ignore_collisions {
            world.ignore_collisions(lookup(a)?, lookup(b)?);
        }

        let focus = match &self.focus {
            Some(id) => lookup(id)?,
            None => world
                .bodies()
                .ids()
                .find(|&id| world.bodies().is_dynamic(id))
                .unwrap_or(BodyId::from_index(0)),
        };
        let mut finish = Vec::with_capacity(self.finish.len());
        for rule in &self.finish {
            let trigger = match &rule.when {
                TriggerSpec::Contact([a, b]) => Trigger::Contact(lookup(a)?, lookup(b)?),
                TriggerSpec::Separation([a, b]) => Trigger::Separation(lookup(a)?, lookup(b)?),
                TriggerSpec::Rest(seconds) => Trigger::Rest(*seconds),
                TriggerSpec::Time(seconds) => Trigger::Time(*seconds),
            };
            if rule.hold < 0.0 {
                return err("finish.hold must not be negative");
            }
            finish.push(Finish::new(trigger).hold(rule.hold));
        }
        let view = self.view.unwrap_or_else(|| {
            let p = world.bodies().position(focus);
            View {
                center: [p.x, p.y],
                half_height: 3.0,
                half_width: None,
            }
        });
        let scene = Scene {
            world,
            dt: self.world.dt,
            labels,
            focus,
            exact: self
                .answers
                .iter()
                .map(|a| (a.name.clone(), a.value))
                .collect(),
            view,
            finish,
            playback: self.playback.unwrap_or(1.0),
        };
        Ok((scene, explicit))
    }
}

/// How far a shape's centre of mass lies from the origin its description is written about.
fn centroid_offset(shape: &ShapeSpec) -> Vec2 {
    match shape {
        ShapeSpec::Polygon(points) if points.len() >= 3 => {
            Polygon::with_centroid(points.iter().map(|&p| v(p)).collect::<Vec<_>>()).1
        }
        _ => Vec2::ZERO,
    }
}

fn build_body(spec: &BodySpec) -> Result<Body, SceneError> {
    // Polygon vertices are given in the body's frame about any origin; the body's centre of mass is their centroid,
    // so the position absorbs the offset. A static incline drawn in world coordinates then lands where it was drawn.
    let mut centroid = Vec2::ZERO;
    let shape: Shape = match &spec.shape {
        ShapeSpec::Circle(r) => Circle::new(*r).into(),
        ShapeSpec::Rectangle([w, h]) => Polygon::rectangle(*w, *h).into(),
        ShapeSpec::Polygon(points) => {
            if points.len() < 3 {
                return err(format!(
                    "body `{}`: a polygon needs at least three vertices",
                    spec.id
                ));
            }
            let (polygon, offset) =
                Polygon::with_centroid(points.iter().map(|&p| v(p)).collect::<Vec<_>>());
            centroid = offset;
            polygon.into()
        }
        ShapeSpec::Halfplane { normal, offset } => HalfPlane::new(v(*normal), *offset).into(),
    };
    let mut body = match spec.kind {
        KindSpec::Static => Body::fixed(shape),
        KindSpec::Particle | KindSpec::Rigid => {
            let mass = spec
                .mass
                .ok_or_else(|| SceneError(format!("body `{}` needs a mass", spec.id)))?;
            if mass.is_nan() || mass <= 0.0 {
                return err(format!("body `{}`: mass must be positive", spec.id));
            }
            if spec.kind == KindSpec::Particle {
                Body::particle(mass).shape(shape)
            } else {
                Body::rigid(mass, shape)
            }
        }
    };
    let position = v(spec.position) + centroid.rotated(spec.angle);
    body = body
        .at(position.x, position.y)
        .angle(spec.angle)
        .velocity(spec.velocity[0], spec.velocity[1]);
    body = body.spin(spec.angular_velocity);
    if let Some([s, k]) = spec.friction {
        body = body.friction(s, k);
    }
    if let Some(e) = spec.restitution {
        body = body.restitution(e);
    }
    if let Some(i) = spec.inertia {
        body = body.inertia(i);
    }
    Ok(body)
}
