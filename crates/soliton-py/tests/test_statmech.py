import math

import numpy as np
import pytest

from soliton import statmech


def test_onsager_values():
    assert statmech.CRITICAL_TEMPERATURE == pytest.approx(2.269185, rel=1e-6)
    assert statmech.onsager_magnetisation(3.0) == 0.0
    assert statmech.onsager_magnetisation(2.0) == pytest.approx(0.911319, rel=1e-5)
    assert statmech.onsager_energy(statmech.CRITICAL_TEMPERATURE) == pytest.approx(-math.sqrt(2), rel=1e-5)


def test_a_cold_lattice_is_magnetised_as_onsager_says():
    run = statmech.scenario("ising-model", temperature=1.8, size=32)
    run.advance(1500)
    spins = run.spins()
    assert spins.shape == (32, 32) and set(np.unique(spins)) <= {-1, 1}
    name = "Magnetisation per spin, Onsager's formula"
    assert run.measured()[name] == pytest.approx(run.answers()[name], abs=0.03)
    assert run.history().shape[1] == 3


def test_darts_and_walkers():
    darts = statmech.scenario("monte-carlo-pi", batch=2000)
    darts.advance(200)
    assert darts.measured()["π"] == pytest.approx(math.pi, abs=0.02)
    walk = statmech.scenario("random-walk", walkers=3000, steps=400)
    while not walk.finished:
        walk.advance(100)
    distance = np.hypot(*walk.walkers().T)
    assert math.sqrt((distance**2).mean()) == pytest.approx(20.0, rel=0.05)
    with pytest.raises(ValueError, match="no lattice"):
        walk.spins()
    with pytest.raises(ValueError, match="unknown scenario"):
        statmech.scenario("no-such-thing")


def test_its_run_is_not_the_mechanics_one():
    from soliton import mechanics

    assert statmech.Run is not mechanics.Run
    assert hasattr(mechanics.Run, "energy") and hasattr(statmech.Run, "spins")
