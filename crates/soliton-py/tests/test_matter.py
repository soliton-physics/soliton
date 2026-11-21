import pytest

from soliton import matter


def test_catalogue_covers_three_chapters():
    specs = matter.catalogue()
    assert len(specs) == 16
    assert {s["syllabus"]["chapter"] for s in specs} == {
        "Mechanical properties of solids",
        "Mechanical properties of fluids",
        "Thermal properties of matter",
    }


def test_ice_floats_with_nine_tenths_under():
    run = matter.scenario("buoyancy", block=920, liquid=1030)
    while not run.finished:
        run.advance(0.02)
    (name, expected), = run.answers().items()
    assert expected == pytest.approx(89.32, abs=0.01)
    assert run.measured()[name] == pytest.approx(expected, abs=0.1)


def test_a_law_evaluated_has_nothing_to_play():
    wire = matter.scenario("stress-strain", load=10)
    assert wire.still and wire.answers()["Extension, MgL/AY (mm)"] == pytest.approx(1.248, abs=1e-3)
    assert wire.picture()["kind"] == "wire"
    with pytest.raises(ValueError, match="no parameter"):
        matter.scenario("stress-strain", colour=1)
    with pytest.raises(ValueError, match="unknown scenario"):
        matter.scenario("no-such-thing")


def test_a_tank_empties_on_time():
    run = matter.scenario("torricelli")
    while not run.finished:
        run.advance(0.05)
    name = "Time to empty, (A/a)√(2h/g) (s)"
    assert run.measured()[name] == pytest.approx(run.answers()[name], rel=0.01)
