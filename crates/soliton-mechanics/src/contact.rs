use crate::body::{Bodies, BodyId};
use crate::shape::{Circle, HalfPlane, Polygon, Shape};
use soliton_core::Vec2;

/// Gap below which two surfaces are treated as touching. Wide enough that both corners of a resting box stay in
/// the contact set while it settles; a contact with a positive gap carries no force, so nothing hovers.
pub const ACTIVATION: f64 = 1e-4;

/// A touching point between two bodies. The normal points from `a` to `b`; `depth` is positive when overlapping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contact {
    pub a: BodyId,
    pub b: BodyId,
    pub point: Vec2,
    pub normal: Vec2,
    pub depth: f64,
    /// Identifies which features touch, so impulses can be carried from one step to the next.
    pub feature: u32,
}

struct Touch {
    point: Vec2,
    normal: Vec2,
    depth: f64,
    feature: u32,
}

/// Every contact between every pair of bodies at the current positions, skipping `ignored` pairs.
pub(crate) fn detect(bodies: &Bodies, ignored: &[(BodyId, BodyId)], out: &mut Vec<Contact>) {
    out.clear();
    let n = bodies.len();
    let mut touches = Vec::new();
    for i in 0..n {
        for j in i + 1..n {
            let (a, b) = (BodyId(i as u32), BodyId(j as u32));
            if !bodies.is_dynamic(a) && !bodies.is_dynamic(b) {
                continue;
            }
            if ignored
                .iter()
                .any(|&(x, y)| (x == a && y == b) || (x == b && y == a))
            {
                continue;
            }
            touches.clear();
            collide(bodies, a, b, &mut touches);
            out.extend(touches.iter().map(|t| Contact {
                a,
                b,
                point: t.point,
                normal: t.normal,
                depth: t.depth,
                feature: t.feature,
            }));
        }
    }
}

fn collide(bodies: &Bodies, a: BodyId, b: BodyId, out: &mut Vec<Touch>) {
    let (sa, sb) = (bodies.shape(a), bodies.shape(b));
    let (pa, pb) = (bodies.position(a), bodies.position(b));
    let (ta, tb) = (bodies.angle(a), bodies.angle(b));
    match (sa, sb) {
        (Shape::Circle(ca), Shape::Circle(cb)) => circle_circle(ca, pa, cb, pb, out),
        (Shape::Circle(c), Shape::HalfPlane(h)) => flipped(out, |o| circle_halfplane(h, c, pa, o)),
        (Shape::HalfPlane(h), Shape::Circle(c)) => circle_halfplane(h, c, pb, out),
        (Shape::Polygon(p), Shape::HalfPlane(h)) => {
            flipped(out, |o| polygon_halfplane(h, &world_vertices(p, pa, ta), o))
        }
        (Shape::HalfPlane(h), Shape::Polygon(p)) => {
            polygon_halfplane(h, &world_vertices(p, pb, tb), out)
        }
        (Shape::Circle(c), Shape::Polygon(p)) => flipped(out, |o| {
            polygon_circle(&world_vertices(p, pb, tb), c, pa, o)
        }),
        (Shape::Polygon(p), Shape::Circle(c)) => {
            polygon_circle(&world_vertices(p, pa, ta), c, pb, out)
        }
        (Shape::Polygon(p), Shape::Polygon(q)) => {
            polygon_polygon(&world_vertices(p, pa, ta), &world_vertices(q, pb, tb), out)
        }
        (Shape::HalfPlane(_), Shape::HalfPlane(_)) => {}
    }
}

/// Runs a pair routine written for the opposite order and flips its normals.
fn flipped(out: &mut Vec<Touch>, routine: impl FnOnce(&mut Vec<Touch>)) {
    let start = out.len();
    routine(out);
    for t in &mut out[start..] {
        t.normal = -t.normal;
    }
}

fn world_vertices(polygon: &Polygon, position: Vec2, angle: f64) -> Vec<Vec2> {
    polygon
        .vertices()
        .iter()
        .map(|&v| position + v.rotated(angle))
        .collect()
}

