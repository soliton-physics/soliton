"""Ray optics: mirrors, glass and lenses, with rays traced exactly.

    from soliton import optics

    bench = optics.scenario("convex-lens", focal=15, distance=40)
    bench.answers()     # {'Image distance, v (cm)': 24.0, 'Magnification, m = v/u': -0.6, ...}
    bench.measured()    # the same quantities read off the traced rays
    bench.rays()        # list of (N, 2) arrays, metres
    bench.images()      # [{'base': (0.24, 0.0), 'tip': (0.24, -0.018), 'real': True}]

Every ray is turned by the law of reflection or Snell's law where it actually strikes, so a wide beam shows
the aberration of a real surface. Images are the paraxial ones the mirror and lens formulas are about.
"""

import json

from soliton._native import Bench, optics_catalogue_json

__all__ = ["Bench", "catalogue", "scenario"]


def catalogue() -> list[dict]:
    """Every optics scenario: id, title, description, syllabus and parameter schema."""
    return json.loads(optics_catalogue_json())


def scenario(id: str, **params: float) -> Bench:
    """A catalogue scenario with chosen parameters; unset parameters take their defaults."""
    return Bench.scenario(id, **params)
