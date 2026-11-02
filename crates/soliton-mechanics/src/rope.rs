use crate::body::{Bodies, BodyId};
use crate::constraint::{Constraint, Rows, Sketch};
use crate::force::ForceLabel;
use soliton_core::Vec2;
use std::f64::consts::TAU;

/// Which way the rope turns as it passes around a pulley, travelling from the previous node to the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrap {
    Clockwise,
    CounterClockwise,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RopeNode {
    /// The rope is tied to a point on a body.
    Attach { body: BodyId, local: Vec2 },
    /// A frictionless fixed peg the rope bends around as a point.
    Peg(Vec2),
    /// A massless, frictionless pulley at a fixed centre; the rope follows the tangent geometry.
    Pulley {
        center: Vec2,
        radius: f64,
        wrap: Wrap,
    },
    /// A rigid body the rope wraps without slipping, so its rotation is coupled to the rope's motion.
    /// The body is normally pinned; the rope's tangent points move with its rim.
    Wheel {
        body: BodyId,
        radius: f64,
        wrap: Wrap,
    },
}

impl RopeNode {
    pub fn attach(body: BodyId) -> Self {
        Self::Attach {
            body,
            local: Vec2::ZERO,
        }
    }
}

/// An inextensible string of fixed length through a sequence of nodes. It pulls when taut and does nothing when
/// slack, and it reports its tension (design 002).
#[derive(Clone, Debug)]
pub struct Rope {
    nodes: Vec<RopeNode>,
    /// Rest length of each segment between consecutive wheels; one entry for a rope without wheels.
    lengths: Vec<f64>,
    /// Each wheel's angle when the rope was created, so rope wound on or paid out from the rim counts towards the
    /// adjacent segments' lengths.
    reference_angles: Option<Vec<f64>>,
}

/// How far the path may exceed the rope length before the rope is treated as taut.
const TAUT_TOLERANCE: f64 = 1e-9;

struct Attachment {
    body: BodyId,
    point: Vec2,
    gradient: Vec2,
    segment: usize,
}

impl Rope {
    /// A rope of the given total length. With wheels, the slack is given to the last segment.
    pub fn new(nodes: impl Into<Vec<RopeNode>>, length: f64) -> Self {
        let nodes = nodes.into();
        assert!(nodes.len() >= 2, "a rope needs at least two nodes");
        assert!(length > 0.0, "rope length must be positive");
        let segments = 1 + nodes
            .iter()
            .filter(|n| matches!(n, RopeNode::Wheel { .. }))
            .count();
        Self {
            nodes,
            lengths: vec![length / segments as f64; segments],
            reference_angles: None,
        }
    }

    /// A rope whose segments are exactly as long as the path measures now: taut from the start.
    pub fn taut(nodes: impl Into<Vec<RopeNode>>, bodies: &Bodies) -> Self {
        let mut rope = Self::new(nodes, 1.0);
        rope.reference_angles = Some(rope.wheel_angles(bodies));
        rope.lengths = rope.path(bodies).0;
        rope
    }

    fn wheel_angles(&self, bodies: &Bodies) -> Vec<f64> {
        self.nodes
            .iter()
            .filter_map(|n| match n {
                RopeNode::Wheel { body, .. } => Some(bodies.angle(*body)),
                _ => None,
            })
            .collect()
    }

    /// Sets the total length, giving any difference from the current path to the last segment.
    pub fn with_length(mut self, bodies: &Bodies, length: f64) -> Self {
        if self.reference_angles.is_none() {
            self.reference_angles = Some(self.wheel_angles(bodies));
        }
        self.lengths = self.path(bodies).0;
        let current: f64 = self.lengths.iter().sum();
        if let Some(last) = self.lengths.last_mut() {
            *last += length - current;
        }
        self
    }

    pub fn length(&self) -> f64 {
        self.lengths.iter().sum()
    }

    pub fn path_length(&self, bodies: &Bodies) -> f64 {
        self.path(bodies).0.iter().sum()
    }

    /// World points the rope passes through, for drawing.
    pub fn path_points(&self, bodies: &Bodies) -> Vec<Vec2> {
        self.path(bodies).2
    }

