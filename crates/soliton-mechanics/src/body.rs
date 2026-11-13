use crate::magnetism::Magnet;
use crate::shape::{Circle, Polygon, Shape};
use soliton_core::Vec2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BodyId(pub(crate) u32);

impl BodyId {
    pub fn index(self) -> usize {
        self.0 as usize
    }

    pub const fn from_index(index: usize) -> Self {
        Self(index as u32)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyKind {
    /// A point mass; rotation is locked.
    Particle,
    /// A body with extent and rotational inertia.
    Rigid,
    /// Infinite mass; never moves. Ground, incline, wall, pivot.
    Static,
}

/// Surface properties used by contact. Defaults are "smooth" and "elastic", the textbook's silence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    pub static_friction: f64,
    pub kinetic_friction: f64,
    pub restitution: f64,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            static_friction: 0.0,
            kinetic_friction: 0.0,
            restitution: 1.0,
        }
    }
}

/// A body description, built fluently and handed to [`crate::World::add`].
#[derive(Clone, Debug, PartialEq)]
pub struct Body {
    pub kind: BodyKind,
    pub mass: f64,
    pub inertia: f64,
    pub shape: Shape,
    pub position: Vec2,
    pub angle: f64,
    pub velocity: Vec2,
    pub angular_velocity: f64,
    pub material: Material,
    /// Electric charge, C, taken to sit at the centre of the body.
    pub charge: f64,
    /// Makes the body a bar magnet lying along its own x axis.
    pub magnet: Option<Magnet>,
    /// Magnetic moment the body takes on per tesla of field it sits in, A·m²/T: positive for a paramagnet,
    /// negative for a diamagnet.
    pub polarisability: f64,
}

impl Body {
    pub fn particle(mass: f64) -> Self {
        assert!(mass > 0.0, "mass must be positive");
        Self::with(
            BodyKind::Particle,
            mass,
            f64::INFINITY,
            Shape::Circle(Circle::new(0.0)),
        )
    }

    pub fn rigid(mass: f64, shape: impl Into<Shape>) -> Self {
        assert!(mass > 0.0, "mass must be positive");
        let shape = shape.into();
        let inertia = shape.inertia(mass);
        Self::with(BodyKind::Rigid, mass, inertia, shape)
    }

    pub fn fixed(shape: impl Into<Shape>) -> Self {
        Self::with(BodyKind::Static, f64::INFINITY, f64::INFINITY, shape.into())
    }

    /// A static polygon given by vertices in world coordinates, such as an incline drawn on the page.
    pub fn fixed_polygon(world_vertices: impl Into<Vec<Vec2>>) -> Self {
        let (polygon, centroid) = Polygon::with_centroid(world_vertices);
        Self::fixed(polygon).at(centroid.x, centroid.y)
    }

    fn with(kind: BodyKind, mass: f64, inertia: f64, shape: Shape) -> Self {
        Self {
            kind,
            mass,
            inertia,
            shape,
            position: Vec2::ZERO,
            angle: 0.0,
            velocity: Vec2::ZERO,
            angular_velocity: 0.0,
            material: Material::default(),
            charge: 0.0,
            magnet: None,
            polarisability: 0.0,
        }
    }

    pub fn at(mut self, x: f64, y: f64) -> Self {
        self.position = Vec2::new(x, y);
        self
    }

    pub fn velocity(mut self, x: f64, y: f64) -> Self {
        self.velocity = Vec2::new(x, y);
        self
    }

    pub fn angle(mut self, angle: f64) -> Self {
        self.angle = angle;
        self
    }

    pub fn spin(mut self, angular_velocity: f64) -> Self {
        self.angular_velocity = angular_velocity;
        self
    }

    pub fn shape(mut self, shape: impl Into<Shape>) -> Self {
        self.shape = shape.into();
        self
    }

    pub fn inertia(mut self, inertia: f64) -> Self {
        self.inertia = inertia;
        self
    }

    pub fn friction(mut self, static_friction: f64, kinetic_friction: f64) -> Self {
        self.material.static_friction = static_friction;
        self.material.kinetic_friction = kinetic_friction;
        self
    }

    pub fn restitution(mut self, restitution: f64) -> Self {
        self.material.restitution = restitution;
        self
    }

