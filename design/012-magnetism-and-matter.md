# 012: Gauss's law, bar magnets and matter

Status: implemented. Completes NCERT class 12 chapter 1 (electric charges and fields) and adds chapter 5
(magnetism and matter). Builds on [011](011-electromagnetism.md).

## Scope

Three things, each still a body moved by a force and so still mechanics:

1. Fixed spreads of charge whose field Gauss's law gives: a long wire, a large sheet, a spherical shell.
2. The electric flux through a closed surface, found by adding it up.
3. Bar magnets, a uniform magnetic field in the plane of the page, and samples of matter that a field magnetises.

Electromagnetic waves (chapter 8) are not here: the playground draws them from their formulas, as it does the
class 11 waves.

## Fixed charge

`Electrostatics` holds a list of `Distribution`s, the way `Gravitation` holds fixed masses.

| Distribution | Field | Potential |
|---|---|---|
| `Wire { at, density }`, through the page | `λ / 2πε₀r`, away from it | `−(λ/2πε₀) ln r`, zero at 1 m |
| `Sheet { point, normal, density }`, edge on | `σ / 2ε₀`, away from it on both sides | `−σ|d| / 2ε₀`, zero on the sheet |
| `Shell { centre, radius, charge }` | nothing inside, `kQ/r²` outside | `kQ / max(r, R)` |

A wire and a sheet reach to infinity, so their potential cannot be zero there; the zeros above are conventions and
only differences matter. A body passes through a sheet or a shell freely: they are sources of field, not bodies.
The force on a body is discontinuous as it crosses either one, which costs the integrator one first-order step
there.

## Flux

`World::electric_flux(centre, radius)` is `∮ E·dA` over a sphere centred in the page. It is summed, not looked
up: the upper hemisphere is cut into 160 rings of equal height and 256 points round each (equal areas, so every
patch weighs the same), the field at each is evaluated in three dimensions from the charges and distributions,
and the sum is doubled, since everything is symmetric about the page. For charges not close to the surface it
agrees with `q/ε₀` to better than `1e-3`. Uniform fields contribute nothing, correctly for one that fills space;
a field confined to a region is not a field Gauss's law applies to and is left out.

A scene names one such surface with `"surface": {"centre", "radius"}`; front ends draw it and report the flux.

## Field lines, revised

`field_lines` now takes `Seed`s (a point, a signed strength, and a radius for a shell) and, after tracing from
every positive seed, traces backwards from every negative one and keeps the lines that came from far away and not
from a positive seed. This replaces the earlier rule of thumb for unequal charges and draws the far halves of
loops that leave the picture.

In a scene with a fixed distribution the lines are those of the fixed charge alone. The same function draws
magnetic lines, seeded on the poles of the fixed magnets.

## Magnets

A body may carry a `Magnet { moment, length }`: two poles of strength `±m/length`, `length` apart along the
body's x axis, north at `+x`. `Magnetostatics` acts pole on pole by an inverse-square law with `μ₀/4π`, between
every two magnets and from a uniform `background` field in the plane (the earth's). A static magnet is felt and
not moved. The force reports as `magnetic` under a new label, `Magnetostatic`, which unlike the force on a moving
charge has a potential and does work.

Why poles: outside the magnet the field of two poles is exactly that of the magnet's real currents for a thin
uniformly magnetised bar, it reduces to a dipole far away, and it gives the textbook's two formulas (on the axis
and beside the middle) exactly. Torque in a uniform field, `m × B`, and the energy `−m·B` come out without being
put in. Inside the magnet the pole picture gives `H`, not `B`, and points the wrong way; nothing is read there.

`World::magnetic_field_in_plane(point)` is the field of everything; `magnetic_field_on(body)` leaves out the
body's own magnet (what a compass reads); `fixed_magnetic_field(point)` is that of the static magnets and the
background, which is what the lattice of needles and the field lines draw.

## Matter

A body with a `polarisability` `α` takes on the moment `αB` of the field at its centre and feels
`F = (m·∇)B`, towards stronger field for `α > 0` and away for `α < 0`, with the reaction applied to the poles
that made the field. Its energy is `−½αB²`. A scene gives `{"susceptibility", "volume"}` and the engine uses the
sphere's `α = 3χ/(3 + χ) · V/μ₀`.

## Scenarios

| id | Shows | Checked against |
|---|---|---|
| `gauss-law` | flux through a movable surface | `q/ε₀` round one charge, both, neither |
| `charged-wire` | `1/r` field; circling speed independent of distance | circle, speed, energy at two radii |
| `charged-sheet` | field independent of distance | `½at²` at two distances; return of an opposite charge |
| `charged-shell` | no field inside, a point charge outside | unchanged velocity inside; energy outside |
| `bar-magnet-field` | field lines; axial field twice the equatorial | the two formulas; a compass at rest along the field |
| `compass-needle` | `T = 2π√(I/mB)` | period, fastest spin, centre at rest |
| `neutral-point` | magnet and earth cancelling | zero field at the predicted points, both ways round |
| `magnetic-materials` | diamagnet pushed out, paramagnet pulled in | force at the start; energy |

## Known limits

- **Two kinds of magnetic field do not mix.** `MagneticField` (011) is perpendicular to the page and acts on
  moving charges; `Magnetostatics` is in the page and acts on magnets. An in-plane field would push a moving
  charge out of the page, which the engine cannot follow, so it does not act on charges at all.
- **Induced samples do not act on one another** and take their moment from the field at their centre: right for
  a sample small beside the distance to the pole.
- **No ferromagnetism**: no saturation, no hysteresis, no magnet weakened by another.
- **The flux quadrature is fixed**: a charge within a few per cent of the radius from the surface is counted
  inexactly.
- **A magnet's poles sit where the scene puts them**; the scenarios put them at the ends, as the textbook
  formulas do. A real bar's are a little in from the ends.
