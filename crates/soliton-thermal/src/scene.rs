//! A thermal experiment as data: the box, what is put in it, what is then done to it stage by stage, and the
//! textbook answers with how each is read off the molecules. Lengths in nm, times in ps, masses in amu,
//! temperatures in K.

use crate::gas::{Gas, Molecule, Wall, WALLS};
use crate::{ATMOSPHERE, BOLTZMANN, JOULES, METRES_PER_SECOND, PASCALS};
use serde::{Deserialize, Serialize};

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

fn is_false(v: &bool) -> bool {
    !*v
}

fn yes() -> bool {
    true
}

fn is_true(v: &bool) -> bool {
    *v
}

/// Molecules of one kind put into the box together.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupSpec {
    /// What they are called: "argon", "the grain".
    pub name: String,
    pub count: usize,
    pub mass: f64,
    pub radius: f64,
    /// They start with velocities drawn for this temperature, then scaled so that it is exactly theirs.
    pub temperature: f64,
    /// The part of the box's length they start in, as two fractions of it; all of it unless given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<[f64; 2]>,
    /// All start at one speed, in random directions: as far from equilibrium as the same energy can be.
    #[serde(default, skip_serializing_if = "is_false")]
    pub equal_speeds: bool,
}

/// What the walls do during a stage.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WallsSpec {
    /// No heat passes.
    Insulated,
    /// Every wall held at this temperature.
    Bath(f64),
    /// The two ends held at different temperatures, the sides insulated.
    Ends { left: f64, right: f64 },
}

/// One thing done to the gas. A stage lasts for `duration` if it has one, and until the piston has arrived if
/// it is sent somewhere; the last stage of a scene may have neither and then never ends.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageSpec {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    /// Length of the box the piston is sent to, nm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub piston_to: Option<f64>,
    /// nm/ps. Zero with `piston_to` set moves the piston there at once, as when a wall is suddenly taken away.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub piston_speed: f64,
    pub walls: WallsSpec,
    #[serde(default, skip_serializing_if = "is_false")]
    pub remove_partition: bool,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

impl StageSpec {
    pub fn new(label: &str, walls: WallsSpec) -> Self {
        Self {
            label: label.to_string(),
            duration: None,
            piston_to: None,
            piston_speed: 0.0,
            walls,
            remove_partition: false,
        }
    }

    pub fn lasting(mut self, duration: f64) -> Self {
        self.duration = Some(duration);
        self
    }

    pub fn piston(mut self, to: f64, speed: f64) -> Self {
        self.piston_to = Some(to);
        self.piston_speed = speed;
        self
    }

    pub fn without_partition(mut self) -> Self {
        self.remove_partition = true;
        self
    }
}

/// How an answer is read off the gas, during the stage the answer names. Temperatures in K, pressures in
/// atmospheres, speeds in m/s, energies in units of 10⁻²¹ J.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasureSpec {
    /// The temperature now, or as it was when the stage ended.
    Temperature,
    /// The temperature averaged over the stage. A gas of a few hundred molecules in contact with a reservoir
    /// has an energy that flickers by several per cent; this is its steady value.
    MeanTemperature,
    /// The temperature of one kind of molecule, averaged over the stage.
    SpeciesTemperature {
        species: u8,
    },
    /// Blows on the walls per unit area and time, averaged over the stage.
    Pressure,
    /// Averaged over the stage and over the molecules of one kind.
    MeanSpeed {
        species: u8,
    },
    RmsSpeed {
        species: u8,
    },
    /// Work done on the gas by the piston during the stage.
    Work,
    /// Heat given to the gas by the walls during the stage; negative when the gas gave it up.
    Heat,
    /// Mean kinetic energy of a molecule, now or at the stage's end.
    EnergyPerMolecule,
    /// Share of one kind of molecule found in the left half of the box, averaged over the stage.
    ShareOnLeft {
        species: u8,
    },
    /// The efficiency of a cycle whose heat is taken in during the stage `hot` and given out during the stage
    /// `cold`, both at fixed temperatures: one minus the work done on the gas at the cold reservoir over the
    /// work it did at the hot one. At a fixed temperature an ideal gas turns heat into work one for one, so
    /// those two works are the two heats, and they are known far more exactly than the heats themselves,
    /// which carry the flicker of the gas's own energy at each end of a stroke. Given once `cold` has ended.
    Efficiency {
        hot: usize,
        cold: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerSpec {
    pub name: String,
    pub value: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measure: Option<MeasureSpec>,
    /// The stage the measurement belongs to; the last one unless given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<usize>,
    /// How close, as a fraction, the gas has to come to count as agreeing: a few hundred molecules fluctuate.
    #[serde(default = "tolerance")]
    pub tolerance: f64,
}

fn tolerance() -> f64 {
    0.03
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
pub struct ThermalScene {
    pub schema: u32,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub syllabus: Option<SyllabusSpec>,
    /// The box, nm. The piston is its far end along x.
    pub size: [f64; 3],
    pub groups: Vec<GroupSpec>,
    /// A wall across the box at this x that keeps each molecule to its side until a stage removes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition: Option<f64>,
    /// Whether molecules bounce off one another.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub collisions: bool,
    /// Time step, ps.
    pub dt: f64,
    /// Picoseconds of the gas shown in one second of watching.
    pub playback: f64,
    pub seed: u64,
    pub stages: Vec<StageSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub answers: Vec<AnswerSpec>,
}

/// What happened during one stage, and running totals from which its averages come.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StageRecord {
    pub started: f64,
    pub ended: Option<f64>,
    /// Work on the gas and heat into it during the stage.
    pub work: f64,
    pub heat: f64,
    /// Blows on all walls, and the wall area they landed on integrated over time.
    impulse: f64,
    exposure: f64,
    samples: usize,
    temperature: f64,
    /// Per species: sums of temperature, mean speed, mean squared speed and share on the left.
    sums: Vec<[f64; 4]>,
    /// As the stage ended.
    pub final_temperature: f64,
    pub final_volume: f64,
}

/// One row of the record kept for graphs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub time: f64,
    /// nm³.
    pub volume: f64,
    /// atm, averaged over the last forty picoseconds.
    pub pressure: f64,
    /// K.
    pub temperature: f64,
    /// Running totals since the start, 10⁻²¹ J.
    pub work: f64,
    pub heat: f64,
    pub energy: f64,
}

