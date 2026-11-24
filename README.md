# RDDemo: 2D reaction-diffusion simulator

A browser app for two-species reaction-diffusion models on a 2D grid, written in
Rust with [egui](https://github.com/emilk/egui) and compiled to WebAssembly.
Students pick a model, paint concentrations or parameter values with the mouse,
and watch patterns form. A panel shows whether linear theory predicts a stable
uniform state, a Turing pattern, or oscillations.

Live: https://hsauro.github.io/ReactionDiffusionSim/

## Models

| Model | Presets | Painted parameter |
| --- | --- | --- |
| Brusselator | Spots, Stripes, Oscillating, W prepattern | b |
| Gray-Scott | Dividing spots, Coral, Maze, Unsteady spots | F |
| Lengyel-Epstein (CIMA) | Maze, Spots, Stable | b |
| Barkley (excitable) | Spiral, Target | a |

## Using it

- **Brush**: choose u, v, the parameter map, or Probe as the target. Left drag
  blends toward the brush value; right drag (or Shift + drag) blends toward the
  background value.
- **Keys**: Space play/pause, S step, R reset (restart the preset's start
  state with the current parameter values; choose the preset again to restore
  its parameter values).
- **Copy link** puts the full control state in the URL fragment, e.g.
  `#m=brusselator&p=spots&b=6&Du=4&Dv=32`.

## Building

```
cargo run --release                 # native
cargo run --release -- "m=barkley&p=spiral"   # native, starting from a link fragment
trunk serve                         # web, then open the printed localhost address
cargo test --release -- --include-ignored     # all tests, including pattern runs
```

On Windows with Git Bash, prefix trunk commands that take `--public-url` with
`MSYS_NO_PATHCONV=1`, or Git Bash rewrites the path.

Pushing to `main` runs the tests and deploys to GitHub Pages
(`.github/workflows/pages.yml`). In the repository settings, set
Pages > Source to "GitHub Actions".

## Layout

```
src/main.rs                    native and web entry
src/app.rs                     UI, input, painting, texture
src/url_state.rs               URL fragment encode and decode
src/sim/grid.rs                fields, Laplacian, boundaries, brush
src/sim/model.rs               Model trait, ParamSpec, Preset
src/sim/stability.rs           steady state, Jacobian, dispersion relation
src/sim/models/*.rs            one file per model
```

Adding a model means one new file in `src/sim/models/` and one `ModelKind` variant.
