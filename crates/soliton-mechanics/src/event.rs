use crate::body::BodyId;

/// A discrete happening the physics reports. Constraints and contact produce these (designs 002 and 003).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Event {
    pub time: f64,
    pub kind: EventKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EventKind {
    ContactBegin { a: BodyId, b: BodyId },
    ContactEnd { a: BodyId, b: BodyId },
    RopeTaut { rope: usize },
    RopeSlack { rope: usize },
    FrictionSlip { a: BodyId, b: BodyId },
    FrictionStick { a: BodyId, b: BodyId },
}