/// A scene being run.
#[derive(Clone, Debug)]
pub struct Experiment {
    pub scene: ThermalScene,
    pub gas: Gas,
    stage: usize,
    finished: bool,
    pub records: Vec<StageRecord>,
    pub samples: Vec<Sample>,
    /// For each kind of molecule, the share of it in the left half of the box at each sample.
    pub shares: Vec<Vec<f64>>,
    /// The middle of the box at its longest: "the left half" means the same place throughout.
    middle: f64,
    // Totals as they were at the last sample, and a short history of impulse for the pressure shown live.
    seen: Totals,
    recent: Vec<(f64, f64)>,
    until_sample: f64,
}

#[derive(Clone, Copy, Debug, Default)]
struct Totals {
    work: f64,
    heat: f64,
    impulse: f64,
}

/// Picoseconds between samples, and how many of them the live pressure is averaged over.
const SAMPLE: f64 = 1.0;
const WINDOW: usize = 40;

impl ThermalScene {
    pub fn from_json(text: &str) -> Result<Self, SceneError> {
        let scene: Self = serde_json::from_str(text)
            .map_err(|e| SceneError(format!("invalid scene JSON: {e}")))?;
        scene.start()?;
        Ok(scene)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("a thermal scene always serialises")
    }

    fn check(&self) -> Result<(), SceneError> {
        if self.schema != SCHEMA {
            return err(format!("unsupported scene schema {}", self.schema));
        }
        if self.size.iter().any(|s| s.is_nan() || *s <= 0.0) {
            return err("size: the box needs three positive lengths");
        }
        if self.dt.is_nan() || self.dt <= 0.0 {
            return err("dt must be positive");
        }
        if self.playback.is_nan() || self.playback <= 0.0 {
            return err("playback must be positive");
        }
        if self.stages.is_empty() {
            return err("a scene needs at least one stage");
        }
        for group in &self.groups {
            let name = &group.name;
            if group.mass.is_nan() || group.mass <= 0.0 {
                return err(format!("{name}: mass must be positive"));
            }
            if group.radius.is_nan() || group.radius < 0.0 {
                return err(format!("{name}: radius cannot be negative"));
            }
            if group.temperature.is_nan() || group.temperature < 0.0 {
                return err(format!("{name}: temperature cannot be below absolute zero"));
            }
            if let Some([from, to]) = group.region {
                if !(0.0..=1.0).contains(&from) || !(0.0..=1.0).contains(&to) || from >= to {
                    return err(format!(
                        "{name}: a region is two fractions of the box, in order"
                    ));
                }
            }
        }
        if self.groups.len() > 8 {
            return err("at most eight kinds of molecule");
        }
        for stage in &self.stages {
            if stage.piston_to.is_some_and(|l| l.is_nan() || l <= 0.0) {
                return err(format!(
                    "{}: the piston needs somewhere positive to go",
                    stage.label
                ));
            }
            let cold = |t: f64| t.is_nan() || t < 0.0;
            let bad = match stage.walls {
                WallsSpec::Insulated => false,
                WallsSpec::Bath(t) => cold(t),
                WallsSpec::Ends { left, right } => cold(left) || cold(right),
            };
            if bad {
                return err(format!(
                    "{}: a wall cannot be below absolute zero",
                    stage.label
                ));
            }
        }
        for answer in &self.answers {
            if answer.stage.is_some_and(|s| s >= self.stages.len()) {
                return err(format!("`{}` names a stage that is not there", answer.name));
            }
        }
        Ok(())
    }

