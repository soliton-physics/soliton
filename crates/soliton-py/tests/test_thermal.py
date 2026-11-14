import numpy as np
import pytest

from soliton import thermal

BOLTZMANN = 1.380649e-23
AMU = 1.66053906660e-27


def test_catalogue_lists_both_chapters():
    chapters = {s["syllabus"]["chapter"] for s in thermal.catalogue()}
    assert chapters == {"Kinetic theory", "Thermodynamics"}


def test_temperature_is_the_mean_kinetic_energy():
    gas = thermal.scenario("ideal-gas", temperature=400, count=200)
    v = gas.velocities()
    assert v.shape == (200, 3)
    kinetic = 0.5 * 39.95 * AMU * (v**2).sum(axis=1).mean()
    assert kinetic == pytest.approx(1.5 * BOLTZMANN * 400, rel=1e-6)
    assert gas.temperature == pytest.approx(400)


def test_an_insulated_gas_keeps_its_temperature_and_obeys_the_gas_law():
    gas = thermal.scenario("ideal-gas")
    gas.advance(600)
    assert gas.time == pytest.approx(600)
    assert gas.temperature == pytest.approx(300, rel=1e-9)
    name = "Pressure, NkT/V (atm)"
    assert gas.measured()[name] == pytest.approx(gas.answers()[name], rel=0.05)
    inside = gas.positions()
    assert inside.min() > -0.2 and inside.max() < 12.2


def test_adiabatic_compression_heats_the_gas():
    gas = thermal.scenario("adiabatic-process")
    while gas.stage < 2:
        gas.advance(100)
    expected = 300 * 2 ** (2 / 3)
    assert gas.temperature == pytest.approx(expected, rel=0.03)
    series = gas.series()
    assert series["volume"][0] == pytest.approx(1600) and series["volume"][-1] == pytest.approx(800)
    # No heat passed, so the work done on the gas is all in its energy.
    assert np.allclose(series["heat"], 0.0)
    assert series["work"][-1] == pytest.approx(series["energy"][-1] - series["energy"][0], rel=1e-9)


def test_scenes_round_trip_and_bad_ones_are_refused():
    gas = thermal.scenario("mixing")
    again = thermal.Gas.from_json(gas.to_json())
    assert set(again.species()) == {0, 1}
    with pytest.raises(ValueError, match="unknown scenario"):
        thermal.scenario("no-such-thing")