fn circle_circle(ca: &Circle, pa: Vec2, cb: &Circle, pb: Vec2, out: &mut Vec<Touch>) {
    let separation = pb - pa;
    let distance = separation.length();
    let depth = ca.radius + cb.radius - distance;
    if depth < -ACTIVATION {
        return;
    }
    let normal = if distance > 0.0 {
        separation / distance
    } else {
        Vec2::Y
    };
    out.push(Touch {
        point: pa + normal * (ca.radius - 0.5 * depth),
        normal,
        depth,
        feature: 0,
    });
}

/// Plane is body `a`; the normal points out of the plane into the circle.
fn circle_halfplane(plane: &HalfPlane, circle: &Circle, center: Vec2, out: &mut Vec<Touch>) {
    let depth = circle.radius - plane.height_of(center);
    if depth < -ACTIVATION {
        return;
    }
    out.push(Touch {
        point: center - plane.normal * circle.radius,
        normal: plane.normal,
        depth,
        feature: 0,
    });
}

/// Plane is body `a`. Up to the two deepest vertices form the manifold.
fn polygon_halfplane(plane: &HalfPlane, vertices: &[Vec2], out: &mut Vec<Touch>) {
    let mut touching: Vec<(f64, usize)> = vertices
        .iter()
        .enumerate()
        .map(|(i, &v)| (-plane.height_of(v), i))
        .filter(|&(depth, _)| depth >= -ACTIVATION)
        .collect();
    touching.sort_by(|x, y| y.0.total_cmp(&x.0));
    for &(depth, i) in touching.iter().take(2) {
        out.push(Touch {
            point: vertices[i],
            normal: plane.normal,
            depth,
            feature: i as u32,
        });
    }
}

/// Polygon is body `a`; the normal points from the polygon to the circle.
fn polygon_circle(vertices: &[Vec2], circle: &Circle, center: Vec2, out: &mut Vec<Touch>) {
    let n = vertices.len();
    let mut deepest_edge = 0;
    let mut deepest = f64::NEG_INFINITY;
    for i in 0..n {
        let separation = (center - vertices[i]).dot(outward_normal(vertices, i));
        if separation > deepest {
            deepest = separation;
            deepest_edge = i;
        }
    }
    if deepest <= 0.0 {
        let normal = outward_normal(vertices, deepest_edge);
        let depth = circle.radius - deepest;
        out.push(Touch {
            point: center - normal * deepest,
            normal,
            depth,
            feature: deepest_edge as u32,
        });
        return;
    }
    let (closest, _) = (0..n)
        .map(|i| closest_point_on_segment(vertices[i], vertices[(i + 1) % n], center))
        .map(|p| (p, (center - p).length_squared()))
        .min_by(|x, y| x.1.total_cmp(&y.1))
        .unwrap();
    let separation = center - closest;
    let distance = separation.length();
    let depth = circle.radius - distance;
    if depth < -ACTIVATION {
        return;
    }
    let normal = if distance > 0.0 {
        separation / distance
    } else {
        outward_normal(vertices, deepest_edge)
    };
    out.push(Touch {
        point: closest,
        normal,
        depth,
        feature: 0,
    });
}

/// Separating axis test with reference-face clipping. Polygon `a` first; normals point from `a` to `b`.
fn polygon_polygon(a: &[Vec2], b: &[Vec2], out: &mut Vec<Touch>) {
    let (edge_a, separation_a) = max_separation(a, b);
    if separation_a > ACTIVATION {
        return;
    }
    let (edge_b, separation_b) = max_separation(b, a);
    if separation_b > ACTIVATION {
        return;
    }
    let reference_is_b = separation_b > separation_a + 1e-12;
    let (reference, incident, edge) = if reference_is_b {
        (b, a, edge_b)
    } else {
        (a, b, edge_a)
    };

    let n = reference.len();
    let (v11, v12) = (reference[edge], reference[(edge + 1) % n]);
    let normal = outward_normal(reference, edge);
    let tangent = (v12 - v11).normalized();

    let m = incident.len();
    let incident_edge = (0..m)
        .min_by(|&i, &j| {
            outward_normal(incident, i)
                .dot(normal)
                .total_cmp(&outward_normal(incident, j).dot(normal))
        })
        .unwrap();
    let (v21, v22) = (incident[incident_edge], incident[(incident_edge + 1) % m]);

    let Some((c1, c2)) = clip(v21, v22, -tangent, -v11.dot(tangent)) else {
        return;
    };
    let Some((c1, c2)) = clip(c1, c2, tangent, v12.dot(tangent)) else {
        return;
    };

    for (k, p) in [c1, c2].into_iter().enumerate() {
        let depth = -(p - v11).dot(normal);
        if depth < -ACTIVATION {
            continue;
        }
        let feature = ((edge as u32) << 8)
            | ((incident_edge as u32) << 4)
            | (k as u32)
            | if reference_is_b { 1 << 16 } else { 0 };
        out.push(Touch {
            point: p,
            normal: if reference_is_b { -normal } else { normal },
            depth,
            feature,
        });
    }
}

