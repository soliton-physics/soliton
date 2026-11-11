import json
import math

import numpy as np
import pytest

from soliton import mechanics


def test_catalogue_lists_scenarios_in_chapter_order():
    ids = [s["id"] for s in mechanics.catalogue()]
    assert ids[0] == "free-fall"
    assert "atwood" in ids and "simple-pendulum" in ids
    spec = next(s for s in mechanics.catalogue() if s["id"] == "atwood")
    assert {p["name"] for p in spec["params"]} == {"m1", "m2", "height"}


def test_atwood_run_matches_textbook():
    scene = mechanics.scenario("atwood", m1=2, m2=3)
    answers = scene.answers()
    run = scene.run(until=1.0, record_every=1)
    v = run.body("m2")["velocity"]
    assert v.shape[1] == 2
    assert abs(abs(v[-1, 1]) - answers["Acceleration (m/s²)"] * run.time_end) < 1e-8
    tension = run.force("m2", "tension")
    assert abs(np.linalg.norm(tension[-1]) - answers["Tension (N)"]) < 1e-8
    # A rope that is taut from the start is not news; only what changes during the run is an event.
    assert run.events == []


def test_json_round_trip_and_editing(tmp_path):
    scene = mechanics.scenario("simple-pendulum", length=1.0, amplitude=10)
    path = tmp_path / "pendulum.json"
    scene.save(str(path))
    data = json.loads(path.read_text())
    assert data["schema"] == 1 and data["bodies"][0]["id"] == "bob"
    data["world"]["gravity"] = [0, -1.6]
    edited = mechanics.Scene.from_json(json.dumps(data))
    run = edited.run(until=16.0, record_every=1)
    x = run.body("bob")["position"][:, 0]
    crossings = np.flatnonzero((x[:-1] > 0) & (x[1:] <= 0))
    period = (run.t[crossings[-1]] - run.t[crossings[0]]) / (len(crossings) - 1)
    amplitude = math.radians(10)
    expected = 2 * math.pi * math.sqrt(1.0 / 1.6) * (1 + amplitude**2 / 16 + 11 * amplitude**4 / 3072)
    assert abs(period - expected) / period < 1e-4


def test_errors_are_informative():
    with pytest.raises(ValueError, match="unknown scenario"):
        mechanics.scenario("warp-drive")
    with pytest.raises(ValueError, match="schema"):
        mechanics.Scene.from_json('{"schema": 9, "title": "x", "world": {"gravity": [0, 0], "dt": 0.1}, "bodies": []}')
    run = mechanics.scenario("free-fall").run(until=0.1)
    with pytest.raises(ValueError, match="no body named"):
        run.body("rocket")


def test_run_reports_where_the_problem_ends():
    run = mechanics.scenario("free-fall", height=20).run(until=3.0)
    assert run.finished["reason"] in ("ball touched ground", "everything came to rest")
    assert abs(run.finished["time"] - math.sqrt(2 * 20 / 9.8)) < 5e-3
    assert mechanics.scenario("simple-pendulum").run(until=1.0).finished is None
