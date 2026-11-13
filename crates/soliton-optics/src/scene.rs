//! An optical bench as data: what is on it, the rays sent through it, the objects whose images are wanted, and
//! the textbook answers with how each is read off the traced rays. Lengths are in metres, angles in radians.

use crate::bench::{crossing, Bench, Element, Image, Path, Ray};
use crate::curve::Curve;
use serde::{Deserialize, Serialize};
use soliton_core::Vec2;

pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct SceneError(pub String);

impl std::fmt::Display for SceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SceneError {}

fn err<T>(message: impl Into<String>) -> Result<T, SceneError> {
    Err(SceneError(message.into()))
}

fn v(p: [f64; 2]) -> Vec2 {
    Vec2::new(p[0], p[1])
}

fn a(p: Vec2) -> [f64; 2] {
    [p.x, p.y]
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurveSpec {
    Segment {
        a: [f64; 2],
        b: [f64; 2],
    },
    /// Part of a circle from the angle `from`, turning through `sweep`.
    Arc {
        centre: [f64; 2],
        radius: f64,
        from: f64,
        sweep: f64,
    },
}

impl From<Curve> for CurveSpec {
    fn from(curve: Curve) -> Self {
        match curve {
            Curve::Segment { a: p, b: q } => Self::Segment { a: a(p), b: a(q) },
            Curve::Arc {
                centre,
                radius,
                from,
                sweep,
            } => Self::Arc {
                centre: a(centre),
                radius,
                from,
                sweep,
            },
        }
    }
}

impl CurveSpec {
    fn build(&self) -> Result<Curve, SceneError> {
        Ok(match *self {
            Self::Segment { a: p, b: q } => Curve::Segment { a: v(p), b: v(q) },
            Self::Arc {
                centre,
                radius,
                from,
                sweep,
            } => {
                if radius.is_nan() || radius <= 0.0 {
                    return err("an arc's radius must be positive");
                }
                Curve::Arc {
                    centre: v(centre),
                    radius,
                    from,
                    sweep,
                }
            }
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ElementSpec {
    Mirror {
        curve: CurveSpec,
    },
    Screen {
        curve: CurveSpec,
    },
    /// A closed outline filled with a material of refractive index `index`.
    Glass {
        outline: Vec<CurveSpec>,
        index: f64,
    },
    /// An ideal thin lens; `focal` is negative for a diverging one.
    ThinLens {
        centre: [f64; 2],
        axis: [f64; 2],
        focal: f64,
        aperture: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceSpec {
    /// One ray from `from` aimed through `through`. With `angles`, front ends mark the normal and the angles
    /// where it meets each surface.
    Ray {
        from: [f64; 2],
        through: [f64; 2],
        #[serde(default, skip_serializing_if = "is_false")]
        angles: bool,
        /// Drawn lightly: a ray that is there to be measured more than to be looked at.
        #[serde(default, skip_serializing_if = "is_false")]
        faint: bool,
    },
    /// `count` rays from a point, spread evenly `half_angle` either side of the line towards `towards`. A
    /// `faint` fan is background to a ray that is the subject.
    Fan {
        from: [f64; 2],
        towards: [f64; 2],
        half_angle: f64,
        count: usize,
        #[serde(default, skip_serializing_if = "is_false")]
        faint: bool,
    },
    /// `count` parallel rays travelling at `angle`, spread over `width` about the one through `centre`.
    Beam {
        centre: [f64; 2],
        angle: f64,
        width: f64,
        count: usize,
    },
}

fn is_false(v: &bool) -> bool {
    !*v
}

/// An upright object, drawn as an arrow from `base` to `tip`, with its base on the axis. Its image is the
/// paraxial one, found from rays that leave it towards `towards`: the pole of a mirror, or the centre of a lens.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectSpec {
    pub base: [f64; 2],
    pub tip: [f64; 2],
    pub towards: [f64; 2],
}

/// A named point to mark: a focus, a centre of curvature, a pole.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkSpec {
    pub at: [f64; 2],
    pub label: String,
}

/// How an answer is read off the traced rays. Lengths come out multiplied by `scale`, angles in degrees.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasureSpec {
    /// How far the image of an object's base is from `origin` along `axis`.
    ImagePosition {
        object: usize,
        origin: [f64; 2],
        axis: [f64; 2],
        scale: f64,
    },
    /// Height of the image over height of the object, negative when the image is upside down.
    Magnification { object: usize },
    /// The angle a ray makes with the normal as it arrives at its `event`-th meeting, counting from zero.
    Incidence { ray: usize, event: usize },
    /// The angle it makes with the normal as it leaves that meeting.
    Emergence { ray: usize, event: usize },
    /// The angle between the way a ray set out and the way it ends up going.
    Deviation { ray: usize },
    /// Where the last stretches of two rays cross, measured from `origin` along `axis`.
    Focus {
        rays: [usize; 2],
        origin: [f64; 2],
        axis: [f64; 2],
        scale: f64,
    },
    /// The slope a ray leaves with over the slope it arrived with, both taken against `axis`, without sign.
    AngularMagnification { ray: usize, axis: [f64; 2] },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerSpec {
    pub name: String,
    pub value: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure: Option<MeasureSpec>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewSpec {
    pub center: [f64; 2],
    pub half_width: f64,
    pub half_height: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyllabusSpec {
    pub class: u8,
    pub chapter: String,
    pub topic: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpticsScene {
    pub schema: u32,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub syllabus: Option<SyllabusSpec>,
    /// Refractive index of the space between the elements; air unless given.
    #[serde(default = "one")]
    pub ambient: f64,
    /// How far a ray is followed after its last turn, m.
    #[serde(default = "far")]
    pub reach: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elements: Vec<ElementSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<SourceSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objects: Vec<ObjectSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<MarkSpec>,
    /// The principal axis, as two points on it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axis: Option<[[f64; 2]; 2]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub answers: Vec<AnswerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<ViewSpec>,
}

/// The share of an object's height used to find its magnification near the axis.
const SMALL: f64 = 1e-3;

fn one() -> f64 {
    1.0
}

fn far() -> f64 {
    5.0
}

/// One ray of a scene, traced.
#[derive(Clone, Debug, PartialEq)]
pub struct Traced {
    pub path: Path,
    /// Whether front ends should mark its angles.
    pub angles: bool,
    /// Drawn lightly, behind the rays that matter.
    pub faint: bool,
}

/// The image of an object: of its base and of its tip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObjectImage {
    pub base: Image,
    pub tip: Image,
}

/// A scene built and traced: everything a front end draws and every answer measured.
#[derive(Clone, Debug)]
pub struct Traces {
    pub bench: Bench,
    pub rays: Vec<Traced>,
    /// `None` for an object whose image is at infinity.
    pub images: Vec<Option<ObjectImage>>,
    /// Each answer's name, textbook value, and what the traced rays give for it where they can.
    pub answers: Vec<(String, f64, Option<f64>)>,
}

impl OpticsScene {
    pub fn new(title: &str) -> Self {
        Self {
            schema: SCHEMA,
            title: title.to_string(),
            description: None,
            syllabus: None,
            ambient: 1.0,
            reach: far(),
            elements: Vec::new(),
            sources: Vec::new(),
            objects: Vec::new(),
            marks: Vec::new(),
            axis: None,
            answers: Vec::new(),
            view: None,
        }
    }

    pub fn from_json(text: &str) -> Result<Self, SceneError> {
        let scene: Self = serde_json::from_str(text)
            .map_err(|e| SceneError(format!("invalid scene JSON: {e}")))?;
        scene.trace()?;
        Ok(scene)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("an optics scene always serialises")
    }

    pub fn element(mut self, element: ElementSpec) -> Self {
        self.elements.push(element);
        self
    }

    pub fn mirror(self, curve: Curve) -> Self {
        self.element(ElementSpec::Mirror {
            curve: curve.into(),
        })
    }

    pub fn glass(self, outline: Vec<Curve>, index: f64) -> Self {
        self.element(ElementSpec::Glass {
            outline: outline.into_iter().map(CurveSpec::from).collect(),
            index,
        })
    }

    /// An ideal thin lens across the x axis at `x`.
    pub fn thin_lens(self, x: f64, focal: f64, aperture: f64) -> Self {
        self.element(ElementSpec::ThinLens {
            centre: [x, 0.0],
            axis: [1.0, 0.0],
            focal,
            aperture,
        })
    }

    pub fn source(mut self, source: SourceSpec) -> Self {
        self.sources.push(source);
        self
    }

    pub fn ray(self, from: [f64; 2], through: [f64; 2]) -> Self {
        self.source(SourceSpec::Ray {
            from,
            through,
            angles: false,
            faint: false,
        })
    }

    pub fn object(mut self, base: [f64; 2], tip: [f64; 2], towards: [f64; 2]) -> Self {
        self.objects.push(ObjectSpec { base, tip, towards });
        self
    }

    pub fn mark(mut self, at: [f64; 2], label: &str) -> Self {
        self.marks.push(MarkSpec {
            at,
            label: label.to_string(),
        });
        self
    }

    pub fn axis(mut self, from: [f64; 2], to: [f64; 2]) -> Self {
        self.axis = Some([from, to]);
        self
    }

    pub fn answer(mut self, name: &str, value: f64) -> Self {
        self.answers.push(AnswerSpec {
            name: name.to_string(),
            value,
            measure: None,
        });
        self
    }

    pub fn measured(mut self, name: &str, value: f64, measure: MeasureSpec) -> Self {
        self.answers.push(AnswerSpec {
            name: name.to_string(),
            value,
            measure: Some(measure),
        });
        self
    }

    pub fn view(mut self, center: [f64; 2], half_width: f64, half_height: f64) -> Self {
        self.view = Some(ViewSpec {
            center,
            half_width,
            half_height,
        });
        self
    }

    pub fn bench(&self) -> Result<Bench, SceneError> {
        if self.schema != SCHEMA {
            return err(format!("unsupported scene schema {}", self.schema));
        }
        if self.ambient.is_nan() || self.ambient <= 0.0 {
            return err("ambient: a refractive index must be positive");
        }
        if self.reach.is_nan() || self.reach <= 0.0 {
            return err("reach must be positive");
        }
        let mut elements = Vec::new();
        for spec in &self.elements {
            elements.push(match spec {
                ElementSpec::Mirror { curve } => Element::Mirror(curve.build()?),
                ElementSpec::Screen { curve } => Element::Screen(curve.build()?),
                ElementSpec::Glass { outline, index } => {
                    if index.is_nan() || *index <= 0.0 {
                        return err("glass: a refractive index must be positive");
                    }
                    if outline.is_empty() {
                        return err("glass: an outline needs at least one curve");
                    }
                    Element::Glass {
                        outline: outline
                            .iter()
                            .map(CurveSpec::build)
                            .collect::<Result<_, _>>()?,
                        index: *index,
                    }
                }
                ElementSpec::ThinLens {
                    centre,
                    axis,
                    focal,
                    aperture,
                } => {
                    let axis = v(*axis).normalized();
                    if axis == Vec2::ZERO {
                        return err("thin_lens: the axis needs a direction");
                    }
                    if !focal.is_finite() || *focal == 0.0 {
                        return err("thin_lens: the focal length must be a number other than zero");
                    }
                    if aperture.is_nan() || *aperture <= 0.0 {
                        return err("thin_lens: the aperture must be positive");
                    }
                    Element::ThinLens {
                        centre: v(*centre),
                        axis,
                        focal: *focal,
                        aperture: *aperture,
                    }
                }
            });
        }
        Ok(Bench {
            elements,
            ambient: self.ambient,
        })
    }

    /// Every ray the sources send out, in order, with whether its angles are to be marked and whether it is faint.
    fn rays(&self) -> Result<Vec<(Ray, bool, bool)>, SceneError> {
        let mut rays = Vec::new();
        for source in &self.sources {
            match *source {
                SourceSpec::Ray {
                    from,
                    through,
                    angles,
                    faint,
                } => {
                    if from == through {
                        return err("ray: `from` and `through` must differ");
                    }
                    rays.push((Ray::towards(v(from), v(through)), angles, faint));
                }
                SourceSpec::Fan {
                    from,
                    towards,
                    half_angle,
                    count,
                    faint,
                } => {
                    let middle = (v(towards) - v(from)).angle();
                    for k in 0..count {
                        let share = if count > 1 {
                            2.0 * k as f64 / (count - 1) as f64 - 1.0
                        } else {
                            0.0
                        };
                        let direction = Vec2::from_polar(1.0, middle + share * half_angle);
                        rays.push((Ray::new(v(from), direction), false, faint));
                    }
                }
                SourceSpec::Beam {
                    centre,
                    angle,
                    width,
                    count,
                } => {
                    let direction = Vec2::from_polar(1.0, angle);
                    for k in 0..count {
                        let share = if count > 1 {
                            k as f64 / (count - 1) as f64 - 0.5
                        } else {
                            0.0
                        };
                        let origin = v(centre) + direction.perp() * (share * width);
                        rays.push((Ray::new(origin, direction), false, false));
                    }
                }
            }
        }
        Ok(rays)
    }

    /// Builds the bench, follows every ray, finds every image and reads off every answer.
    pub fn trace(&self) -> Result<Traces, SceneError> {
        let bench = self.bench()?;
        let rays: Vec<Traced> = self
            .rays()?
            .into_iter()
            .map(|(ray, angles, faint)| Traced {
                path: bench.trace(ray, self.reach),
                angles,
                faint,
            })
            .collect();
        let images: Vec<Option<ObjectImage>> = self
            .objects
            .iter()
            .map(|o| {
                let (foot, height) = (v(o.base), v(o.tip) - v(o.base));
                let base = bench.image(foot, v(o.towards), self.reach)?;
                // The formulas are about an object of vanishing height. A point a little way up the object
                // gives the magnification in that limit, and the image of the tip is drawn to that scale.
                let near = bench.image(foot + height * SMALL, v(o.towards), self.reach)?;
                let tip = Image {
                    at: base.at + (near.at - base.at) / SMALL,
                    real: base.real,
                };
                Some(ObjectImage { base, tip })
            })
            .collect();
        let mut answers = Vec::new();
        for answer in &self.answers {
            let measured = match &answer.measure {
                Some(measure) => self.measure(measure, &rays, &images)?,
                None => None,
            };
            answers.push((answer.name.clone(), answer.value, measured));
        }
        Ok(Traces {
            bench,
            rays,
            images,
            answers,
        })
    }

    fn measure(
        &self,
        measure: &MeasureSpec,
        rays: &[Traced],
        images: &[Option<ObjectImage>],
    ) -> Result<Option<f64>, SceneError> {
        let path = |index: usize| {
            rays.get(index).map(|t| &t.path).ok_or_else(|| {
                SceneError(format!(
                    "an answer measures ray {index}, which is not there"
                ))
            })
        };
        let image = |index: usize| {
            images.get(index).ok_or_else(|| {
                SceneError(format!(
                    "an answer measures object {index}, which is not there"
                ))
            })
        };
        let along = |at: Vec2, origin: [f64; 2], axis: [f64; 2], scale: f64| {
            (at - v(origin)).dot(v(axis).normalized()) * scale
        };
        Ok(match *measure {
            MeasureSpec::ImagePosition {
                object,
                origin,
                axis,
                scale,
            } => image(object)?.map(|i| along(i.base.at, origin, axis, scale)),
            MeasureSpec::Magnification { object } => {
                let found = image(object)?;
                let spec = &self.objects[object];
                let height = v(spec.tip) - v(spec.base);
                found.map(|i| (i.tip.at - i.base.at).dot(height) / height.length_squared())
            }
            MeasureSpec::Incidence { ray, event } => path(ray)?
                .events
                .get(event)
                .map(|e| e.incidence.to_degrees()),
            MeasureSpec::Emergence { ray, event } => path(ray)?
                .events
                .get(event)
                .map(|e| e.emergence.to_degrees()),
            MeasureSpec::Deviation { ray } => Some(path(ray)?.deviation().to_degrees()),
            MeasureSpec::Focus {
                rays: [first, second],
                origin,
                axis,
                scale,
            } => crossing(path(first)?, path(second)?).map(|c| along(c.at, origin, axis, scale)),
            MeasureSpec::AngularMagnification { ray, axis } => {
                let axis = v(axis).normalized();
                let path = path(ray)?;
                let slope = |d: Vec2| d.cross(axis) / d.dot(axis);
                let arriving = slope(path.first);
                (arriving != 0.0).then(|| (slope(path.last) / arriving).abs())
            }
        })
    }
}