    /// Fills the box and makes ready to run.
    pub fn start(&self) -> Result<Experiment, SceneError> {
        self.check()?;
        let mut gas = Gas::new(self.size, self.seed);
        gas.partition = self.partition;
        gas.collisions = self.collisions;
        for (species, group) in self.groups.iter().enumerate() {
            place(&mut gas, group, species as u8, self.partition)?;
        }
        let mut experiment = Experiment {
            scene: self.clone(),
            gas,
            stage: 0,
            finished: false,
            records: Vec::new(),
            samples: Vec::new(),
            shares: vec![Vec::new(); self.groups.len()],
            middle: 0.5
                * self
                    .stages
                    .iter()
                    .filter_map(|s| s.piston_to)
                    .fold(self.size[0], f64::max),
            seen: Totals::default(),
            recent: Vec::new(),
            until_sample: 0.0,
        };
        experiment.enter(0);
        experiment.sample();
        Ok(experiment)
    }
}

/// Puts a group's molecules where none overlaps another, with velocities for its temperature.
fn place(
    gas: &mut Gas,
    group: &GroupSpec,
    species: u8,
    partition: Option<f64>,
) -> Result<(), SceneError> {
    let [from, to] = group.region.unwrap_or([0.0, 1.0]);
    let size = gas.size;
    let first = gas.molecules.len();
    let r = group.radius;
    for _ in 0..group.count {
        let mut placed = false;
        for _ in 0..4000 {
            let mut position = [0.0; 3];
            for axis in 0..3 {
                let (mut low, mut high) = (r, size[axis] - r);
                if axis == 0 {
                    low = low.max(from * size[0] + r);
                    high = high.min(to * size[0] - r);
                    // Clear of the partition, on whichever side this group is.
                    if let Some(at) = partition {
                        if to * size[0] <= at + 1e-9 {
                            high = high.min(at - r);
                        } else if from * size[0] >= at - 1e-9 {
                            low = low.max(at + r);
                        }
                    }
                }
                if high < low {
                    return err(format!(
                        "{}: there is no room for a molecule this size",
                        group.name
                    ));
                }
                position[axis] = low + (high - low) * gas.rng().uniform();
            }
            let clear = gas.molecules.iter().all(|other| {
                let apart: f64 = (0..3)
                    .map(|a| (other.position[a] - position[a]).powi(2))
                    .sum();
                apart >= (other.radius + r).powi(2)
            });
            if clear {
                gas.molecules.push(Molecule {
                    position,
                    velocity: [0.0; 3],
                    mass: group.mass,
                    radius: r,
                    species,
                });
                placed = true;
                break;
            }
        }
        if !placed {
            return err(format!(
                "{}: the box is too full to fit them all in",
                group.name
            ));
        }
    }
    let spread = (BOLTZMANN * group.temperature / group.mass).sqrt();
    for i in first..gas.molecules.len() {
        let velocity = if group.equal_speeds {
            gas.rng().direction().map(|u| u * spread * 3f64.sqrt())
        } else {
            [0.0; 3].map(|_: f64| spread * gas.rng().normal())
        };
        gas.molecules[i].velocity = velocity;
    }
    // Scaled so the group's temperature is the one asked for, not merely near it.
    if let Some(found) = gas
        .temperature_of(|m| m.species == species)
        .filter(|t| *t > 0.0)
    {
        let scale = (group.temperature / found).sqrt();
        for m in gas.molecules[first..].iter_mut() {
            m.velocity = m.velocity.map(|v| v * scale);
        }
    }
    Ok(())
}

impl Experiment {
    pub fn stage(&self) -> usize {
        self.stage
    }

    /// True once the last stage has ended, for a scene whose last stage has an end.
    pub fn finished(&self) -> bool {
        self.finished
    }

