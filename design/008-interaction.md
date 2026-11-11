# 008: A hand on the physics

Status: Implemented

## Scope

What lets a person take part in a running simulation rather than watch it: taking hold of a body with the pointer,
picking the body under the pointer, and writing the world down as it stands so it can be edited and carried on from.
Editing a scene is otherwise a front-end matter: the scene is data (004), so the playground changes the JSON and
builds a new world.

## The hand

`World::grab(body, world_point, target)` ties a point of a dynamic body to a target with a critically damped spring;
`move_grab(target)` moves the target and `release()` lets go. One body can be held at a time.

```
F = k (target − p) − c v_p,     k = m ω²,   c = 2 m ω,   ω = 30 rad/s
```

- **It is a force, not a teleport.** The body is pulled, so everything it is attached to or resting on responds
  through the ordinary solve. A held pendulum bob still swings on its rod; a block pulled sideways still feels
  friction. Setting positions directly would break constraints and energy accounting.
- **Scaled by mass.** Stiffness and damping carry a factor `m`, so a heavy body and a light one follow the hand in
  the same way and the response is critically damped for a free body.
- **Bounded.** The spring stops stretching at 1.5 m. A pointer flung across the screen cannot launch a body at an
  arbitrary speed.
- **Reported.** The force is labelled `applied`, drawn in the free-body diagram at the held point, and its work goes
  to `external_work` in the energy ledger like any other applied force. A body held still in the air shows `applied`
  equal and opposite to gravity; the spring then sags by `g / ω²`, about 11 mm.
- **Integration.** The damping term depends on velocity, so the closing kick is solved by the same fixed-point
  iteration as drag (001) while a body is held. The hand moves between steps, so loads are re-evaluated at the start
  of each step after it changes, as for forces with a time window. The ledger's drift while dragging is second order
  in `ω dt`, about 1e-3 of the work done at `dt = 1e-3`; released, it is back to the integrator's own.

Rejected: a kinematic drag (position follows the pointer exactly) is simpler but reports no force and injects energy
the ledger cannot see. A velocity-level "mouse joint" solved with the constraints is stiffer but would be a second
kind of constraint row with a soft limit; the spring needs nothing new in the solver.

## Picking

`World::body_at(point, slop)` returns the dynamic body whose shape contains the point, the most recently added one
winning, and otherwise the nearest dynamic body whose centre is within `slop` metres. The slop keeps a particle drawn
as a dot easy to pick; the front end passes a few pixels' worth of metres.

## Snapshots

`SceneFile::snapshot(world, time)` is the scene as it stands part-way through a run: the same description with each
body's current position, angle and velocities, and with every length that was measured at the start written out.

- A rope or rod without a length starts taut, and a spring without a rest length starts relaxed. Rebuilt at later
  positions those would measure something else, so the snapshot writes down what was measured at `t = 0`.
- A rope on a wheel pays out as the wheel turns. Its length is measured afresh from the live world: the path now,
  plus whatever slack it has now.
- Times count from the snapshot: force windows and time rules shift by `time`, and those already past are dropped.
  A driving force keeps its phase.

Not carried over: the solver's memory of which contacts were sliding and their warm-start impulses. A scene resumed
from a snapshot agrees with the original run to about 1e-6 m after 200 steps in every catalogue scenario, which the
tests check.

Polygon vertices may be written in either winding; the body is placed by the centroid of the polygon as drawn.

## WebAssembly surface

`grab(body, x, y)`, `drag_to(x, y)`, `release()`, `grabbed()` (`{ body, point, target }` or `null`),
`body_at(x, y, slop)` and `scene_json_now()`.

## Validation

| Check | Tolerance |
| --- | --- |
| a held ball settles `g / ω²` below the hand with `applied = −m g` | 1e-8 |
| energy ledger while the hand drags a ball along a moving path | 1e-3 of the work done |
| a released ball keeps the hand's velocity and is in free fall | exact |
| static bodies cannot be held; picking follows shapes and slop | exact |
| every catalogue scenario resumed from a snapshot at 0.4 s | 1e-6 m after 200 steps |
| time windows and time rules shift and expire in a snapshot | exact |
| a polygon lands where it is drawn in either winding | 1e-12 |
