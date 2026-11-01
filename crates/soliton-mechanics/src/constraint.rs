use crate::body::{Bodies, BodyId};
use crate::force::{Anchor, ForceLabel};
use soliton_core::Vec2;

/// How a row's impulse is bounded and what it means.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RowKind {
    /// `C = 0`; the impulse may have either sign.
    Bilateral,
    /// `C >= 0` with a non-negative impulse: ropes.
    Rope,
    /// Non-penetration of a contact; non-negative impulse, with a restitution target in `bias`.
    Normal,
    /// Coulomb friction of a contact, bounded by the normal impulse of row `normal`.
    Friction {
        normal: usize,
        static_friction: f64,
        kinetic_friction: f64,
    },
}

/// One scalar equation with its Jacobian. `source` is the constraint or contact index it came from.
#[derive(Clone, Copy, Debug)]
pub struct Row {
    pub source: usize,
    pub label: ForceLabel,
    pub kind: RowKind,
    pub error: f64,
    /// Target relative velocity after the solve; zero except for restitution.
    pub bias: f64,
    pub active: bool,
    pub(crate) start: usize,
    pub(crate) len: usize,
}

impl Row {
    pub fn unilateral(&self) -> bool {
        !matches!(self.kind, RowKind::Bilateral)
    }
}

/// One body's share of a row: `J v = linear · v + angular · ω`.
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub body: BodyId,
    pub linear: Vec2,
    pub angular: f64,
    pub point: Vec2,
}

/// The rows of every constraint at the current positions, rebuilt each time they are needed.
#[derive(Clone, Debug, Default)]
pub struct Rows {
    pub(crate) rows: Vec<Row>,
    pub(crate) entries: Vec<Entry>,
}

impl Rows {
    pub(crate) fn clear(&mut self) {
        self.rows.clear();
        self.entries.clear();
    }

    pub fn bilateral(&mut self, source: usize, label: ForceLabel, error: f64) {
        self.push(source, label, RowKind::Bilateral, error, 0.0, true);
    }

    pub fn rope(&mut self, source: usize, label: ForceLabel, error: f64, active: bool) {
        self.push(source, label, RowKind::Rope, error, 0.0, active);
    }

    pub(crate) fn push(
        &mut self,
        source: usize,
        label: ForceLabel,
        kind: RowKind,
        error: f64,
        bias: f64,
        active: bool,
    ) {
        let start = self.entries.len();
        self.rows.push(Row {
            source,
            label,
            kind,
            error,
            bias,
            active,
            start,
            len: 0,
        });
    }

    pub(crate) fn len(&self) -> usize {
        self.rows.len()
    }

    /// A copy of the rows from `from` onward, with their entries rebased.
    pub(crate) fn tail(&self, from: usize) -> Rows {
        let mut out = Rows::default();
        for row in &self.rows[from..] {
            let start = out.entries.len();
            out.entries.extend_from_slice(self.entries_of(row));
            out.rows.push(Row { start, ..*row });
        }
        out
    }

    /// Adds a body to the last row through a force-like direction at a world point.
    pub fn at_point(&mut self, bodies: &Bodies, body: BodyId, direction: Vec2, point: Vec2) {
        if !bodies.is_dynamic(body) {
            return;
        }
        let angular = (point - bodies.position(body)).cross(direction);
        self.entries.push(Entry {
            body,
            linear: direction,
            angular,
            point,
        });
        self.rows.last_mut().unwrap().len += 1;
    }

    /// Adds a body to the last row through a pure angular term.
    pub fn angular(&mut self, bodies: &Bodies, body: BodyId, angular: f64) {
        if !bodies.is_dynamic(body) {
            return;
        }
        self.entries.push(Entry {
            body,
            linear: Vec2::ZERO,
            angular,
            point: bodies.position(body),
        });
        self.rows.last_mut().unwrap().len += 1;
    }

    pub(crate) fn entries_of(&self, row: &Row) -> &[Entry] {
        &self.entries[row.start..row.start + row.len]
    }
}

/// A relation between bodies enforced exactly by impulses (design 002).
pub trait Constraint {
    fn label(&self) -> ForceLabel;

    /// Appends this constraint's rows for the current positions. `index` identifies it in events and reactions.
    fn rows(&self, bodies: &Bodies, index: usize, rows: &mut Rows);
}