    fn totals(&self) -> Totals {
        Totals {
            work: self.gas.work,
            heat: self.gas.heat.iter().sum(),
            impulse: self.gas.impulse.iter().sum(),
        }
    }

    fn enter(&mut self, stage: usize) {
        let spec = self.scene.stages[stage].clone();
        self.stage = stage;
        self.gas.walls = match spec.walls {
            WallsSpec::Insulated => [Wall::Insulating; WALLS],
            WallsSpec::Bath(t) => [Wall::Bath(t); WALLS],
            WallsSpec::Ends { left, right } => {
                let mut walls = [Wall::Insulating; WALLS];
                walls[0] = Wall::Bath(left);
                walls[1] = Wall::Bath(right);
                walls
            }
        };
        if spec.remove_partition {
            self.gas.partition = None;
        }
        if let Some(to) = spec.piston_to {
            if spec.piston_speed == 0.0 {
                // Taken away at once: no molecule is touching it as it goes, so no work is done.
                self.gas.size[0] = to;
                self.gas.piston_target = to;
            } else {
                self.gas.drive_piston(to, spec.piston_speed);
            }
        }
        self.records.push(StageRecord {
            started: self.gas.time,
            sums: vec![[0.0; 4]; self.scene.groups.len()],
            ..StageRecord::default()
        });
    }

    fn stage_over(&self) -> bool {
        let spec = &self.scene.stages[self.stage];
        if spec.duration.is_none() && spec.piston_to.is_none() {
            return false;
        }
        let record = &self.records[self.stage];
        let timed_out = spec
            .duration
            .is_none_or(|d| self.gas.time - record.started >= d - 1e-9);
        timed_out && self.gas.piston_speed == 0.0
    }

    /// Runs the gas on by `picoseconds`, moving from stage to stage as each ends.
    pub fn advance(&mut self, picoseconds: f64) {
        let dt = self.scene.dt;
        let steps = (picoseconds / dt).round().max(0.0) as usize;
        for _ in 0..steps {
            if self.finished {
                return;
            }
            let surface = self.gas.surface();
            self.gas.step(dt);
            self.records[self.stage].exposure += surface * dt;
            self.until_sample -= dt;
            if self.until_sample <= 1e-9 {
                self.sample();
            }
            if self.stage_over() {
                self.settle();
                let record = &mut self.records[self.stage];
                record.ended = Some(self.gas.time);
                record.final_temperature = self.gas.temperature();
                record.final_volume = self.gas.volume();
                if self.stage + 1 < self.scene.stages.len() {
                    self.enter(self.stage + 1);
                } else {
                    self.finished = true;
                }
            }
        }
    }

    /// Books what has happened since the last look to the stage it happened in.
    fn settle(&mut self) -> Totals {
        let now = self.totals();
        let record = &mut self.records[self.stage];
        record.work += now.work - self.seen.work;
        record.heat += now.heat - self.seen.heat;
        record.impulse += now.impulse - self.seen.impulse;
        self.seen = now;
        now
    }

    fn sample(&mut self) {
        self.until_sample = SAMPLE;
        let now = self.settle();
        let gas = &self.gas;
        let half = self.middle;
        let record = &mut self.records[self.stage];
        record.samples += 1;
        record.temperature += gas.temperature();
        for (species, sums) in record.sums.iter_mut().enumerate() {
            let (mut energy, mut speed, mut squared, mut left, mut count) =
                (0.0, 0.0, 0.0, 0.0, 0.0);
            for m in gas
                .molecules
                .iter()
                .filter(|m| m.species as usize == species)
            {
                let v2 = m.speed_squared();
                energy += 0.5 * m.mass * v2;
                speed += v2.sqrt();
                squared += v2;
                left += f64::from(m.position[0] < half);
                count += 1.0;
            }
            if count > 0.0 {
                sums[0] += 2.0 * energy / (3.0 * count * BOLTZMANN);
                sums[1] += speed / count;
                sums[2] += squared / count;
                sums[3] += left / count;
            }
            self.shares[species].push(if count > 0.0 { left / count } else { 0.0 });
        }
        // Pressure for the graph: blows on the walls over the last few samples.
        self.recent.push((gas.time, now.impulse));
        if self.recent.len() > WINDOW + 1 {
            self.recent.remove(0);
        }
        let (then, before) = self.recent[0];
        let pressure = if gas.time > then {
            (now.impulse - before) / ((gas.time - then) * gas.surface())
        } else {
            0.0
        };
        self.samples.push(Sample {
            time: gas.time,
            volume: gas.volume(),
            pressure: pressure * PASCALS / ATMOSPHERE,
            temperature: gas.temperature(),
            work: now.work * JOULES * 1e21,
            heat: now.heat * JOULES * 1e21,
            energy: gas.kinetic_energy() * JOULES * 1e21,
        });
    }

