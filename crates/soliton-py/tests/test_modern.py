import math

import pytest

from soliton import modern


def test_formulas():
    assert modern.electron_wavelength(54) == pytest.approx(0.1669, abs=1e-4)
    assert modern.bohr_energy(1) == pytest.approx(-13.606, abs=1e-3)
    assert modern.bohr_energy(1, z=2) == pytest.approx(-54.42, abs=0.01)
    assert modern.line_wavelength(3, 2) == pytest.approx(656.3, abs=0.5)
    assert modern.liquid_drop(56, 26) / 56 == pytest.approx(8.79, abs=0.1)
    with pytest.raises(ValueError):
        modern.line_wavelength(2, 2)


def test_an_alpha_follows_rutherfords_formula():
    closest = 2 * 79 * 1.44 / 5.5
    for impact in (20.0, 60.0):
        expected = 2 * math.degrees(math.atan(closest / (2 * impact)))
        assert modern.alpha_deflection(5.5, 79, impact) == pytest.approx(expected, abs=0.2)


def test_a_sample_decays_by_halves():
    run = modern.scenario("radioactive-decay", half_life=2, nuclei=2500)
    while not run.finished:
        run.advance(0.05)
    answers, measured = run.answers(), run.measured()
    assert measured["Left after one half-life, N₀/2"] == pytest.approx(1250, abs=80)
    assert set(measured) == set(answers)
    assert run.picture()["kind"] == "sample"
    assert run.plots()[0]["curves"][1]["name"] == "the sample"


def test_catalogue_and_errors():
    assert len(modern.catalogue()) == 8
    tube = modern.scenario("photoelectric-effect", wavelength=650)
    assert tube.answers()["Electrons freed, at any brightness"] == 0
    with pytest.raises(ValueError, match="unknown scenario"):
        modern.scenario("no-such-thing")
    with pytest.raises(ValueError, match="no parameter"):
        modern.scenario("bohr-model", colour=3)
