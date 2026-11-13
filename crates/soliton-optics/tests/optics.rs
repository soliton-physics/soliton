//! Ray optics (design 013): the laws at a single surface, and every scenario's measured answers against the
//! textbook's.

use soliton_optics::scenario::{catalogue, find, Params};
use soliton_optics::scene::OpticsScene;
use soliton_optics::{Bench, Curve, Element, End, EventKind, Ray, Vec2};

fn relative(measured: f64, expected: f64) -> f64 {
    (measured - expected).abs() / expected.abs().max(1e-9)
}

fn block(index: f64) -> Element {
    let corners = [
        Vec2::new(-10.0, -10.0),
        Vec2::new(10.0, -10.0),
        Vec2::new(10.0, 0.0),
        Vec2::new(-10.0, 0.0),
    ];
    Element::Glass {
        outline: (0..4)
            .map(|i| Curve::Segment {
                a: corners[i],
                b: corners[(i + 1) % 4],
            })
            .collect(),
        index,
    }
}

/// A ray coming down onto the origin at `degrees` to the vertical.
fn down_at(degrees: f64) -> Ray {
    let angle = degrees.to_radians();
    Ray::towards(Vec2::new(-angle.sin(), angle.cos()), Vec2::ZERO)
}

#[test]
fn a_mirror_reflects_at_the_angle_of_incidence() {
    let mirror = Element::Mirror(Curve::Segment {
        a: Vec2::new(-1.0, 0.0),
        b: Vec2::new(1.0, 0.0),
    });
    let bench = Bench::new(vec![mirror]);
    for degrees in [0.0, 17.0, 45.0, 80.0] {
        let path = bench.trace(down_at(degrees), 2.0);
        let event = path.events[0];
        assert_eq!(event.kind, EventKind::Reflection);
        assert!((event.incidence.to_degrees() - degrees).abs() < 1e-10);
        assert!((event.emergence - event.incidence).abs() < 1e-12);
        // It leaves on the other side of the normal, going up.
        assert!(path.last.y > 0.0 || degrees == 90.0);
        assert!((path.last.x - degrees.to_radians().sin()).abs() < 1e-12);
    }
}

#[test]
fn refraction_obeys_snells_law_in_both_directions() {
    let bench = Bench::new(vec![block(1.5)]);
    for degrees in [5.0, 30.0, 60.0, 85.0] {
        let path = bench.trace(down_at(degrees), 2.0);
        let event = path.events[0];
        assert_eq!(event.kind, EventKind::Refraction);
        assert!(relative(1.5 * event.emergence.sin(), event.incidence.sin()) < 1e-12);
        // Sent back along the way it came, light retraces its path.
        let back = bench.trace(Ray::new(*path.points.last().unwrap(), -path.last), 3.0);
        assert!((back.last + path.first).length() < 1e-12);
    }
}

#[test]
fn light_is_totally_reflected_beyond_the_critical_angle_and_not_before() {
    let bench = Bench::new(vec![block(1.5)]);
    let critical = (1.0f64 / 1.5).asin().to_degrees();
    let from_below = |degrees: f64| {
        let angle = degrees.to_radians();
        bench.trace(
            Ray::towards(Vec2::new(-angle.sin(), -angle.cos()), Vec2::ZERO),
            3.0,
        )
    };
    assert_eq!(
        from_below(critical - 0.01).events[0].kind,
        EventKind::Refraction
    );
    let grazing = from_below(critical - 0.01).events[0].emergence.to_degrees();
    assert!(
        grazing > 88.0,
        "just inside the critical angle it left at {grazing}°"
    );
    let turned = from_below(critical + 0.01);
    assert_eq!(turned.events[0].kind, EventKind::TotalReflection);
    assert!((turned.events[0].emergence - turned.events[0].incidence).abs() < 1e-12);
    assert!(turned.last.y < 0.0);
}