    fn node_point(&self, bodies: &Bodies, i: usize) -> Vec2 {
        match self.nodes[i] {
            RopeNode::Attach { body, local } => bodies.world_point(body, local),
            RopeNode::Peg(p) => p,
            RopeNode::Pulley { center, .. } => center,
            RopeNode::Wheel { body, .. } => bodies.position(body),
        }
    }

    /// Per-segment lengths, the attachments (including wheel tangent points, which ride on the wheel) with the
    /// gradient of their segment's length with respect to each, and the drawn path.
    fn path(&self, bodies: &Bodies) -> (Vec<f64>, Vec<Attachment>, Vec<Vec2>) {
        let n = self.nodes.len();
        let mut attachments = Vec::new();
        let mut drawn = Vec::new();

        // Each node contributes the point where the rope arrives from the previous node and the point where it
        // leaves towards the next; for pegs and attachments these coincide.
        let mut leaves = vec![Vec2::ZERO; n];
        let mut arrives = vec![Vec2::ZERO; n];
        let mut arcs = vec![0.0; n];
        for i in 0..n {
            match self.nodes[i] {
                RopeNode::Pulley { radius, wrap, .. } => {
                    assert!(i > 0 && i + 1 < n, "a pulley needs a node on each side");
                    let (previous, next) = (
                        self.node_point(bodies, i - 1),
                        self.node_point(bodies, i + 1),
                    );
                    let (arrive, leave, arc) =
                        pulley_tangents(self.node_point(bodies, i), radius, wrap, previous, next);
                    arrives[i] = arrive;
                    leaves[i] = leave;
                    arcs[i] = arc;
                }
                RopeNode::Wheel { radius, wrap, .. } => {
                    // A wheel may begin or end the rope: the rope is wound on it and leaves from a tangent point.
                    let center = self.node_point(bodies, i);
                    if i > 0 {
                        arrives[i] = tangent_point(
                            center,
                            radius,
                            wrap,
                            self.node_point(bodies, i - 1),
                            true,
                        );
                    }
                    if i + 1 < n {
                        leaves[i] = tangent_point(
                            center,
                            radius,
                            wrap,
                            self.node_point(bodies, i + 1),
                            false,
                        );
                    }
                    if i == 0 {
                        arrives[i] = leaves[i];
                    }
                    if i + 1 == n {
                        leaves[i] = arrives[i];
                    }
                }
                _ => {
                    let p = self.node_point(bodies, i);
                    arrives[i] = p;
                    leaves[i] = p;
                }
            }
        }

        let mut lengths = vec![0.0];
        let mut segment = 0;
        let mut wheel_index = 0;
        let has_wheels = self
            .nodes
            .iter()
            .any(|n| matches!(n, RopeNode::Wheel { .. }));
        let reference = self.reference_angles.clone().unwrap_or_default();
        assert!(
            !has_wheels
                || reference.len()
                    == wheel_index
                        + self
                            .nodes
                            .iter()
                            .filter(|n| matches!(n, RopeNode::Wheel { .. }))
                            .count(),
            "a rope with wheels must be created with Rope::taut or Rope::with_length"
        );
        for i in 0..n {
            drawn.push(arrives[i]);
            if arrives[i] != leaves[i] {
                drawn.push(leaves[i]);
            }
            match self.nodes[i] {
                RopeNode::Attach { body, .. } => {
                    let p = arrives[i];
                    let mut gradient = Vec2::ZERO;
                    if i > 0 {
                        gradient += (p - leaves[i - 1]).normalized();
                    }
                    if i + 1 < n {
                        gradient += (p - arrives[i + 1]).normalized();
                    }
                    attachments.push(Attachment {
                        body,
                        point: p,
                        gradient,
                        segment,
                    });
                }
                RopeNode::Wheel { body, .. } => {
                    // The segment ends where the rope meets the rim and the next one begins where it leaves;
                    // both points are material points of the wheel. Rope wound onto or paid out from the rim since
                    // the rope was created shortens or lengthens the free segment by the rim travel, which is the
                    // row's angular Jacobian times the wheel's rotation.
                    let turned = bodies.angle(body) - reference[wheel_index];
                    wheel_index += 1;
                    let c = bodies.position(body);
                    if i > 0 {
                        let arrive = arrives[i];
                        let gradient = (arrive - leaves[i - 1]).normalized();
                        lengths[segment] -= (arrive - c).cross(-gradient) * turned;
                        attachments.push(Attachment {
                            body,
                            point: arrive,
                            gradient,
                            segment,
                        });
                    }
                    if i + 1 < n {
                        if i > 0 {
                            segment += 1;
                            lengths.push(0.0);
                        }
                        let leave = leaves[i];
                        let gradient = (leave - arrives[i + 1]).normalized();
                        lengths[segment] -= (leave - c).cross(-gradient) * turned;
                        attachments.push(Attachment {
                            body,
                            point: leave,
                            gradient,
                            segment,
                        });
                    }
                }
                RopeNode::Pulley { .. } => lengths[segment] += arcs[i],
                RopeNode::Peg(_) => {}
            }
            if i + 1 < n {
                lengths[segment] += (arrives[i + 1] - leaves[i]).length();
            }
        }
        (lengths, attachments, drawn)
    }
}

