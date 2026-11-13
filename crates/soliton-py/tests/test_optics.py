import json
import math

import numpy as np
import pytest

from soliton import optics


def test_catalogue_lists_the_chapter():
    ids = [s["id"] for s in optics.catalogue()]
    assert "concave-mirror" in ids and "prism" in ids and len(ids) == 15


def test_lens_formula_is_what_the_rays_give():
    bench = optics.scenario("convex-lens", focal=15, distance=40)
    assert bench.answers()["Image distance, v (cm)"] == pytest.approx(24.0)
    assert bench.measured()["Image distance, v (cm)"] == pytest.approx(24.0, rel=1e-9)
    (image,) = bench.images()
    assert image["real"] and image["tip"][1] == pytest.approx(-0.018, rel=1e-6)


def test_snells_law_along_a_ray():
    bench = optics.scenario("snells-law", angle=45, below=1.5)
    ray = bench.rays()[0]
    assert ray.shape == (3, 2)
    out = ray[2] - ray[1]
    refracted = math.degrees(math.atan2(out[0], -out[1]))
    assert 1.5 * math.sin(math.radians(refracted)) == pytest.approx(math.sin(math.radians(45)))
    assert np.allclose(ray[1], 0.0)


def test_scenes_round_trip_and_bad_ones_are_refused():
    bench = optics.scenario("prism")
    again = optics.Bench.from_json(bench.to_json())
    assert again.measured() == bench.measured()
    scene = json.loads(bench.to_json())
    scene["elements"][0]["glass"]["index"] = -1
    with pytest.raises(ValueError, match="refractive index"):
        optics.Bench.from_json(json.dumps(scene))
    with pytest.raises(ValueError, match="unknown scenario"):
        optics.scenario("no-such-thing")