    pub fn charge(mut self, charge: f64) -> Self {
        self.charge = charge;
        self
    }

    pub fn magnet(mut self, magnet: Magnet) -> Self {
        self.magnet = Some(magnet);
        self
    }

    pub fn polarisability(mut self, polarisability: f64) -> Self {
        self.polarisability = polarisability;
        self
    }
}

/// Structure-of-arrays storage for every body in a world.
#[derive(Clone, Debug, Default)]
pub struct Bodies {
    pub(crate) kinds: Vec<BodyKind>,
    pub(crate) masses: Vec<f64>,
    pub(crate) inverse_masses: Vec<f64>,
    pub(crate) inertias: Vec<f64>,
    pub(crate) inverse_inertias: Vec<f64>,
    pub(crate) shapes: Vec<Shape>,
    pub(crate) materials: Vec<Material>,
    pub(crate) charges: Vec<f64>,
    pub(crate) magnets: Vec<Option<Magnet>>,
    pub(crate) polarisabilities: Vec<f64>,
    pub(crate) positions: Vec<Vec2>,
    pub(crate) angles: Vec<f64>,
    pub(crate) velocities: Vec<Vec2>,
    pub(crate) angular_velocities: Vec<f64>,
}

impl Bodies {
    pub(crate) fn push(&mut self, body: Body) -> BodyId {
        let id = BodyId(self.kinds.len() as u32);
        self.kinds.push(body.kind);
        self.masses.push(body.mass);
        self.inverse_masses.push(if body.kind == BodyKind::Static {
            0.0
        } else {
            1.0 / body.mass
        });
        self.inertias.push(body.inertia);
        self.inverse_inertias.push(if body.inertia.is_finite() {
            1.0 / body.inertia
        } else {
            0.0
        });
        self.shapes.push(body.shape);
        self.materials.push(body.material);
        self.charges.push(body.charge);
        self.magnets.push(body.magnet);
        self.polarisabilities.push(body.polarisability);
        self.positions.push(body.position);
        self.angles.push(body.angle);
        self.velocities.push(body.velocity);
        self.angular_velocities.push(body.angular_velocity);
        id
    }

    pub fn len(&self) -> usize {
        self.kinds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }

    pub fn ids(&self) -> impl Iterator<Item = BodyId> {
        (0..self.kinds.len() as u32).map(BodyId)
    }

    pub fn kind(&self, id: BodyId) -> BodyKind {
        self.kinds[id.index()]
    }

    pub fn is_dynamic(&self, id: BodyId) -> bool {
        self.kinds[id.index()] != BodyKind::Static
    }

    pub fn mass(&self, id: BodyId) -> f64 {
        self.masses[id.index()]
    }

    pub fn inertia(&self, id: BodyId) -> f64 {
        self.inertias[id.index()]
    }

    pub fn shape(&self, id: BodyId) -> &Shape {
        &self.shapes[id.index()]
    }

    pub fn material(&self, id: BodyId) -> Material {
        self.materials[id.index()]
    }

    pub fn charge(&self, id: BodyId) -> f64 {
        self.charges[id.index()]
    }

    pub fn magnet(&self, id: BodyId) -> Option<Magnet> {
        self.magnets[id.index()]
    }

    pub fn polarisability(&self, id: BodyId) -> f64 {
        self.polarisabilities[id.index()]
    }

    pub fn position(&self, id: BodyId) -> Vec2 {
        self.positions[id.index()]
    }

    pub fn angle(&self, id: BodyId) -> f64 {
        self.angles[id.index()]
    }

    pub fn velocity(&self, id: BodyId) -> Vec2 {
        self.velocities[id.index()]
    }

    pub fn angular_velocity(&self, id: BodyId) -> f64 {
        self.angular_velocities[id.index()]
    }

    /// World position of a point given in the body's frame.
    pub fn world_point(&self, id: BodyId, local: Vec2) -> Vec2 {
        self.positions[id.index()] + local.rotated(self.angles[id.index()])
    }

    /// Velocity of a world point moving with the body.
    pub fn point_velocity(&self, id: BodyId, world_point: Vec2) -> Vec2 {
        let i = id.index();
        self.velocities[i] + (world_point - self.positions[i]).perp() * self.angular_velocities[i]
    }
}