#[test]
fn the_index_is_that_of_the_innermost_glass() {
    let inner = Element::Glass {
        outline: vec![Curve::Arc {
            centre: Vec2::new(0.0, -1.0),
            radius: 0.5,
            from: 0.0,
            sweep: std::f64::consts::TAU,
        }],
        index: 2.0,
    };
    let bench = Bench::new(vec![block(1.5), inner]);
    assert_eq!(bench.index_at(Vec2::new(0.0, 1.0)), 1.0);
    assert_eq!(bench.index_at(Vec2::new(3.0, -1.0)), 1.5);
    assert_eq!(bench.index_at(Vec2::new(0.2, -1.1)), 2.0);
}

#[test]
fn a_ray_meets_only_the_part_of_a_circle_that_is_there() {
    // A quarter circle in the first quadrant.
    let arc = Curve::Arc {
        centre: Vec2::ZERO,
        radius: 1.0,
        from: 0.0,
        sweep: std::f64::consts::FRAC_PI_2,
    };
    let hits = arc.hits(Vec2::new(-2.0, 0.5), Vec2::new(1.0, 0.0));
    assert_eq!(hits.len(), 1);
    assert!((hits[0].distance - (2.0 + 0.75f64.sqrt())).abs() < 1e-12);
    assert!(arc
        .hits(Vec2::new(-2.0, -0.5), Vec2::new(1.0, 0.0))
        .is_empty());
    // Described the other way round, it is the same arc.
    let reversed = Curve::Arc {
        centre: Vec2::ZERO,
        radius: 1.0,
        from: std::f64::consts::FRAC_PI_2,
        sweep: -std::f64::consts::FRAC_PI_2,
    };
    assert_eq!(
        reversed
            .hits(Vec2::new(-2.0, 0.5), Vec2::new(1.0, 0.0))
            .len(),
        1
    );
}

#[test]
fn an_ideal_lens_images_every_ray_to_the_same_point() {
    let lens = Element::ThinLens {
        centre: Vec2::ZERO,
        axis: Vec2::new(1.0, 0.0),
        focal: 0.2,
        aperture: 0.3,
    };
    let bench = Bench::new(vec![lens]);
    let object = Vec2::new(-0.5, 0.04);
    // 1/v − 1/u = 1/f gives v = 1/3 m and m = −2/3.
    let image = Vec2::new(1.0 / 3.0, -0.04 * 2.0 / 3.0);
    for height in [-0.25, -0.1, 0.0, 0.13, 0.29] {
        let path = bench.trace(Ray::towards(object, Vec2::new(0.0, height)), 1.0);
        let offset = (image - path.last_turn()).cross(path.last);
        assert!(
            offset.abs() < 1e-12,
            "the ray through {height} missed by {offset}"
        );
    }
    let found = bench.image(object, Vec2::ZERO, 1.0).unwrap();
    assert!(found.real && (found.at - image).length() < 1e-9);
}

#[test]
fn a_spherical_mirror_focuses_rays_near_the_axis_and_not_those_far_from_it() {
    let radius = 0.4;
    let mirror = Element::Mirror(Curve::cap(Vec2::ZERO, Vec2::new(-1.0, 0.0), radius, 0.3));
    let bench = Bench::new(vec![mirror]);
    let crosses_axis_at = |height: f64| {
        let path = bench.trace(Ray::new(Vec2::new(-1.0, height), Vec2::new(1.0, 0.0)), 2.0);
        let turn = path.last_turn();
        turn.x - turn.y * path.last.x / path.last.y
    };
    assert!(relative(crosses_axis_at(1e-4), -0.5 * radius) < 1e-6);
    // A ray at half the radius of curvature from the axis comes back through a point well short of the focus.
    let far = crosses_axis_at(0.2);
    assert!(
        far > -0.5 * radius + 0.02,
        "a marginal ray crossed at {far}"
    );
}

#[test]
fn rays_between_facing_mirrors_are_given_up_on() {
    let wall = |x: f64| {
        Element::Mirror(Curve::Segment {
            a: Vec2::new(x, -1.0),
            b: Vec2::new(x, 1.0),
        })
    };
    let bench = Bench::new(vec![wall(-1.0), wall(1.0)]);
    let path = bench.trace(Ray::new(Vec2::ZERO, Vec2::new(1.0, 0.001)), 5.0);
    assert_eq!(path.end, End::Exhausted);
}