    /// Each answer's name, textbook value, what the gas gives for it so far, and the tolerance within which the
    /// two count as agreeing.
    pub fn answers(&self) -> Vec<(String, f64, Option<f64>, f64)> {
        self.scene
            .answers
            .iter()
            .map(|answer| {
                let stage = answer.stage.unwrap_or(self.scene.stages.len() - 1);
                let measured = answer.measure.and_then(|m| self.measure(m, stage));
                (
                    answer.name.clone(),
                    answer.value,
                    measured,
                    answer.tolerance,
                )
            })
            .collect()
    }

    fn measure(&self, measure: MeasureSpec, stage: usize) -> Option<f64> {
        let record = self.records.get(stage)?;
        let current = stage == self.stage && record.ended.is_none();
        // What has happened in the present stage since the last sample is not yet in its record.
        let now = self.totals();
        let extra = |total: f64, seen: f64| if current { total - seen } else { 0.0 };
        let average = |species: u8, which: usize| {
            let sums = record.sums.get(species as usize)?;
            (record.samples > 0).then(|| sums[which] / record.samples as f64)
        };
        let energy = JOULES * 1e21;
        Some(match measure {
            MeasureSpec::Temperature => {
                if current {
                    self.gas.temperature()
                } else {
                    record.final_temperature
                }
            }
            MeasureSpec::EnergyPerMolecule => {
                let t = if current {
                    self.gas.temperature()
                } else {
                    record.final_temperature
                };
                1.5 * BOLTZMANN * t * energy
            }
            MeasureSpec::MeanTemperature => {
                if record.samples == 0 {
                    return None;
                }
                record.temperature / record.samples as f64
            }
            MeasureSpec::SpeciesTemperature { species } => average(species, 0)?,
            MeasureSpec::MeanSpeed { species } => average(species, 1)? * METRES_PER_SECOND,
            MeasureSpec::RmsSpeed { species } => average(species, 2)?.sqrt() * METRES_PER_SECOND,
            MeasureSpec::ShareOnLeft { species } => average(species, 3)?,
            MeasureSpec::Pressure => {
                let impulse = record.impulse + extra(now.impulse, self.seen.impulse);
                if record.exposure <= 0.0 {
                    return None;
                }
                impulse / record.exposure * PASCALS / ATMOSPHERE
            }
            MeasureSpec::Work => (record.work + extra(now.work, self.seen.work)) * energy,
            MeasureSpec::Heat => (record.heat + extra(now.heat, self.seen.heat)) * energy,
            MeasureSpec::Efficiency { hot, cold } => {
                let (taken_in, given_out) = (-self.records.get(hot)?.work, self.records.get(cold)?);
                given_out.ended?;
                if taken_in <= 0.0 {
                    return None;
                }
                1.0 - given_out.work / taken_in
            }
        })
    }

    /// How the molecules of one kind are spread over speed, from zero to `fastest` m/s in `bins` equal steps,
    /// as a probability density per m/s. All kinds together when `species` is `None`.
    pub fn speed_histogram(&self, species: Option<u8>, bins: usize, fastest: f64) -> Vec<f64> {
        let mut counts = vec![0.0; bins];
        let mut total = 0.0;
        for m in &self.gas.molecules {
            if species.is_some_and(|s| s != m.species) {
                continue;
            }
            total += 1.0;
            let speed = m.speed_squared().sqrt() * METRES_PER_SECOND;
            let bin = (speed / fastest * bins as f64) as usize;
            if bin < bins {
                counts[bin] += 1.0;
            }
        }
        let width = fastest / bins as f64;
        if total > 0.0 {
            for c in &mut counts {
                *c /= total * width;
            }
        }
        counts
    }
}

/// Maxwell's distribution of speeds: the probability density, per m/s, of a molecule of `mass` amu moving at
/// `speed` m/s in a gas at `temperature` K.
pub fn maxwell(mass: f64, temperature: f64, speed: f64) -> f64 {
    if temperature <= 0.0 {
        return 0.0;
    }
    let v = speed / METRES_PER_SECOND;
    let a = mass / (2.0 * BOLTZMANN * temperature);
    4.0 * std::f64::consts::PI * (a / std::f64::consts::PI).powf(1.5) * v * v * (-a * v * v).exp()
        / METRES_PER_SECOND
}
