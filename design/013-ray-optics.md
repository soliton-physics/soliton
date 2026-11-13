# 013: Ray optics

Status: implemented. NCERT class 12, chapter 9: ray optics and optical instruments. A new crate,
`soliton-optics`, because nothing here moves: there is no time, no state and no integrator.

## Scope

Light as rays in a plane. A ray is a straight line that turns only where it meets something, and each turn is
worked out exactly at the point of contact. The crate traces rays, finds images, and carries a catalogue of
scenarios with textbook answers, in the same shape as the mechanics catalogue (design 004).

## The bench

A `Bench` is a list of `Element`s and the refractive index of the space between them.

| Element | What a ray does on meeting it |
|---|---|
| `Mirror(Curve)` | reflects, from either side |
| `Screen(Curve)` | stops |
| `Glass { outline, index }` | refracts by Snell's law, or is totally reflected |
| `ThinLens { centre, axis, focal, aperture }` | has its slope changed by `−height / focal` |

A `Curve` is a segment or an arc of a circle. That is enough for every surface in the chapter: plane and
spherical mirrors, slabs, prisms, lenses and spherical refracting surfaces.

**Glass is a region, not a pair of surfaces.** The index at a point is that of the last glass in the list whose
closed outline contains it (even-odd rule along a fixed probe direction), else the ambient index. On meeting an
outline the tracer samples the index a little before and a little beyond the point and applies Snell's law
between those two values. So a lens need not say which side is inside, a core can sit inside its cladding, and a
ray leaving glass is handled by the same code as one entering.

**Total internal reflection is not a special case.** Snell's law in vector form needs
`1 − (n₁/n₂)² sin² i ≥ 0`; when it is not, the ray is reflected.

**The thin lens is ideal.** In the lens's own frame a ray of slope `s` at height `y` leaves with slope
`s − y/f`. This is the paraxial law applied to every ray, so the lens obeys `1/v − 1/u = 1/f` exactly at any
height and has no aberration. It is what a textbook ray diagram draws, and the instruments need it. Real lenses
are built from `Glass`.

## Tracing

`Bench::trace(ray, reach)` returns a `Path`: the points at which the ray turned, an `Event` for each meeting
(kind, normal, angle of incidence, angle of emergence), how it ended, and its first and last directions. A ray is
followed until it meets nothing within `reach`, is absorbed, or has had 200 meetings (two facing mirrors).
A meeting closer than a nanometre to the last one is ignored, so that a ray does not strike again the surface it
is leaving.

Not modelled: partial reflection at a refracting surface, absorption, polarisation, and any dependence of the
index on colour.

## Images

The mirror and lens formulas are statements about rays close to the axis. `Bench::image(object, towards, reach)`
follows two rays that leave `object` a hair (10⁻⁵ of the distance) either side of the line to `towards`, and
returns where their last stretches cross. It is real when that crossing lies ahead of both rays' last turn and
virtual when it is found only by producing them backwards; `None` when they leave parallel.

For an extended object the scene finds the image of the base, and of a point one thousandth of the way up the
object, and scales the difference up. That is the magnification in the limit of a small object, which is what
`m = −v/u` means. The traced rays from the real tip are still exact, so for a tall object in a spherical mirror
they miss the paraxial image a little: spherical aberration, shown not hidden.

## Scenes and answers

`OpticsScene` is the serialisable description: elements, sources (a ray, a fan from a point, a parallel beam),
objects, named marks, the axis, a view, and answers. Each answer has the textbook value and, optionally, a
`MeasureSpec` saying how to read it off the traced rays: an image position, a magnification, the angle at a given
meeting of a given ray, a ray's total deviation, where two rays cross, or an angular magnification. `trace()`
returns everything a front end draws together with every answer measured, so nothing about the measurement lives
in a front end.

## Catalogue

Fifteen scenarios: `concave-mirror`, `convex-mirror`, `snells-law`, `glass-slab`, `apparent-depth`,
`total-internal-reflection`, `optical-fibre`, `spherical-surface`, `convex-lens`, `concave-lens`, `lens-maker`,
`lenses-in-contact`, `prism`, `telescope`, `compound-microscope`. Distances follow the Cartesian sign convention
of the textbook.

Every measured answer is tested against its formula: `1e-6` or better, except the two that compare something real
with something ideal. `lens-maker` traces a lens 8 mm thick against the thin-lens formula (within 2 %), and
`lenses-in-contact` has its lenses a micrometre apart (within `1e-4`).

## Known limits

- **Two dimensions.** Every surface is the section of one with symmetry about the plane; skew rays do not exist.
- **Monochromatic.** No dispersion, so no spectrum from the prism and no chromatic aberration.
- **Outlines must close** for the even-odd rule to mean anything; this is not checked.
- **A ray along an outline's edge, or through a corner,** is decided by round-off.
- **No bench of one's own yet.** Scenes are data, so a front end could edit them; none does.
