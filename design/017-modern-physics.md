# 017: Modern physics

Status: implemented. NCERT class 12, the chapters on dual nature of radiation and matter, atoms, and nuclei. A
new crate, `soliton-modern`.

## Scope

Eight lessons. Three of them follow something, and the rest evaluate a formula.

- **Followed.** Alpha particles past a nucleus (`rutherford`), nuclei decaying by chance (`nucleus::Sample`),
  photoelectrons crossing a tube (`photoelectric::Tube`).
- **Evaluated.** De Broglie's wavelength and the Davisson–Germer angle (`matter`), Bohr's levels and the lines
  of hydrogen (`atom`), the liquid-drop binding energy (`nucleus::Nuclide`).

## One shape for every lesson

The lessons have little in common physically, so they share an interface and not a state. `Lesson` is a trait:
`advance(seconds)`, `finished`, `still`, `clock`, and four things to show.

- `picture()`: an enum, one variant a lesson, holding what is on the stage in the lesson's own units.
- `readings()`: labelled numbers with units, or words.
- `plots()`: curves with labels and a marker.
- `answers()`: textbook values, with what the run measured where it measures anything.

The playground draws these and decides nothing about the physics. A lesson with nothing to play says `still`,
and the playground hides the play controls.

## Units

Whatever keeps the numbers near one. Photons and atoms: eV and nm, with `hc = 1239.84 eV nm`. Nuclei: MeV and
fm, with `e²/4πε₀ = 1.44 MeV fm`, and times in fm/c.

## Rutherford scattering

Newton's second law for an alpha in the Coulomb field of a fixed point nucleus, by velocity Verlet with a step
proportional to the distance from the nucleus, so that it is short where the path bends. The alpha starts
40 000 fm away with the speed energy conservation gives it there, and is followed out to the same distance; the
angle is read from its final velocity. Rutherford's `θ = 2 arctan(d/2b)` is used only to check.

The nucleus does not recoil and has no size. The first is a 2 % effect for gold; the second matters only above
the energies offered, where the alpha would reach the nuclear surface.

## Radioactive decay

In each short step every surviving nucleus decays with probability `1 − e^{−λ dt}`. That is the whole rule; the
exponential law is what the count is compared with. The tolerance on a count expected to be `Np` is three
times `√(Np(1 − p))`.

## The phototube

Photons arrive at random, at a rate proportional to the brightness, and each frees one electron if
`hf > φ`. Electrons are given energies spread evenly from zero to `hf − φ` and move straight across under the
collector's voltage. The spread and the straight paths are a sketch; with them the current against a retarding
voltage falls in a straight line to zero at the stopping potential. Einstein's equation, the threshold and the
stopping potential do not depend on the sketch.

## Davisson–Germer

The angle is `sin θ = λ/d`. The strength drawn round it is a grating of six rows times a slowly falling
background. It is there to give the hump a shape, and the docs say it is a model.

## Binding energy

The liquid-drop formula with `a_V = 15.75`, `a_S = 17.8`, `a_C = 0.711`, `a_A = 23.7`, `δ = 11.18/√A` MeV, at
the proton number that maximises it. Thirty measured values are tabulated beside it. The formula is within
0.12 MeV per nucleon of them from calcium upward and is not drawn below mass number 16.

## Next

X-rays and Moseley's law; alpha, beta and gamma decay and decay chains; a chain reaction; semiconductors, which
need a diode in `soliton-circuits`.
