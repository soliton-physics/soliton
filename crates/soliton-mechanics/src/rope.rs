use crate::body::{Bodies, BodyId};
use crate::constraint::{Constraint, Rows};
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
    length: f64,
}

/// How far the path may exceed the rope length before the rope is treated as taut.
const TAUT_TOLERANCE: f64 = 1e-9;

struct Attachment {
    body: BodyId,
    point: Vec2,
    gradient: Vec2,
}

impl Rope {
    pub fn new(nodes: impl Into<Vec<RopeNode>>, length: f64) -> Self {
        let nodes = nodes.into();
        assert!(nodes.len() >= 2, "a rope needs at least two nodes");
        assert!(length > 0.0, "rope length must be positive");
        Self { nodes, length }
    }

    /// A rope whose length is whatever the path measures at the current positions: taut from the start.
    pub fn taut(nodes: impl Into<Vec<RopeNode>>, bodies: &Bodies) -> Self {
        let nodes = nodes.into();
        let mut rope = Self { nodes, length: 1.0 };
        rope.length = rope.path_length(bodies);
        rope
    }

    pub fn length(&self) -> f64 {
        self.length
    }

    pub fn path_length(&self, bodies: &Bodies) -> f64 {
        self.path(bodies).0
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
        }
    }

    /// Total length, the attachments with the gradient of the length with respect to each, and the drawn path.
    fn path(&self, bodies: &Bodies) -> (f64, Vec<Attachment>, Vec<Vec2>) {
        let n = self.nodes.len();
        let mut length = 0.0;
        let mut attachments = Vec::new();
        let mut drawn = Vec::new();

        // Each node contributes the point where the rope leaves it towards the next node and the point where it
        // arrives from the previous one; for pegs and attachments these coincide.
        let mut leaves = vec![Vec2::ZERO; n];
        let mut arrives = vec![Vec2::ZERO; n];
        for i in 0..n {
            match self.nodes[i] {
                RopeNode::Pulley {
                    center,
                    radius,
                    wrap,
                } => {
                    assert!(i > 0 && i + 1 < n, "a pulley needs a node on each side");
                    let (previous, next) = (
                        self.node_point(bodies, i - 1),
                        self.node_point(bodies, i + 1),
                    );
                    let (arrive, leave, arc) =
                        pulley_tangents(center, radius, wrap, previous, next);
                    arrives[i] = arrive;
                    leaves[i] = leave;
                    length += arc;
                }
                _ => {
                    let p = self.node_point(bodies, i);
                    arrives[i] = p;
                    leaves[i] = p;
                }
            }
        }

        for i in 0..n {
            drawn.push(arrives[i]);
            if arrives[i] != leaves[i] {
                drawn.push(leaves[i]);
            }
            if i + 1 < n {
                length += (arrives[i + 1] - leaves[i]).length();
            }
            if let RopeNode::Attach { body, .. } = self.nodes[i] {
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
                });
            }
        }
        (length, attachments, drawn)
    }
}

impl Constraint for Rope {
    fn label(&self) -> ForceLabel {
        ForceLabel::Tension
    }

    fn rows(&self, bodies: &Bodies, index: usize, rows: &mut Rows) {
        let (length, attachments, _) = self.path(bodies);
        let slack = self.length - length;
        rows.rope(index, self.label(), slack, slack <= TAUT_TOLERANCE);
        for a in &attachments {
            rows.at_point(bodies, a.body, -a.gradient, a.point);
        }
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
    let tangent_angle = |p: Vec2, sign: f64| {
        let d = p - center;
        let distance = d.length();
        assert!(distance > radius, "rope end point lies inside the pulley");
        d.angle() + sign * (radius / distance).acos()
    };
    let sign = match wrap {
        Wrap::Clockwise => 1.0,
        Wrap::CounterClockwise => -1.0,
    };
    let arrive_angle = tangent_angle(previous, -sign);
    let leave_angle = tangent_angle(next, sign);
    let arrive = center + Vec2::from_polar(radius, arrive_angle);
    let leave = center + Vec2::from_polar(radius, leave_angle);
    let sweep = (sign * (arrive_angle - leave_angle)).rem_euclid(TAU);
    (arrive, leave, radius * sweep)
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
