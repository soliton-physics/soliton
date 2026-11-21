//! Thermal properties of matter: expansion, heat and temperature, change of state, conduction, cooling and
//! radiation.

use crate::picture::{Answer, Lesson, Picture, Plot, Reading};
use soliton_modern::frame::{answer, curve, measured, plot, reading, said, Style};

/// Specific heats, J/(kg K), and latent heats, J/kg, of water in its three states.
pub const ICE: f64 = 2100.0;
pub const WATER: f64 = 4186.0;
pub const STEAM: f64 = 2010.0;
pub const FUSION: f64 = 3.34e5;
pub const VAPORISATION: f64 = 2.26e6;

/// A rod that is heated.
pub struct Expanding {
    /// Per kelvin; m; K.
    pub alpha: f64,
    pub length: f64,
    pub rise: f64,
}

impl Lesson for Expanding {
    fn still(&self) -> bool {
        true
    }

    fn picture(&self) -> Picture {
        Picture::Rod {
            share: self.alpha * self.rise,
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let share = self.alpha * self.rise;
        vec![
            reading("It grows longer by", share * self.length * 1e3, "mm"),
            reading("As a share of its length, α ΔT", 100.0 * share, "%"),
            reading("A plate of it gains in area, 2α ΔT", 200.0 * share, "%"),
            reading("A block of it gains in volume, 3α ΔT", 300.0 * share, "%"),
            // Young's modulus of steel.
            reading(
                "Stress if a steel rod were held from growing, Y α ΔT",
                2.0e11 * share / 1e6,
                "MPa",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let rises = [0.0, 200.0];
        let mut out = plot(
            "Expansion",
            "rise in temperature (K)",
            "gain in length (mm)",
            vec![curve(
                "α L ΔT",
                rises.map(|t| (t, self.alpha * self.length * t * 1e3)),
                Style::Solid,
                0,
            )],
        );
        out.marker = Some([self.rise, self.alpha * self.length * self.rise * 1e3]);
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        vec![
            answer(
                "Gain in length, α L ΔT (mm)",
                self.alpha * self.length * self.rise * 1e3,
            ),
            answer("Gain in volume, 3α ΔT (%)", 300.0 * self.alpha * self.rise),
        ]
    }
}

/// A hot block dropped into cooler water in a vessel that lets no heat out.
pub struct Mixing {
    /// Heat capacities, J/K.
    block_capacity: f64,
    water_capacity: f64,
    start: [f64; 2],
    block: f64,
    water: f64,
    time: f64,
    history: Vec<[f64; 3]>,
}

/// How readily heat passes from the block to the water, W/K, as a share of the smaller heat capacity a second.
const CONTACT: f64 = 0.35;

impl Mixing {
    /// Masses in kg, the block's specific heat in J/(kg K), temperatures in °C.
    pub fn new(
        block_mass: f64,
        specific_heat: f64,
        block: f64,
        water_mass: f64,
        water: f64,
    ) -> Self {
        Self {
            block_capacity: block_mass * specific_heat,
            water_capacity: water_mass * WATER,
            start: [block, water],
            block,
            water,
            time: 0.0,
            history: vec![[0.0, block, water]],
        }
    }

    /// Where the two temperatures meet: the heat one loses is the heat the other gains.
    pub fn final_temperature(&self) -> f64 {
        (self.block_capacity * self.start[0] + self.water_capacity * self.start[1])
            / (self.block_capacity + self.water_capacity)
    }

    pub fn boils(&self) -> bool {
        self.final_temperature() >= 100.0
    }

    fn settled(&self) -> bool {
        (self.block - self.water).abs() < 0.01
    }
}

impl Lesson for Mixing {
    fn advance(&mut self, dt: f64) {
        let conductance = CONTACT * self.block_capacity.min(self.water_capacity);
        let steps = (dt / 0.01).ceil().max(1.0) as usize;
        let h = dt / steps as f64;
        for _ in 0..steps {
            // Heat goes from the hotter to the cooler at a rate set by how far apart they are.
            let flow = conductance * (self.block - self.water) * h;
            self.block -= flow / self.block_capacity;
            self.water += flow / self.water_capacity;
        }
        self.time += dt;
        self.history.push([self.time, self.block, self.water]);
    }

    fn finished(&self) -> bool {
        self.settled()
    }

    fn clock(&self) -> String {
        format!("{:.1} s", self.time)
    }

    fn picture(&self) -> Picture {
        Picture::Mixing {
            block: self.block,
            water: self.water,
            hottest: self.start[0].max(self.start[1]),
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let mut out = vec![
            reading("Temperature of the block", self.block, "°C"),
            reading("Temperature of the water", self.water, "°C"),
            reading(
                "Heat the block has lost",
                self.block_capacity * (self.start[0] - self.block) / 1e3,
                "kJ",
            ),
            reading(
                "Heat the water has gained",
                self.water_capacity * (self.water - self.start[1]) / 1e3,
                "kJ",
            ),
        ];
        if self.boils() {
            out.push(said(
                "Take care",
                "this would boil the water, which this simple balance leaves out",
            ));
        }
        out
    }

    fn plots(&self) -> Vec<Plot> {
        let mut out = plot(
            "Temperatures",
            "time (s)",
            "temperature (°C)",
            vec![
                curve(
                    "the block",
                    self.history.iter().map(|p| (p[0], p[1])),
                    Style::Solid,
                    1,
                ),
                curve(
                    "the water",
                    self.history.iter().map(|p| (p[0], p[2])),
                    Style::Solid,
                    0,
                ),
            ],
        );
        out.from_zero = false;
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        vec![measured(
            "Final temperature, where heat lost equals heat gained (°C)",
            self.final_temperature(),
            self.settled().then_some(0.5 * (self.block + self.water)),
            0.02,
        )]
    }
}

/// Ice heated at a steady rate until it has all boiled away.
pub struct Heating {
    /// kg, °C, W.
    pub mass: f64,
    pub start: f64,
    pub power: f64,
    /// Joules put in so far.
    heat: f64,
    time: f64,
    /// When the ice began and finished melting and the water began and finished boiling, s.
    marks: [Option<f64>; 4],
}

/// Seconds of watching for the whole run, and how hot the steam is taken.
const WATCHING: f64 = 30.0;
const HOTTEST: f64 = 130.0;

impl Heating {
    pub fn new(mass: f64, start: f64, power: f64) -> Self {
        Self {
            mass,
            start: start.min(0.0),
            power,
            heat: 0.0,
            time: 0.0,
            marks: [None; 4],
        }
    }

    /// The heat needed to bring it to each turning point: ice at 0 °C, all melted, water at 100 °C, all boiled,
    /// and steam at the temperature the run ends at.
    pub fn stages(&self) -> [f64; 5] {
        let warm = self.mass * ICE * -self.start;
        let melted = warm + self.mass * FUSION;
        let hot = melted + self.mass * WATER * 100.0;
        let boiled = hot + self.mass * VAPORISATION;
        [
            warm,
            melted,
            hot,
            boiled,
            boiled + self.mass * STEAM * (HOTTEST - 100.0),
        ]
    }

    /// The temperature after `heat` joules, and how much of it is then ice, water and steam.
    pub fn state(&self, heat: f64) -> (f64, [f64; 3]) {
        let [warm, melted, hot, boiled, _] = self.stages();
        if heat < warm {
            (self.start + heat / (self.mass * ICE), [1.0, 0.0, 0.0])
        } else if heat < melted {
            let share = (heat - warm) / (melted - warm);
            (0.0, [1.0 - share, share, 0.0])
        } else if heat < hot {
            ((heat - melted) / (self.mass * WATER), [0.0, 1.0, 0.0])
        } else if heat < boiled {
            let share = (heat - hot) / (boiled - hot);
            (100.0, [0.0, 1.0 - share, share])
        } else {
            (
                100.0 + (heat - boiled) / (self.mass * STEAM),
                [0.0, 0.0, 1.0],
            )
        }
    }
}

impl Lesson for Heating {
    fn advance(&mut self, dt: f64) {
        let stages = self.stages();
        let span = dt * stages[4] / self.power / WATCHING;
        let before = self.heat;
        self.heat = (self.heat + self.power * span).min(stages[4]);
        for (mark, stage) in self.marks.iter_mut().zip(stages) {
            if mark.is_none() && before < stage && self.heat >= stage {
                // The moment within this step at which the turning point was passed.
                *mark = Some(self.time + (stage - before) / self.power);
            }
        }
        self.time += span;
    }

    fn finished(&self) -> bool {
        self.heat >= self.stages()[4]
    }

    fn clock(&self) -> String {
        format!("{:.0} s", self.time)
    }

    fn picture(&self) -> Picture {
        let (temperature, [ice, water, steam]) = self.state(self.heat);
        Picture::Beaker {
            ice,
            water,
            steam,
            temperature,
        }
    }

    fn readings(&self) -> Vec<Reading> {
        let (temperature, [ice, water, steam]) = self.state(self.heat);
        vec![
            reading("Temperature", temperature, "°C"),
            reading("Heat put in so far", self.heat / 1e3, "kJ"),
            reading("Ice", 100.0 * ice, "%"),
            reading("Water", 100.0 * water, "%"),
            reading("Steam", 100.0 * steam, "%"),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let stages = self.stages();
        let turning = std::iter::once(0.0).chain(stages);
        let mut out = plot(
            "Temperature",
            "time (s)",
            "temperature (°C)",
            vec![curve(
                "as heat goes in",
                turning.map(|q| (q / self.power, self.state(q).0)),
                Style::Solid,
                0,
            )],
        );
        out.marker = Some([self.time, self.state(self.heat).0]);
        out.from_zero = false;
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        let stages = self.stages();
        let seconds = |heat: f64| heat / self.power;
        let slack = 1e-6 * seconds(stages[4]);
        vec![
            measured(
                "Time to bring the ice to 0 °C, mcΔT/P (s)",
                seconds(stages[0]),
                self.marks[0],
                slack,
            ),
            measured(
                "Time then to melt it, mL/P (s)",
                seconds(stages[1] - stages[0]),
                self.marks[1].zip(self.marks[0]).map(|(b, a)| b - a),
                slack,
            ),
            measured(
                "Time then to reach 100 °C (s)",
                seconds(stages[2] - stages[1]),
                self.marks[2].zip(self.marks[1]).map(|(b, a)| b - a),
                slack,
            ),
            measured(
                "Time then to boil it away, mL/P (s)",
                seconds(stages[3] - stages[2]),
                self.marks[3].zip(self.marks[2]).map(|(b, a)| b - a),
                slack,
            ),
        ]
    }
}

/// A rod of two metals end to end, one end kept hot and the other cold, lagged along its length.
pub struct Conducting {
    /// °C, °C, W/(m K), W/(m K), m, m².
    pub hot: f64,
    pub cold: f64,
    pub first: f64,
    pub second: f64,
    pub length: f64,
    pub area: f64,
    temperatures: Vec<f64>,
    time: f64,
}

/// Slices the rod is cut into, the heat a cubic metre of it takes per kelvin, and seconds of watching for it
/// to settle.
const SLICES: usize = 40;
const CAPACITY: f64 = 3.5e6;
const SETTLING: f64 = 16.0;

impl Conducting {
    pub fn new(hot: f64, cold: f64, first: f64, second: f64, length: f64, area: f64) -> Self {
        Self {
            hot,
            cold,
            first,
            second,
            length,
            area,
            temperatures: vec![cold; SLICES],
            time: 0.0,
        }
    }

    fn conductivity(&self, slice: usize) -> f64 {
        if slice < SLICES / 2 {
            self.first
        } else {
            self.second
        }
    }

    /// The temperature where the two metals meet, once things are steady: the same heat crosses both halves.
    pub fn junction(&self) -> f64 {
        (self.first * self.hot + self.second * self.cold) / (self.first + self.second)
    }

    /// The steady flow of heat along the rod, W: two thermal resistances in series.
    pub fn current(&self) -> f64 {
        let half = 0.5 * self.length;
        self.area * (self.hot - self.cold) / (half / self.first + half / self.second)
    }

    /// The time the slower half takes to settle, s.
    fn settling_time(&self) -> f64 {
        0.6 * self.length * self.length * CAPACITY / self.first.min(self.second)
    }

    pub fn junction_now(&self) -> f64 {
        // Midway, in terms of resistance, between the slices either side of the join.
        let (a, b) = (
            self.temperatures[SLICES / 2 - 1],
            self.temperatures[SLICES / 2],
        );
        (self.first * a + self.second * b) / (self.first + self.second)
    }

    /// Heat leaving through the cold end now, W.
    pub fn current_now(&self) -> f64 {
        let dx = self.length / SLICES as f64;
        self.second * self.area * (self.temperatures[SLICES - 1] - self.cold) / (0.5 * dx)
    }

    fn steady(&self) -> bool {
        self.time >= self.settling_time()
    }
}

impl Lesson for Conducting {
    fn advance(&mut self, dt: f64) {
        if self.steady() {
            return;
        }
        let dx = self.length / SLICES as f64;
        let span = dt * self.settling_time() / SETTLING;
        // Short enough steps that heat cannot overshoot from one slice to the next.
        let limit = 0.2 * dx * dx * CAPACITY / self.first.max(self.second);
        let steps = (span / limit).ceil().max(1.0) as usize;
        let h = span / steps as f64;
        // Heat flowing from slice `i` to the next, per square metre: Fourier's law across the two half slices.
        let flux = |t: &[f64], i: usize, k: [f64; 2]| {
            (t[i] - t[i + 1]) / (0.5 * dx / k[0] + 0.5 * dx / k[1])
        };
        for _ in 0..steps {
            let t = self.temperatures.clone();
            for i in 0..SLICES {
                let k = self.conductivity(i);
                let inward = if i == 0 {
                    k * (self.hot - t[0]) / (0.5 * dx)
                } else {
                    flux(&t, i - 1, [self.conductivity(i - 1), k])
                };
                let outward = if i == SLICES - 1 {
                    k * (t[i] - self.cold) / (0.5 * dx)
                } else {
                    flux(&t, i, [k, self.conductivity(i + 1)])
                };
                self.temperatures[i] += (inward - outward) * h / (CAPACITY * dx);
            }
        }
        self.time += span;
    }

    fn finished(&self) -> bool {
        self.steady()
    }

    fn clock(&self) -> String {
        format!("{:.0} min", self.time / 60.0)
    }

    fn picture(&self) -> Picture {
        Picture::Bar {
            temperatures: self.temperatures.clone(),
            hot: self.hot,
            cold: self.cold,
        }
    }

    fn readings(&self) -> Vec<Reading> {
        vec![
            reading("Where the metals meet, now", self.junction_now(), "°C"),
            reading("Heat leaving at the cold end, now", self.current_now(), "W"),
            reading(
                "Temperature gradient in the first metal, when steady",
                (self.hot - self.junction()) / (0.5 * self.length),
                "K/m",
            ),
            reading(
                "In the second",
                (self.junction() - self.cold) / (0.5 * self.length),
                "K/m",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let dx = self.length / SLICES as f64;
        let now = self
            .temperatures
            .iter()
            .enumerate()
            .map(|(i, t)| ((i as f64 + 0.5) * dx, *t));
        let steady = [
            (0.0, self.hot),
            (0.5 * self.length, self.junction()),
            (self.length, self.cold),
        ];
        vec![plot(
            "Along the rod",
            "distance from the hot end (m)",
            "temperature (°C)",
            vec![
                curve("when steady", steady, Style::Dashed, 2),
                curve("now", now, Style::Solid, 1),
            ],
        )]
    }

    fn answers(&self) -> Vec<Answer> {
        vec![
            measured(
                "Temperature where the metals meet (°C)",
                self.junction(),
                self.steady().then(|| self.junction_now()),
                0.01 * (self.hot - self.cold),
            ),
            measured(
                "Heat flowing along the rod, A ΔT / (L₁/K₁ + L₂/K₂) (W)",
                self.current(),
                self.steady().then(|| self.current_now()),
                0.02 * self.current(),
            ),
        ]
    }
}

/// Something hot left to cool in a room.
pub struct Cooling {
    /// °C, °C, per minute.
    pub start: f64,
    pub room: f64,
    pub rate: f64,
    temperature: f64,
    /// Minutes.
    time: f64,
    halved: Option<f64>,
    history: Vec<[f64; 2]>,
}

/// Seconds of watching for the whole of the cooling.
const WATCHING_COOL: f64 = 20.0;

impl Cooling {
    pub fn new(start: f64, room: f64, rate: f64) -> Self {
        Self {
            start,
            room,
            rate,
            temperature: start,
            time: 0.0,
            halved: None,
            history: vec![[0.0, start]],
        }
    }

    fn watched(&self) -> f64 {
        5.0 / self.rate
    }

    pub fn temperature(&self) -> f64 {
        self.temperature
    }
}

impl Lesson for Cooling {
    fn advance(&mut self, dt: f64) {
        let end = (self.time + dt * self.watched() / WATCHING_COOL).min(self.watched());
        let cool = |t: f64| -self.rate * (t - self.room);
        while self.time < end {
            let h = (end - self.time).min(0.02 / self.rate);
            let before = self.temperature;
            // Newton's law of cooling: it loses heat in proportion to how much hotter than the room it is.
            let k1 = cool(before);
            let k2 = cool(before + 0.5 * h * k1);
            let k3 = cool(before + 0.5 * h * k2);
            let k4 = cool(before + h * k3);
            self.temperature = before + h * (k1 + 2.0 * k2 + 2.0 * k3 + k4) / 6.0;
            let half = self.room + 0.5 * (self.start - self.room);
            if self.halved.is_none()
                && (before - half) * (self.temperature - half) <= 0.0
                && before != self.temperature
            {
                self.halved = Some(self.time + h * (before - half) / (before - self.temperature));
            }
            self.time += h;
        }
        self.history.push([self.time, self.temperature]);
    }

    fn finished(&self) -> bool {
        self.time >= self.watched() - 1e-9
    }

    fn clock(&self) -> String {
        format!("{:.1} min", self.time)
    }

    fn picture(&self) -> Picture {
        Picture::Cup {
            temperature: self.temperature,
            room: self.room,
            start: self.start,
        }
    }

    fn readings(&self) -> Vec<Reading> {
        vec![
            reading("Temperature now", self.temperature, "°C"),
            reading("Hotter than the room by", self.temperature - self.room, "K"),
            reading(
                "Cooling at",
                self.rate * (self.temperature - self.room),
                "K a minute",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let law = (0..=100)
            .map(|i| self.watched() * f64::from(i) / 100.0)
            .map(|t| {
                (
                    t,
                    self.room + (self.start - self.room) * (-self.rate * t).exp(),
                )
            });
        let mut out = plot(
            "Cooling",
            "time (min)",
            "temperature (°C)",
            vec![
                curve("T_room + ΔT e^(−kt)", law, Style::Dashed, 2),
                curve(
                    "the cup",
                    self.history.iter().map(|p| (p[0], p[1])),
                    Style::Solid,
                    1,
                ),
            ],
        );
        out.from_zero = false;
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        let after = 10.0f64.min(self.watched());
        vec![
            measured(
                "Time for its excess over the room to halve, ln 2 / k (min)",
                std::f64::consts::LN_2 / self.rate,
                self.halved,
                1e-3 / self.rate,
            ),
            answer(
                "Temperature after ten minutes (°C)",
                self.room + (self.start - self.room) * (-self.rate * after).exp(),
            ),
        ]
    }
}

/// Planck's constant, the speed of light, Boltzmann's constant, and the constants of Wien and of Stefan.
const H: f64 = 6.626_070_15e-34;
const C: f64 = 2.997_924_58e8;
const K: f64 = 1.380_649e-23;
pub const WIEN: f64 = 2.897_772e-3;
pub const STEFAN: f64 = 5.670_374e-8;

/// A body that absorbs all the light falling on it, glowing at a temperature.
pub struct Glowing {
    /// K.
    pub temperature: f64,
}

impl Glowing {
    /// Planck's law: power sent out per square metre per metre of wavelength, at a wavelength in metres.
    pub fn planck(&self, wavelength: f64) -> f64 {
        let x = H * C / (wavelength * K * self.temperature);
        2.0 * std::f64::consts::PI * H * C * C / wavelength.powi(5) / x.exp_m1()
    }

    /// The wavelength at which it is brightest, found from the curve itself, m.
    pub fn brightest(&self) -> f64 {
        let around = WIEN / self.temperature;
        (0..=4000)
            .map(|i| around * (0.5 + f64::from(i) / 4000.0))
            .max_by(|a, b| self.planck(*a).total_cmp(&self.planck(*b)))
            .unwrap_or(around)
    }

    /// The whole power sent out per square metre, by adding Planck's curve up over every wavelength.
    pub fn power(&self) -> f64 {
        // Evenly in the logarithm of the wavelength, from far below the peak to far above it.
        let around = WIEN / self.temperature;
        let (from, to, steps) = ((0.02 * around).ln(), (200.0 * around).ln(), 6000);
        let step = (to - from) / f64::from(steps);
        (0..steps)
            .map(|i| {
                let wavelength = (from + (f64::from(i) + 0.5) * step).exp();
                self.planck(wavelength) * wavelength * step
            })
            .sum()
    }
}

impl Lesson for Glowing {
    fn still(&self) -> bool {
        true
    }

    fn picture(&self) -> Picture {
        Picture::Chart
    }

    fn readings(&self) -> Vec<Reading> {
        let peak = WIEN / self.temperature * 1e9;
        let region = if peak < 380.0 {
            "ultraviolet"
        } else if peak <= 750.0 {
            "visible"
        } else {
            "infrared"
        };
        vec![
            reading("Brightest at a wavelength of", peak, "nm"),
            said("which is in the", region),
            reading(
                "Power sent out by each square metre",
                STEFAN * self.temperature.powi(4) / 1e3,
                "kW",
            ),
            reading(
                "Compared with the same at 300 K",
                (self.temperature / 300.0).powi(4),
                "times",
            ),
        ]
    }

    fn plots(&self) -> Vec<Plot> {
        let wavelengths = (1..=300).map(|i| 10e-9 * f64::from(i));
        // In units that keep the numbers readable: kW per square metre per nanometre.
        let mut out = plot(
            "Spectrum",
            "wavelength (nm)",
            "power per m² per nm (kW)",
            vec![curve(
                "Planck's law",
                wavelengths.map(|w| (w * 1e9, self.planck(w) * 1e-9 / 1e3)),
                Style::Solid,
                1,
            )],
        );
        let peak = WIEN / self.temperature;
        out.marker = (peak < 3e-6).then(|| [peak * 1e9, self.planck(peak) * 1e-9 / 1e3]);
        vec![out]
    }

    fn answers(&self) -> Vec<Answer> {
        let (wien, stefan) = (
            WIEN / self.temperature * 1e9,
            STEFAN * self.temperature.powi(4) / 1e3,
        );
        vec![
            measured(
                "Brightest wavelength, Wien's law, b/T (nm)",
                wien,
                Some(self.brightest() * 1e9),
                0.002 * wien,
            ),
            measured(
                "Power per square metre, Stefan's law, σT⁴ (kW)",
                stefan,
                Some(self.power() / 1e3),
                0.002 * stefan,
            ),
        ]
    }
}
