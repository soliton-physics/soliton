use soliton_core::Vec2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    pub radius: f64,
}

impl Circle {
    pub fn new(radius: f64) -> Self {
        Self { radius }
    }
}

/// Convex polygon with counter-clockwise vertices and its centroid at the origin.
#[derive(Clone, Debug, PartialEq)]
pub struct Polygon {
    vertices: Vec<Vec2>,
}

impl Polygon {
    /// Builds the polygon from vertices in any winding, recentred on their centroid.
    pub fn new(vertices: impl Into<Vec<Vec2>>) -> Self {
        Self::with_centroid(vertices).0
    }

    /// The polygon together with where its centroid was in the coordinates the vertices were given in.
    pub fn with_centroid(vertices: impl Into<Vec<Vec2>>) -> (Self, Vec2) {
        let mut vertices = vertices.into();
        assert!(
            vertices.len() >= 3,
            "a polygon needs at least three vertices"
        );
        if signed_area(&vertices) < 0.0 {
            vertices.reverse();
        }
        let centroid = centroid(&vertices);
        vertices.iter_mut().for_each(|v| *v -= centroid);
        (Self { vertices }, centroid)
    }

    pub fn rectangle(width: f64, height: f64) -> Self {
        let (w, h) = (0.5 * width, 0.5 * height);
        Self::new([
            Vec2::new(-w, -h),
            Vec2::new(w, -h),
            Vec2::new(w, h),
            Vec2::new(-w, h),
        ])
    }

    pub fn vertices(&self) -> &[Vec2] {
        &self.vertices
    }

    pub fn area(&self) -> f64 {
        signed_area(&self.vertices)
    }

    /// Moment of inertia about the centroid per unit mass.
    pub fn inertia_per_mass(&self) -> f64 {
        let n = self.vertices.len();
        let (mut numerator, mut denominator) = (0.0, 0.0);
        for i in 0..n {
            let (a, b) = (self.vertices[i], self.vertices[(i + 1) % n]);
            let cross = a.cross(b);
            numerator += cross * (a.dot(a) + a.dot(b) + b.dot(b));
            denominator += cross;
        }
        numerator / (6.0 * denominator)
    }
}

/// The region `normal · p <= offset`, with the surface at equality.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HalfPlane {
    pub normal: Vec2,
    pub offset: f64,
}

impl HalfPlane {
    pub fn new(normal: Vec2, offset: f64) -> Self {
        Self {
            normal: normal.normalized(),
            offset,
        }
    }

    /// Signed distance of a point above the surface.
    pub fn height_of(&self, point: Vec2) -> f64 {
        self.normal.dot(point) - self.offset
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Circle(Circle),
    Polygon(Polygon),
    HalfPlane(HalfPlane),
}

impl Shape {
    pub fn inertia(&self, mass: f64) -> f64 {
        match self {
            Shape::Circle(c) => 0.5 * mass * c.radius * c.radius,
            Shape::Polygon(p) => mass * p.inertia_per_mass(),
            Shape::HalfPlane(_) => f64::INFINITY,
        }
    }
}

impl From<Circle> for Shape {
    fn from(c: Circle) -> Self {
        Shape::Circle(c)
    }
}

impl From<Polygon> for Shape {
    fn from(p: Polygon) -> Self {
        Shape::Polygon(p)
    }
}

impl From<HalfPlane> for Shape {
    fn from(h: HalfPlane) -> Self {
        Shape::HalfPlane(h)
    }
}

fn signed_area(vertices: &[Vec2]) -> f64 {
    let n = vertices.len();
    0.5 * (0..n)
        .map(|i| vertices[i].cross(vertices[(i + 1) % n]))
        .sum::<f64>()
}

fn centroid(vertices: &[Vec2]) -> Vec2 {
    let n = vertices.len();
    let area = signed_area(vertices);
    let mut sum = Vec2::ZERO;
    for i in 0..n {
        let (a, b) = (vertices[i], vertices[(i + 1) % n]);
        sum += (a + b) * a.cross(b);
    }
    sum / (6.0 * area)
}
