import cmath
import math

import numpy as np
import pytest

from soliton import circuits


def test_catalogue_covers_four_chapters():
    chapters = {s["syllabus"]["chapter"] for s in circuits.catalogue()}
    assert "Current electricity" in chapters and "Alternating current" in chapters
    assert len(chapters) == 4


def test_a_balanced_bridge_passes_no_current():
    bridge = circuits.scenario("wheatstone-bridge", s=30)
    assert abs(bridge.current("G")) < 1e-12
    off = circuits.scenario("wheatstone-bridge", s=20)
    assert off.current("G") == pytest.approx(8.76e-3, rel=1e-3)
    assert off.measured() == pytest.approx(
        {k: v for k, v in off.answers().items() if k in off.measured()}
    )


def test_a_cell_supplying_current_has_a_negative_one():
    cell = circuits.scenario("internal-resistance", emf=1.5, internal=0.5, load=2.5)
    assert cell.current("cell") == pytest.approx(-0.5)
    assert cell.voltage("cell") == pytest.approx(1.25)


def test_a_capacitor_charges_exponentially():
    rc = circuits.scenario("charging-capacitor", emf=9, resistance=10, capacitance=100)
    rc.advance(2.0)
    t, v, i = rc.series("C")
    # Time moves in whole steps of three milliseconds here, so two seconds is reached to the nearest one.
    assert t[-1] == pytest.approx(2.0, abs=2e-3)
    assert np.allclose(v, 9 * (1 - np.exp(-t / 1.0)), atol=1e-5)
    assert i[0] == pytest.approx(9 / 10e3)
    assert not rc.finished


def test_resonance_makes_the_circuit_a_plain_resistance():
    f0 = 1 / (2 * math.pi * math.sqrt(0.2 * 100e-6))
    lcr = circuits.scenario("series-lcr", frequency=f0)
    v, i = lcr.phasor("source")
    current = lcr.phasor("A")[1]
    assert abs(current) == pytest.approx(100 / 20, rel=1e-3)
    assert abs(cmath.phase(v / current)) < 1e-2
    with pytest.raises(ValueError, match="no component"):
        lcr.phasor("nothing")


def test_bad_scenes_are_refused():
    with pytest.raises(ValueError, match="unknown scenario"):
        circuits.scenario("no-such-thing")
    steady = circuits.scenario("ohms-law")
    with pytest.raises(ValueError, match="not an alternating"):
        steady.phasor("R")
    assert circuits.Circuit.from_json(steady.to_json()).current("A") == pytest.approx(0.5)