impl Constraint for Rope {
    fn label(&self) -> ForceLabel {
        ForceLabel::Tension
    }

    fn rows(&self, bodies: &Bodies, index: usize, rows: &mut Rows) {
        let (lengths, attachments, _) = self.path(bodies);
        for (segment, length) in lengths.iter().enumerate() {
            let slack = self.lengths[segment] - length;
            rows.rope(index, self.label(), slack, slack <= TAUT_TOLERANCE);
            for a in attachments.iter().filter(|a| a.segment == segment) {
                rows.at_point(bodies, a.body, -a.gradient, a.point);
            }
        }
    }

    fn sketches(&self, bodies: &Bodies) -> Vec<Sketch> {
        let mut out = vec![Sketch::Polyline(self.path_points(bodies))];
        for node in &self.nodes {
            if let RopeNode::Pulley { center, radius, .. } = *node {
                out.push(Sketch::Circle { center, radius });
            }
        }
        out
    }

    fn joined(&self) -> Option<(BodyId, BodyId)> {
        None
    }
}

/// Where a rope coming from `previous` touches a pulley, where it leaves towards `next`, and the arc between.
/// The gradient of the wrapped length with respect to an end point is the unit vector along its tangent segment,
/// so callers can treat the tangent points as fixed when differentiating.
fn pulley_tangents(
    center: Vec2,
    radius: f64,
    wrap: Wrap,
    previous: Vec2,
    next: Vec2,
) -> (Vec2, Vec2, f64) {
    let arrive_angle = tangent_angle(center, radius, wrap, previous, true);
    let leave_angle = tangent_angle(center, radius, wrap, next, false);
    let arrive = center + Vec2::from_polar(radius, arrive_angle);
    let leave = center + Vec2::from_polar(radius, leave_angle);
    let sweep = (wrap_sign(wrap) * (arrive_angle - leave_angle)).rem_euclid(TAU);
    (arrive, leave, radius * sweep)
}

fn wrap_sign(wrap: Wrap) -> f64 {
    match wrap {
        Wrap::Clockwise => 1.0,
        Wrap::CounterClockwise => -1.0,
    }
}

/// Angle on the rim where the rope from `p` touches, arriving at or leaving the pulley.
/// A point inside the pulley has no tangent; it is treated as sitting on the rim so the geometry stays finite.
fn tangent_angle(center: Vec2, radius: f64, wrap: Wrap, p: Vec2, arriving: bool) -> f64 {
    let d = p - center;
    let distance = d.length().max(radius);
    let sign = if arriving {
        -wrap_sign(wrap)
    } else {
        wrap_sign(wrap)
    };
    d.angle() + sign * (radius / distance).acos()
}

fn tangent_point(center: Vec2, radius: f64, wrap: Wrap, p: Vec2, arriving: bool) -> Vec2 {
    center + Vec2::from_polar(radius, tangent_angle(center, radius, wrap, p, arriving))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atwood_tangents_are_horizontal_extremes() {
        let (arrive, leave, arc) = pulley_tangents(
            Vec2::ZERO,
            1.0,
            Wrap::Clockwise,
            Vec2::new(-1.0, -5.0),
            Vec2::new(1.0, -5.0),
        );
        assert!((arrive - Vec2::new(-1.0, 0.0)).length() < 1e-12);
        assert!((leave - Vec2::new(1.0, 0.0)).length() < 1e-12);
        assert!((arc - std::f64::consts::PI).abs() < 1e-12);
    }
}
