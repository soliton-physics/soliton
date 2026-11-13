use crate::curve::{Curve, Hit};
use soliton_core::Vec2;

/// Something on the optical bench that light meets.
#[derive(Clone, Debug, PartialEq)]
pub enum Element {
    /// Reflects everything that reaches it, from either side.
    Mirror(Curve),
    /// Stops a ray: a screen, or the blackened back of something.
    Screen(Curve),
    /// A piece of transparent material: a closed outline and its refractive index. Where two overlap, the later
    /// one in the list is what is there.
    Glass { outline: Vec<Curve>, index: f64 },
    /// An ideal thin lens: every ray through it is turned by `height / focal`, whatever its height or slope, so
    /// it obeys the lens formula exactly and has no aberration. `axis` is a unit vector along its principal
    /// axis and `aperture` how far it reaches either side of it.
    ThinLens {
        centre: Vec2,
        axis: Vec2,
        focal: f64,
        aperture: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec2,
    /// Unit vector.
    pub direction: Vec2,
}

impl Ray {
    pub fn new(origin: Vec2, direction: Vec2) -> Self {
        Self {
            origin,
            direction: direction.normalized(),
        }
    }

    /// The ray from `origin` that passes through `through`.
    pub fn towards(origin: Vec2, through: Vec2) -> Self {
        Self::new(origin, through - origin)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Reflection,
    Refraction,
    /// Refraction was impossible: the ray met a rarer medium beyond the critical angle.
    TotalReflection,
    /// Turned by an ideal thin lens.
    Lens,
    Absorbed,
}

/// One meeting of a ray with an element.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Event {
    pub at: Vec2,
    pub kind: EventKind,
    /// Unit normal to the surface, on the side the ray came from.
    pub normal: Vec2,
    /// Angle between the incoming ray and the normal, radians.
    pub incidence: f64,
    /// Angle between the outgoing ray and the normal, on whichever side it leaves, radians.
    pub emergence: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// Left the bench.
    Escaped,
    Absorbed,
    /// Still bouncing when the count of meetings ran out, as between two mirrors.
    Exhausted,
}

/// The whole course of one ray.
#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    /// Where it started and every point at which it turned, then where it stopped being followed.
    pub points: Vec<Vec2>,
    pub events: Vec<Event>,
    pub end: End,
    /// The direction it started in and the one it finished in.
    pub first: Vec2,
    pub last: Vec2,
}

impl Path {
    /// The angle the ray has been turned through in all, radians, from 0 to π.
    pub fn deviation(&self) -> f64 {
        self.first
            .cross(self.last)
            .atan2(self.first.dot(self.last))
            .abs()
    }

    /// Where the last straight stretch begins.
    pub fn last_turn(&self) -> Vec2 {
        self.points[self.points.len().saturating_sub(2)]
    }
}

/// Where the rays from one point of an object come together again.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Image {
    pub at: Vec2,
    /// The rays really pass through it; otherwise they only seem to come from it.
    pub real: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bench {
    pub elements: Vec<Element>,
    /// Refractive index of whatever fills the space between the elements.
    pub ambient: f64,
}

/// A ray is not allowed to meet again the surface it has just left.
const CLEAR: f64 = 1e-9;
/// How far either side of a surface the medium is sampled to decide what the ray is passing between.
const PROBE: f64 = 1e-7;
const MEETINGS: usize = 200;

impl Default for Bench {
    fn default() -> Self {
        Self {
            elements: Vec::new(),
            ambient: 1.0,
        }
    }
}

impl Bench {
    pub fn new(elements: Vec<Element>) -> Self {
        Self {
            elements,
            ambient: 1.0,
        }
    }

    /// The refractive index at a point.
    pub fn index_at(&self, point: Vec2) -> f64 {
        // A fixed direction no outline in a scene is likely to run along.
        let probe = Vec2::new(0.754_877_666_2, 0.655_865_618_0);
        let mut index = self.ambient;
        for element in &self.elements {
            if let Element::Glass { outline, index: n } = element {
                let crossings: usize = outline
                    .iter()
                    .map(|curve| {
                        curve
                            .hits(point, probe)
                            .iter()
                            .filter(|h| h.distance > 0.0)
                            .count()
                    })
                    .sum();
                if crossings % 2 == 1 {
                    index = *n;
                }
            }
        }
        index
    }