/// Keeps the part of segment `p1 p2` with `axis · p <= offset`.
fn clip(p1: Vec2, p2: Vec2, axis: Vec2, offset: f64) -> Option<(Vec2, Vec2)> {
    let (d1, d2) = (p1.dot(axis) - offset, p2.dot(axis) - offset);
    match (d1 <= 0.0, d2 <= 0.0) {
        (true, true) => Some((p1, p2)),
        (false, false) => None,
        (true, false) => Some((p1, p1 + (p2 - p1) * (d1 / (d1 - d2)))),
        (false, true) => Some((p1 + (p2 - p1) * (d1 / (d1 - d2)), p2)),
    }
}

/// The edge of `a` along whose outward normal `b` is least overlapping, and that separation.
fn max_separation(a: &[Vec2], b: &[Vec2]) -> (usize, f64) {
    (0..a.len())
        .map(|i| {
            let normal = outward_normal(a, i);
            let separation = b
                .iter()
                .map(|&v| (v - a[i]).dot(normal))
                .fold(f64::INFINITY, f64::min);
            (i, separation)
        })
        .max_by(|x, y| x.1.total_cmp(&y.1))
        .unwrap()
}

fn outward_normal(vertices: &[Vec2], edge: usize) -> Vec2 {
    let (p, q) = (vertices[edge], vertices[(edge + 1) % vertices.len()]);
    -(q - p).perp().normalized()
}

fn closest_point_on_segment(p: Vec2, q: Vec2, x: Vec2) -> Vec2 {
    let edge = q - p;
    let t = ((x - p).dot(edge) / edge.length_squared()).clamp(0.0, 1.0);
    p + edge * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_resting_on_floor_has_two_point_manifold() {
        let plane = HalfPlane::new(Vec2::Y, 0.0);
        let vertices = world_vertices(&Polygon::rectangle(1.0, 0.5), Vec2::new(0.0, 0.25), 0.0);
        let mut out = Vec::new();
        polygon_halfplane(&plane, &vertices, &mut out);
        assert_eq!(out.len(), 2);
        assert!(out
            .iter()
            .all(|t| t.depth.abs() < 1e-12 && t.normal == Vec2::Y));
    }

    #[test]
    fn overlapping_boxes_touch_along_the_shallow_axis() {
        let a = world_vertices(&Polygon::rectangle(2.0, 2.0), Vec2::ZERO, 0.0);
        let b = world_vertices(&Polygon::rectangle(2.0, 2.0), Vec2::new(1.9, 0.3), 0.0);
        let mut out = Vec::new();
        polygon_polygon(&a, &b, &mut out);
        assert_eq!(out.len(), 2);
        for t in &out {
            assert!((t.normal - Vec2::X).length() < 1e-12, "{:?}", t.normal);
            assert!((t.depth - 0.1).abs() < 1e-12, "{}", t.depth);
        }
    }

    #[test]
    fn circle_near_polygon_corner() {
        let square = world_vertices(&Polygon::rectangle(2.0, 2.0), Vec2::ZERO, 0.0);
        let mut out = Vec::new();
        polygon_circle(&square, &Circle::new(0.5), Vec2::new(1.3, 1.3), &mut out);
        assert_eq!(out.len(), 1);
        let t = &out[0];
        assert!((t.normal - Vec2::new(1.0, 1.0).normalized()).length() < 1e-12);
        assert!((t.depth - (0.5 - 0.3 * 2f64.sqrt())).abs() < 1e-12);
    }
}