/// A point on a body coincides with a point in the world or on another body.
#[derive(Clone, Copy, Debug)]
pub struct PinJoint {
    a: Anchor,
    b: Anchor,
}

impl PinJoint {
    /// Pins a point of `body` (in its frame) to a fixed world point.
    pub fn to_world(body: BodyId, local: Vec2, world: Vec2) -> Self {
        Self {
            a: Anchor::Body { id: body, local },
            b: Anchor::Fixed(world),
        }
    }

    pub fn between(a: BodyId, local_a: Vec2, b: BodyId, local_b: Vec2) -> Self {
        Self {
            a: Anchor::Body {
                id: a,
                local: local_a,
            },
            b: Anchor::Body {
                id: b,
                local: local_b,
            },
        }
    }
}

impl Constraint for PinJoint {
    fn label(&self) -> ForceLabel {
        ForceLabel::Reaction
    }

    fn rows(&self, bodies: &Bodies, index: usize, rows: &mut Rows) {
        let (pa, pb) = (self.a.world_point(bodies), self.b.world_point(bodies));
        let gap = pa - pb;
        for axis in [Vec2::X, Vec2::Y] {
            rows.bilateral(index, self.label(), gap.dot(axis));
            anchor_entry(rows, bodies, self.a, axis, pa);
            anchor_entry(rows, bodies, self.b, -axis, pb);
        }
    }
}

/// Two anchors stay a fixed distance apart. Reports tension, positive when pulling the anchors together.
#[derive(Clone, Copy, Debug)]
pub struct Rod {
    a: Anchor,
    b: Anchor,
    length: f64,
}

impl Rod {
    pub fn new(a: Anchor, b: Anchor, length: f64) -> Self {
        assert!(length > 0.0, "rod length must be positive");
        Self { a, b, length }
    }
}

impl Constraint for Rod {
    fn label(&self) -> ForceLabel {
        ForceLabel::Tension
    }

    fn rows(&self, bodies: &Bodies, index: usize, rows: &mut Rows) {
        let (pa, pb) = (self.a.world_point(bodies), self.b.world_point(bodies));
        let separation = pb - pa;
        let direction = separation.normalized();
        rows.bilateral(index, self.label(), self.length - separation.length());
        anchor_entry(rows, bodies, self.a, direction, pa);
        anchor_entry(rows, bodies, self.b, -direction, pb);
    }
}

/// A point on a body stays on a line. Reports the normal reaction. A block on a smooth table is a slot.
#[derive(Clone, Copy, Debug)]
pub struct Slot {
    body: BodyId,
    local: Vec2,
    origin: Vec2,
    normal: Vec2,
}

impl Slot {
    pub fn new(body: BodyId, local: Vec2, origin: Vec2, direction: Vec2) -> Self {
        Self {
            body,
            local,
            origin,
            normal: direction.normalized().perp(),
        }
    }

    pub fn along(body: BodyId, origin: Vec2, direction: Vec2) -> Self {
        Self::new(body, Vec2::ZERO, origin, direction)
    }
}

impl Constraint for Slot {
    fn label(&self) -> ForceLabel {
        ForceLabel::Normal
    }

    fn rows(&self, bodies: &Bodies, index: usize, rows: &mut Rows) {
        let point = bodies.world_point(self.body, self.local);
        rows.bilateral(index, self.label(), (point - self.origin).dot(self.normal));
        rows.at_point(bodies, self.body, self.normal, point);
    }
}

/// Locks a body's orientation.
#[derive(Clone, Copy, Debug)]
pub struct FixedAngle {
    body: BodyId,
    angle: f64,
}

impl FixedAngle {
    pub fn new(body: BodyId, angle: f64) -> Self {
        Self { body, angle }
    }
}

impl Constraint for FixedAngle {
    fn label(&self) -> ForceLabel {
        ForceLabel::Reaction
    }

    fn rows(&self, bodies: &Bodies, index: usize, rows: &mut Rows) {
        rows.bilateral(index, self.label(), bodies.angle(self.body) - self.angle);
        rows.angular(bodies, self.body, 1.0);
    }
}

fn anchor_entry(rows: &mut Rows, bodies: &Bodies, anchor: Anchor, direction: Vec2, point: Vec2) {
    if let Anchor::Body { id, .. } = anchor {
        rows.at_point(bodies, id, direction, point);
    }
}