    fn nearest(&self, ray: Ray) -> Option<(usize, Hit)> {
        let mut best: Option<(usize, Hit)> = None;
        let mut offer = |index: usize, hits: Vec<Hit>| {
            for hit in hits {
                if hit.distance > CLEAR && best.is_none_or(|(_, b)| hit.distance < b.distance) {
                    best = Some((index, hit));
                }
            }
        };
        for (index, element) in self.elements.iter().enumerate() {
            match element {
                Element::Mirror(curve) | Element::Screen(curve) => {
                    offer(index, curve.hits(ray.origin, ray.direction));
                }
                Element::Glass { outline, .. } => {
                    for curve in outline {
                        offer(index, curve.hits(ray.origin, ray.direction));
                    }
                }
                Element::ThinLens {
                    centre,
                    axis,
                    aperture,
                    ..
                } => {
                    let reach = axis.perp() * *aperture;
                    let plane = Curve::Segment {
                        a: *centre - reach,
                        b: *centre + reach,
                    };
                    offer(index, plane.hits(ray.origin, ray.direction));
                }
            }
        }
        best
    }

    /// Follows a ray through everything on the bench until it leaves, is stopped, or has gone `reach` since its
    /// last turn.
    pub fn trace(&self, ray: Ray, reach: f64) -> Path {
        let mut ray = Ray::new(ray.origin, ray.direction);
        let first = ray.direction;
        let mut points = vec![ray.origin];
        let mut events = Vec::new();
        let mut end = End::Exhausted;
        for _ in 0..MEETINGS {
            let Some((index, hit)) = self.nearest(ray).filter(|(_, h)| h.distance <= reach) else {
                points.push(ray.origin + ray.direction * reach);
                end = End::Escaped;
                break;
            };
            let at = ray.origin + ray.direction * hit.distance;
            // The normal on the side the ray arrives from.
            let normal = if hit.normal.dot(ray.direction) > 0.0 {
                -hit.normal
            } else {
                hit.normal
            };
            let cos_in = -ray.direction.dot(normal);
            let incidence = cos_in.clamp(-1.0, 1.0).acos();
            let reflected = ray.direction + normal * (2.0 * cos_in);
            points.push(at);
            let (kind, direction) = match &self.elements[index] {
                Element::Mirror(_) => (EventKind::Reflection, reflected),
                Element::Screen(_) => (EventKind::Absorbed, ray.direction),
                Element::Glass { .. } => {
                    let from = self.index_at(at - ray.direction * PROBE);
                    let into = self.index_at(at + ray.direction * PROBE);
                    let ratio = from / into;
                    // Snell's law, n₁ sin i = n₂ sin r, in vector form.
                    let sin_out_squared = ratio * ratio * (1.0 - cos_in * cos_in);
                    if sin_out_squared > 1.0 {
                        (EventKind::TotalReflection, reflected)
                    } else {
                        let cos_out = (1.0 - sin_out_squared).sqrt();
                        (
                            EventKind::Refraction,
                            ray.direction * ratio + normal * (ratio * cos_in - cos_out),
                        )
                    }
                }
                Element::ThinLens {
                    centre,
                    axis,
                    focal,
                    ..
                } => {
                    let forward = if ray.direction.dot(*axis) >= 0.0 {
                        *axis
                    } else {
                        -*axis
                    };
                    let across = axis.perp();
                    let height = (at - *centre).dot(across);
                    let slope = ray.direction.dot(across) / ray.direction.dot(forward);
                    (EventKind::Lens, forward + across * (slope - height / focal))
                }
            };
            let direction = direction.normalized();
            events.push(Event {
                at,
                kind,
                normal,
                incidence,
                emergence: direction.dot(normal).abs().clamp(0.0, 1.0).acos(),
            });
            ray = Ray {
                origin: at,
                direction,
            };
            if kind == EventKind::Absorbed {
                end = End::Absorbed;
                break;
            }
        }
        Path {
            points,
            events,
            end,
            first,
            last: ray.direction,
        }
    }

    /// The image of the point `object`, formed by rays that leave it close to the line towards `towards`: the
    /// paraxial image, the one the mirror and lens formulas are about. Two rays a hair either side of that line
    /// are followed through everything, and the image is where their last stretches cross. `None` when they
    /// meet nothing, or leave parallel and the image is at infinity.
    pub fn image(&self, object: Vec2, towards: Vec2, reach: f64) -> Option<Image> {
        let aim = towards - object;
        let hair = aim.perp() * 1e-5;
        let paths = [-1.0, 1.0].map(|side| self.trace(Ray::new(object, aim + hair * side), reach));
        if paths.iter().any(|p| p.events.is_empty()) {
            return None;
        }
        crossing(&paths[0], &paths[1])
    }
}

/// Where the last stretches of two rays cross, and whether they do so ahead of both (a real crossing) or only
/// when produced backwards.
pub fn crossing(a: &Path, b: &Path) -> Option<Image> {
    let (p, q) = (a.last_turn(), b.last_turn());
    let across = a.last.cross(b.last);
    if across.abs() < 1e-12 {
        return None;
    }
    let along_a = (q - p).cross(b.last) / across;
    let along_b = (q - p).cross(a.last) / across;
    Some(Image {
        at: p + a.last * along_a,
        real: along_a > 0.0 && along_b > 0.0,
    })
}