/// The measured answers of a scenario, as `(name, textbook, measured)`.
fn measured(id: &str, params: &Params) -> Vec<(String, f64, f64)> {
    let traces = find(id).unwrap().scene(params).trace().unwrap();
    traces
        .answers
        .into_iter()
        .filter_map(|(name, value, measured)| measured.map(|m| (name, value, m)))
        .collect()
}

fn agree(id: &str, params: &Params, tolerance: f64) {
    let answers = measured(id, params);
    assert!(!answers.is_empty(), "{id}: nothing was measured");
    for (name, value, found) in answers {
        // The answer that is there to show a spherical lens missing its focus is meant to disagree.
        if name.contains("outermost") {
            continue;
        }
        let off = (found - value).abs() / value.abs().max(1.0);
        assert!(off < tolerance, "{id}, {name}: {found} against {value}");
    }
}

#[test]
fn every_scenario_measures_what_the_textbook_predicts() {
    for scenario in catalogue() {
        let tolerance = match scenario.spec.id {
            // A lens with real thickness against the thin-lens formula.
            "lens-maker" => 2e-2,
            // Two lenses a hair apart are not quite one lens.
            "lenses-in-contact" => 1e-4,
            _ => 1e-6,
        };
        agree(scenario.spec.id, &Params::default(), tolerance);
    }
}

#[test]
fn the_mirror_formula_holds_wherever_the_object_is() {
    for distance in [80.0, 30.0, 22.0, 10.0, 5.0] {
        let params = Params::default().set("distance", distance);
        agree("concave-mirror", &params, 1e-6);
        agree("convex-mirror", &params, 1e-6);
    }
    // Inside the focus of a concave mirror the image is behind it, upright and enlarged.
    let inside = measured("concave-mirror", &Params::default().set("distance", 10.0));
    assert!(inside[0].2 > 0.0 && inside[1].2 > 1.0, "{inside:?}");
    let convex = measured("convex-mirror", &Params::default().set("distance", 60.0));
    assert!(convex[0].2 > 0.0 && convex[0].2 < 15.0 && convex[1].2 > 0.0 && convex[1].2 < 1.0);
}

#[test]
fn the_lens_formula_holds_wherever_the_object_is() {
    for distance in [90.0, 30.0, 20.0, 9.0, 4.0] {
        let params = Params::default().set("distance", distance);
        agree("convex-lens", &params, 1e-6);
        agree("concave-lens", &params, 1e-6);
    }
    let inside = measured("convex-lens", &Params::default().set("distance", 9.0));
    assert!(inside[0].2 < 0.0 && inside[1].2 > 1.0, "{inside:?}");
    // With the object at the focus the rays leave parallel and there is no image to measure.
    let at_focus = find("convex-lens")
        .unwrap()
        .scene(&Params::default().set("distance", 15.0))
        .trace()
        .unwrap();
    assert!(at_focus.images[0].is_none());
}

#[test]
fn refraction_scenarios_hold_across_their_sliders() {
    for angle in [0.0, 20.0, 70.0, 89.0] {
        agree("snells-law", &Params::default().set("angle", angle), 1e-9);
        agree(
            "glass-slab",
            &Params::default().set("angle", angle.min(85.0)),
            1e-9,
        );
    }
    for (depth, index) in [(5.0, 1.33), (40.0, 1.33), (20.0, 2.4)] {
        agree(
            "apparent-depth",
            &Params::default().set("depth", depth).set("index", index),
            1e-6,
        );
    }
    for distance in [100.0, 25.0, 12.0] {
        agree(
            "spherical-surface",
            &Params::default().set("distance", distance),
            1e-6,
        );
    }
    // Glass above water: below the critical angle the ray gets out, above it nothing is measured.
    let dense = Params::default().set("above", 1.5).set("below", 1.0);
    agree("snells-law", &dense.clone().set("angle", 30.0), 1e-9);
    assert!(measured("snells-law", &dense.set("angle", 50.0)).is_empty());
}

