import numpy as np
import pytest

from soliton import Simulation, kdv_soliton, nls_bright_soliton


def test_kdv_soliton_tracks_exact_solution():
    sim = Simulation("kdv", points=512, length=60.0, dt=1e-3)
    sim.state = kdv_soliton(sim.x, 0.0, 4.0, -10.0)
    sim.step(5000)
    exact = kdv_soliton(sim.x, sim.time, 4.0, -10.0)
    assert np.abs(sim.state - exact).max() < 1e-6


def test_nls_soliton_tracks_exact_solution():
    sim = Simulation("nls", points=512, length=40 * np.pi, dt=1e-3)
    sim.state = nls_bright_soliton(sim.x, 0.0, 1.0, 1.0, -8.0)
    sim.step(10000)
    exact = nls_bright_soliton(sim.x, sim.time, 1.0, 1.0, -8.0)
    assert np.abs(sim.state - exact).max() < 1e-6


@pytest.mark.parametrize("model", ["kdv", "nls"])
def test_invariants_are_conserved(model):
    sim = Simulation(model, points=512, length=60.0, dt=1e-3)
    if model == "kdv":
        sim.state = kdv_soliton(sim.x, 0.0, 4.0, -15.0) + kdv_soliton(sim.x, 0.0, 1.0, 0.0)
    else:
        sim.state = nls_bright_soliton(sim.x, 0.0, 1.0, 0.0, -10.0)
    before = sim.invariants()
    sim.step(4000)
    after = sim.invariants()
    for name in before:
        assert abs(after[name] - before[name]) / max(abs(before[name]), 1.0) < 1e-8


def test_unknown_model_raises():
    with pytest.raises(ValueError):
        Simulation("heat", 64, 1.0, 1e-3)
