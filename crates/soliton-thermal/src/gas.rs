use crate::rng::Rng;
use crate::BOLTZMANN;

/// The six walls of the box, in the order low x, high x, low y, high y, low z, high z. The high-x wall is the
/// one that can move: the piston.
pub const WALLS: usize = 6;
const PISTON: usize = 1;

/// What a wall does to a molecule that hits it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Wall {
    /// Throws it back with the speed it came with: no energy changes hands.
    Insulating,
    /// Held at a temperature, K. A molecule leaves with a fresh velocity drawn for that temperature, whatever
    /// it arrived with; the difference in its energy is heat.
    Bath(f64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Molecule {
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    /// amu.
    pub mass: f64,
    /// nm.
    pub radius: f64,
    /// Which kind it is, for mixtures.
    pub species: u8,
}

/// Molecules as hard spheres in a rectangular box from the origin to `size`.
#[derive(Clone, Debug)]
pub struct Gas {
    pub molecules: Vec<Molecule>,
    pub size: [f64; 3],
    pub walls: [Wall; WALLS],
    /// Speed of the piston along x, nm/ps; it stops on reaching `piston_target`.
    pub piston_speed: f64,
    pub piston_target: f64,
    /// A thin insulating wall across the box at this x, with molecules kept to the side they are on.
    pub partition: Option<f64>,
    /// Whether molecules bounce off one another. Without it they pass through each other: an ideal gas in the
    /// strictest sense, which never comes to equilibrium by itself.
    pub collisions: bool,
    pub time: f64,
    /// Work done on the gas by the piston so far, and heat given to it by each wall.
    pub work: f64,
    pub heat: [f64; WALLS],
    /// Momentum handed to each wall so far, counted without sign.
    pub impulse: [f64; WALLS],
    /// Collisions between molecules so far.
    pub encounters: u64,
    rng: Rng,
    // Cells for finding neighbours: the first molecule in each, and the next one after each molecule.
    heads: Vec<u32>,
    next: Vec<u32>,
}

const NONE: u32 = u32::MAX;

impl Gas {
    pub fn new(size: [f64; 3], seed: u64) -> Self {
        Self {
            molecules: Vec::new(),
            size,
            walls: [Wall::Insulating; WALLS],
            piston_speed: 0.0,
            piston_target: size[0],
            partition: None,
            collisions: true,
            time: 0.0,
            work: 0.0,
            heat: [0.0; WALLS],
            impulse: [0.0; WALLS],
            encounters: 0,
            rng: Rng::new(seed),
            heads: Vec::new(),
            next: Vec::new(),
        }
    }

    pub fn rng(&mut self) -> &mut Rng {
        &mut self.rng
    }

    pub fn volume(&self) -> f64 {
        self.size[0] * self.size[1] * self.size[2]
    }

    /// The area of all six walls.
    pub fn surface(&self) -> f64 {
        let [x, y, z] = self.size;
        2.0 * (x * y + y * z + z * x)
    }

    pub fn kinetic_energy(&self) -> f64 {
        self.molecules.iter().map(Molecule::energy).sum()
    }

    /// The temperature of the molecules for which `which` holds: two thirds of their mean kinetic energy, over
    /// Boltzmann's constant. `None` when there are none of them.
    pub fn temperature_of(&self, which: impl Fn(&Molecule) -> bool) -> Option<f64> {
        let (mut energy, mut count) = (0.0, 0usize);
        for m in self.molecules.iter().filter(|m| which(m)) {
            energy += m.energy();
            count += 1;
        }
        (count > 0).then(|| 2.0 * energy / (3.0 * count as f64 * BOLTZMANN))
    }

    pub fn temperature(&self) -> f64 {
        self.temperature_of(|_| true).unwrap_or(0.0)
    }

    /// Sends the piston towards `target` at `speed`, nm/ps.
    pub fn drive_piston(&mut self, target: f64, speed: f64) {
        self.piston_target = target;
        self.piston_speed = speed.abs() * (target - self.size[0]).signum();
    }

    /// One step of `dt` picoseconds: everything flies straight, then whatever has met a wall or another
    /// molecule bounces.
    pub fn step(&mut self, dt: f64) {
        if self.piston_speed != 0.0 {
            let ahead = self.size[0] + self.piston_speed * dt;
            let arrived = (self.piston_target - ahead) * self.piston_speed <= 0.0;
            self.size[0] = if arrived { self.piston_target } else { ahead };
            if arrived {
                self.piston_speed = 0.0;
            }
        }
        for i in 0..self.molecules.len() {
            let mut m = self.molecules[i];
            let side = self.partition.map(|at| m.position[0] < at);
            for axis in 0..3 {
                m.position[axis] += m.velocity[axis] * dt;
            }
            if let (Some(at), Some(left)) = (self.partition, side) {
                // Kept to its own side of the partition, which gives back exactly what it gets.
                let through = if left {
                    m.position[0] > at && m.velocity[0] > 0.0
                } else {
                    m.position[0] < at && m.velocity[0] < 0.0
                };
                if through {
                    // Put back on its own side as well as turned round, or the next step would take it for a
                    // molecule of the other side.
                    m.velocity[0] = -m.velocity[0];
                    m.position[0] = 2.0 * at - m.position[0];
                }
            }
            for wall in 0..WALLS {
                self.meet_wall(&mut m, wall);
            }
            self.molecules[i] = m;
        }
        if self.collisions {
            self.collide();
        }
        self.time += dt;
    }

    fn meet_wall(&mut self, m: &mut Molecule, wall: usize) {
        let axis = wall / 2;
        let high = wall % 2 == 1;
        let moving = if wall == PISTON {
            self.piston_speed
        } else {
            0.0
        };
        // Speed towards the wall, in the wall's own frame.
        let towards = if high {
            m.velocity[axis] - moving
        } else {
            -m.velocity[axis]
        };
        // The walls act on a molecule's centre, so the volume open to the gas is the volume of the box.
        let beyond = if high {
            m.position[axis] - self.size[axis]
        } else {
            -m.position[axis]
        };
        if beyond <= 0.0 || towards <= 0.0 {
            return;
        }
        let before = m.energy();
        let arriving = m.velocity[axis];
        match self.walls[wall] {
            // A wall that is moving is a piston, and a piston only pushes: heat is the fixed walls' business.
            Wall::Bath(temperature) if moving == 0.0 => {
                let spread = (BOLTZMANN * temperature / m.mass).sqrt();
                // Molecules leaving a wall are not a fair sample of the gas: the fast ones leave more often.
                let away = spread * (-2.0 * self.rng.uniform().ln()).sqrt();
                for other in 0..3 {
                    m.velocity[other] = spread * self.rng.normal();
                }
                m.velocity[axis] = if high { -away } else { away };
                self.heat[wall] += m.energy() - before;
            }
            _ => {
                m.velocity[axis] = if high {
                    2.0 * moving - arriving
                } else {
                    -arriving
                };
                if wall == PISTON {
                    self.work += m.energy() - before;
                }
            }
        }
        self.impulse[wall] += m.mass * (m.velocity[axis] - arriving).abs();
    }

    /// Bounces every pair of molecules that overlap and are closing on each other, as smooth hard spheres:
    /// momentum is exchanged along the line of centres and nothing else changes.
    fn collide(&mut self) {
        let count = self.molecules.len();
        if count < 2 {
            return;
        }
        let reach = 2.0 * self.molecules.iter().map(|m| m.radius).fold(0.0, f64::max);
        if reach <= 0.0 {
            return;
        }
        let size = self.size;
        let cells = size.map(|length| ((length / reach).floor() as usize).clamp(1, 64));
        let cell_of = |m: &Molecule| {
            let mut index = [0usize; 3];
            for axis in 0..3 {
                let share = m.position[axis] / size[axis] * cells[axis] as f64;
                index[axis] = (share.max(0.0) as usize).min(cells[axis] - 1);
            }
            index
        };
        let flat = |c: [usize; 3]| (c[0] * cells[1] + c[1]) * cells[2] + c[2];
        self.heads.clear();
        self.heads.resize(cells[0] * cells[1] * cells[2], NONE);
        self.next.clear();
        self.next.resize(count, NONE);
        for i in 0..count {
            let cell = flat(cell_of(&self.molecules[i]));
            self.next[i] = self.heads[cell];
            self.heads[cell] = i as u32;
        }
        for i in 0..count {
            let home = cell_of(&self.molecules[i]);
            for dx in -1i64..=1 {
                for dy in -1i64..=1 {
                    for dz in -1i64..=1 {
                        let cell = [
                            home[0] as i64 + dx,
                            home[1] as i64 + dy,
                            home[2] as i64 + dz,
                        ];
                        if (0..3).any(|a| cell[a] < 0 || cell[a] >= cells[a] as i64) {
                            continue;
                        }
                        let mut j = self.heads[flat(cell.map(|c| c as usize))];
                        while j != NONE {
                            // Each pair once.
                            if (j as usize) > i {
                                self.bounce(i, j as usize);
                            }
                            j = self.next[j as usize];
                        }
                    }
                }
            }
        }
    }

    fn bounce(&mut self, i: usize, j: usize) {
        let (a, b) = (self.molecules[i], self.molecules[j]);
        let mut apart = [0.0; 3];
        let mut closing = 0.0;
        let mut distance_squared = 0.0;
        for axis in 0..3 {
            apart[axis] = b.position[axis] - a.position[axis];
            distance_squared += apart[axis] * apart[axis];
            closing += apart[axis] * (b.velocity[axis] - a.velocity[axis]);
        }
        let touching = a.radius + b.radius;
        if distance_squared >= touching * touching || closing >= 0.0 || distance_squared == 0.0 {
            return;
        }
        // The impulse along the line of centres that reverses their speed of approach.
        let impulse = 2.0 * closing / (distance_squared * (1.0 / a.mass + 1.0 / b.mass));
        for axis in 0..3 {
            self.molecules[i].velocity[axis] += impulse * apart[axis] / a.mass;
            self.molecules[j].velocity[axis] -= impulse * apart[axis] / b.mass;
        }
        self.encounters += 1;
    }
}

impl Molecule {
    pub fn energy(&self) -> f64 {
        0.5 * self.mass * self.speed_squared()
    }

    pub fn speed_squared(&self) -> f64 {
        self.velocity.iter().map(|v| v * v).sum()
    }
}