#[test]
fn a_prism_deviates_least_when_the_ray_goes_through_symmetrically() {
    let scenario = find("prism").unwrap();
    let deviation = |angle: f64| {
        let answers = measured("prism", &Params::default().set("angle", angle));
        answers
            .iter()
            .find(|a| a.0.starts_with("Deviation"))
            .unwrap()
            .2
    };
    let traces = scenario.scene(&Params::default()).trace().unwrap();
    let answer = |name: &str| {
        traces
            .answers
            .iter()
            .find(|a| a.0.starts_with(name))
            .unwrap()
            .1
    };
    let (least, at) = (
        answer("Least deviation"),
        answer("Angle of incidence that gives it"),
    );
    assert!(relative(deviation(at), least) < 1e-9);
    assert!(deviation(at - 6.0) > least && deviation(at + 6.0) > least);
    for angle in [35.0, 49.0, 70.0, 85.0] {
        agree("prism", &Params::default().set("angle", angle), 1e-9);
    }
}

#[test]
fn a_fibre_carries_light_inside_its_acceptance_angle_and_leaks_it_outside() {
    let ends_inside = |angle: f64| {
        let scene = find("optical-fibre")
            .unwrap()
            .scene(&Params::default().set("angle", angle));
        let traces = scene.trace().unwrap();
        let path = &traces.rays[0].path;
        let kinds: Vec<EventKind> = path.events.iter().map(|e| e.kind).collect();
        (kinds, traces.answers)
    };
    let (kinds, answers) = ends_inside(20.0);
    let widest = answers
        .iter()
        .find(|a| a.0.starts_with("Widest"))
        .unwrap()
        .1;
    assert!(widest > 20.0 && widest < 40.0, "{widest}");
    // In at the end face, then nothing but total reflections off the wall of the core, and out at the far end.
    assert_eq!(kinds[0], EventKind::Refraction);
    let bounces = &kinds[1..kinds.len() - 1];
    assert!(bounces.len() >= 4 && bounces.iter().all(|k| *k == EventKind::TotalReflection));
    let (kinds, _) = ends_inside(widest + 5.0);
    assert_eq!(
        kinds[1],
        EventKind::Refraction,
        "it should leak into the cladding"
    );
}

#[test]
fn instruments_magnify_as_their_formulas_say() {
    for (objective, eyepiece) in [(20.0, 2.0), (80.0, 10.0), (40.0, 5.0)] {
        let params = Params::default()
            .set("objective", objective)
            .set("eyepiece", eyepiece);
        agree("telescope", &params, 1e-9);
    }
    for beyond in [0.2, 0.5, 2.0] {
        agree(
            "compound-microscope",
            &Params::default().set("beyond", beyond),
            1e-6,
        );
    }
    let lenses = Params::default().set("first", 20.0).set("second", -40.0);
    agree("lenses-in-contact", &lenses, 1e-4);
}

#[test]
fn a_real_lens_brings_a_wide_beam_to_a_focus_short_of_the_paraxial_one() {
    let answers = measured("lens-maker", &Params::default().set("width", 9.0));
    let (paraxial, marginal) = (answers[0].2, answers[1].2);
    assert!(relative(paraxial, 30.0) < 2e-2, "{paraxial}");
    assert!(
        marginal < paraxial - 0.3,
        "marginal {marginal}, paraxial {paraxial}"
    );
}

#[test]
fn scenes_survive_a_round_trip_through_json() {
    for scenario in catalogue() {
        let scene = scenario.scene(&Params::default());
        let back = OpticsScene::from_json(&scene.to_json()).unwrap();
        assert_eq!(scene, back, "{}", scenario.spec.id);
    }
    assert!(OpticsScene::from_json("{\"schema\": 1, \"title\": \"x\", \"ambient\": 0}").is_err());
}
