import pytest

from soliton import magnetism


def test_catalogue_covers_three_chapters():
    specs = magnetism.catalogue()
    assert len(specs) == 13
    assert {s["syllabus"]["chapter"] for s in specs} == {
        "Moving charges and magnetism",
        "Electromagnetic induction",
        "Alternating current",
    }


def test_biot_savart_agrees_with_the_formula_beside_a_wire():
    wire = magnetism.scenario("straight-wire-field", current=5, distance=5)
    (name, expected), = wire.answers().items()
    assert wire.still and expected == pytest.approx(20.0)
    assert wire.measured()[name] == pytest.approx(expected, rel=1e-3)
    assert wire.picture()["kind"] == "field"


def test_a_rod_on_rails_reaches_its_terminal_speed():
    run = magnetism.scenario("motional-emf")
    while not run.finished:
        run.advance(0.05)
    name = "Terminal speed, FR/B²l² (m/s)"
    assert run.answers()[name] == pytest.approx(2.5)
    assert run.measured()[name] == pytest.approx(2.5, rel=2e-3)


def test_unknown_names_are_refused():
    with pytest.raises(ValueError, match="no parameter"):
        magnetism.scenario("faradays-law", colour=1)
    with pytest.raises(ValueError, match="unknown scenario"):
        magnetism.scenario("no-such-thing")
