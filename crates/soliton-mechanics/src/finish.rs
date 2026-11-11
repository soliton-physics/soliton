//! Where the problem a scene poses ends (design 004). A scene lists rules; the first to fire wins. Stepping past
//! the end is always allowed, and once the problem has ended the watch keeps looking for the moment everything
//! comes to rest, so a front end can say so.

use crate::body::BodyId;
use crate::event::{Event, EventKind};
use crate::world::World;

/// What makes a rule fire.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Trigger {
    /// The two bodies first touch.
    Contact(BodyId, BodyId),
    /// Two touching bodies come apart.
    Separation(BodyId, BodyId),
    /// Every moving body has been still for this many seconds.
    Rest(f64),
    /// The clock reaches this time.
    Time(f64),
}

/// One finish rule: a trigger and how long to keep going after it fires.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Finish {
    pub trigger: Trigger,
    pub hold: f64,
}

impl Finish {
    pub fn new(trigger: Trigger) -> Self {
        Self { trigger, hold: 0.0 }
    }

    pub fn hold(mut self, seconds: f64) -> Self {
        self.hold = seconds;
        self
    }
}

/// The end of the problem: when it happened and which trigger decided it. For a rest trigger `time` is the moment
/// the bodies became still, not the moment the wait ran out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Finished {
    pub time: f64,
    pub trigger: Trigger,
}

impl Finished {
    /// The reason in words, naming bodies by their scene labels.
    pub fn describe(&self, labels: &[String]) -> String {
        let name = |id: BodyId| labels[id.index()].as_str();
        match self.trigger {
            Trigger::Contact(a, b) => format!("{} touched {}", name(a), name(b)),
            Trigger::Separation(a, b) => format!("{} left {}", name(a), name(b)),
            Trigger::Rest(_) if self.time == 0.0 => {
                "nothing moves, it is in equilibrium from the start".to_string()
            }
            Trigger::Rest(_) => "everything came to rest".to_string(),
            Trigger::Time(_) => "the problem's time window ended".to_string(),
        }
    }
}

/// Speeds below this, in m/s and rad/s, count as still.
const REST_SPEED: f64 = 1e-3;
/// How long everything must stay still, after the problem has ended, before the watch reports rest.
const REST_AFTER_FINISH: f64 = 0.5;
/// Steps to keep going after a trigger fires, so the reactions shown at the end are steady ones rather than the
/// impulse of the impact that ended the problem.
const SETTLE_STEPS: u32 = 2;
const TIME_TOLERANCE: f64 = 1e-12;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    /// Waiting for one of the scene's rules.
    Rules,
    /// The problem has ended; waiting for everything to come to rest.
    Rest,
    Done,
}

#[derive(Clone, Copy, Debug)]
struct Pending {
    finished: Finished,
    due: f64,
    settle: u32,
}

/// Watches a simulation for its finish rules.
#[derive(Clone, Debug)]
pub(crate) struct FinishWatch {
    rules: Vec<Finish>,
    stage: Stage,
    still_since: Option<f64>,
    pending: Option<Pending>,
}

impl Default for FinishWatch {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl FinishWatch {
    pub(crate) fn new(rules: Vec<Finish>) -> Self {
        let stage = if rules.is_empty() {
            Stage::Done
        } else {
            Stage::Rules
        };
        Self {
            rules,
            stage,
            still_since: Some(0.0),
            pending: None,
        }
    }

    pub(crate) fn rules(&self) -> &[Finish] {
        &self.rules
    }

    /// Called once after every step with the events that step produced. Returns the end of the problem on the
    /// step it is reached, and later, once, the moment everything came to rest.
    pub(crate) fn observe(
        &mut self,
        world: &World,
        new_events: &[Event],
        time: f64,
    ) -> Option<Finished> {
        if is_still(world) {
            self.still_since.get_or_insert(time);
        } else {
            self.still_since = None;
        }

        if let Some(pending) = &mut self.pending {
            pending.settle = pending.settle.saturating_sub(1);
            if pending.settle > 0 || time < pending.due - TIME_TOLERANCE {
                return None;
            }
            let finished = pending.finished;
            self.pending = None;
            self.stage = match finished.trigger {
                Trigger::Rest(_) => Stage::Done,
                _ => Stage::Rest,
            };
            return Some(finished);
        }

        let fired = match self.stage {
            Stage::Rules => self
                .rules
                .iter()
                .find_map(|rule| self.fire(rule, new_events, time)),
            Stage::Rest => self.fire(
                &Finish::new(Trigger::Rest(REST_AFTER_FINISH)),
                new_events,
                time,
            ),
            Stage::Done => None,
        };
        self.pending = fired.map(|(finished, due)| Pending {
            finished,
            due,
            settle: SETTLE_STEPS,
        });
        None
    }

    /// Whether `rule` fires on this step: the end it reports and the time to keep running until.
    fn fire(&self, rule: &Finish, new_events: &[Event], time: f64) -> Option<(Finished, f64)> {
        let pair =
            |x: BodyId, y: BodyId, a: BodyId, b: BodyId| (x == a && y == b) || (x == b && y == a);
        let at = match rule.trigger {
            Trigger::Contact(a, b) => new_events.iter().find_map(|e| match e.kind {
                EventKind::ContactBegin { a: x, b: y } if pair(x, y, a, b) => Some(e.time),
                _ => None,
            })?,
            Trigger::Separation(a, b) => new_events.iter().find_map(|e| match e.kind {
                EventKind::ContactEnd { a: x, b: y } if pair(x, y, a, b) => Some(e.time),
                _ => None,
            })?,
            Trigger::Time(end) => {
                if time < end - TIME_TOLERANCE {
                    return None;
                }
                end
            }
            Trigger::Rest(seconds) => {
                let since = self.still_since?;
                if time - since < seconds - TIME_TOLERANCE {
                    return None;
                }
                return Some((
                    Finished {
                        time: since,
                        trigger: rule.trigger,
                    },
                    time + rule.hold,
                ));
            }
        };
        let end = at + rule.hold;
        Some((
            Finished {
                time: end,
                trigger: rule.trigger,
            },
            end,
        ))
    }
}

fn is_still(world: &World) -> bool {
    let bodies = world.bodies();
    bodies.ids().filter(|&id| bodies.is_dynamic(id)).all(|id| {
        bodies.velocity(id).length() < REST_SPEED && bodies.angular_velocity(id).abs() < REST_SPEED
    })
}
