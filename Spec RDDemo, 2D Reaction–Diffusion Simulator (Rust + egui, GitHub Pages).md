# Spec: RDDemo, 2D Reaction–Diffusion Simulator (Rust + egui, GitHub Pages)

Nov 24, 2026 · @Herbert Sauro

## Purpose

RDDemo is a browser app for two-species reaction–diffusion models on a 2D grid. Students pick a model, paint concentrations or parameter values with the mouse, and watch patterns form. A panel shows whether the uniform state should be stable, form a Turing pattern, or oscillate.

It is written in Rust with egui, compiled to WebAssembly, and served from GitHub Pages. It is the second tool in the series after the 2D diffusion painter, and is used in an undergraduate computational systems biology class.

Students should be able to:

- See stationary Turing patterns (spots, stripes, mazes) grow from noise or from a painted disturbance.
- Paint a prepattern into a parameter, such as a W in which b is above the Turing threshold, and see the pattern confined to it.
- See travelling waves and spirals in an excitable model, linking to the BZ reaction.
- Compare the stability panel’s prediction with what appears on the canvas.
- Share an exact setup as a link.

## Scope

v1 has four built-in two-species models with presets, painting of species and parameters, a probe, a stability panel and URL sharing.

**In scope**

- Models: Brusselator, Gray–Scott, Lengyel–Epstein (the CIMA reaction model), Barkley (excitable spirals).
- Two species per model, u and v, each with its own diffusion constant.
- Per-cell maps for the model’s main control parameter, which can be painted.
- Paint u, v or the parameter map; seed noise; clear.
- Probe: a time course of u and v at a clicked cell.
- Stability panel computed from the model’s equations at the current parameter values.
- Full state of the controls encoded in the URL.
- Chrome, Firefox and Safari on a laptop; native `cargo run` for development.

**Not in v1**

- Models typed in by the user, or loaded from Antimony or SBML. Models are Rust code behind a small trait.
- Three or more species.
- Growing or irregular domains.
- Saving images, video or simulation state to files.
- GPU compute.

## Models and presets

Four models, each with two species u and v, one paintable parameter, and presets whose behaviour was checked by simulation on 2026-11-24. Lengths are in grid cells (Δx = 1). All runs used the five-point Laplacian and the time steps given here, with no-flux boundaries for the Brusselator and periodic boundaries for the others.

### Brusselator

```latex
\begin{aligned}
\dot u &= a - (b+1)\,u + u^2 v + D_u \nabla^2 u \\
\dot v &= b\,u - u^2 v + D_v \nabla^2 v
\end{aligned}
```

Steady state u = a, v = b/a. Paintable parameter: b. Time step Δt ≤ 0.005.

| Preset | a | b | D\_u | D\_v | Start | Result |
| --- | --- | --- | --- | --- | --- | --- |
| Spots | 3 | 6 | 4 | 32 | steady state + noise 0.01 | hexagonal spots, spacing about 12 cells |
| Stripes | 3 | 4.6 | 4 | 32 | steady state + noise 0.01 | stripes and a maze |
| Oscillating | 3 | 12 | 4 | 4 | steady state + noise 0.01 | whole field oscillates, with travelling waves |
| W prepattern | 3 | map: 7 inside a W, 3 outside | 4 | 32 | steady state + noise 0.01 | spots only inside the W |

### Gray–Scott

```latex
\begin{aligned}
\dot u &= -u v^2 + F\,(1-u) + D_u \nabla^2 u \\
\dot v &= u v^2 - (F+k)\,v + D_v \nabla^2 v
\end{aligned}
```

Uniform state u = 1, v = 0. Paintable parameter: F. D\_u = 0.16, D\_v = 0.08, Δt = 1. Start: u = 1, v = 0 everywhere, with a 20 × 20 square at the centre set to u = 0.5, v = 0.25, plus uniform noise of 0.02 on both.

| Preset | F | k | Result after 10,000 steps (200 × 200) |
| --- | --- | --- | --- |
| Dividing spots | 0.0367 | 0.0649 | spots that split and spread outward from the seed |
| Coral | 0.060 | 0.062 | branching fingers growing outward |
| Maze | 0.040 | 0.060 | stripes that fill the whole domain |
| Unsteady spots | 0.014 | 0.054 | spots that appear and fade across the domain |

### Lengyel–Epstein (CIMA reaction)

```latex
\begin{aligned}
\dot u &= a - u - \frac{4uv}{1+u^2} + D_u \nabla^2 u \\
\dot v &= \sigma\left[b\left(u - \frac{uv}{1+u^2}\right) + d\,D_u \nabla^2 v\right]
\end{aligned}
```

Steady state u = a/5, v = 1 + a²/25. Paintable parameter: b. Fixed a = 12, σ = 50, d = 1, D\_u = 1.5, so v diffuses 50 times faster than u. Δt ≤ 0.0025. Start: steady state plus noise 0.01.

| Preset | b | Predicted | Result at t = 60 (150 × 150) |
| --- | --- | --- | --- |
| Maze | 0.20 | Turing | labyrinth |
| Spots | 0.30 | Turing | spots mixed with short stripes |
| Stable | 0.50 | stable | stays uniform |

For a = 12, σ = 50, d = 1, a linear-stability scan in steps of 0.05 found oscillation at b ≤ 0.10, Turing patterns at 0.15–0.35, and a stable uniform state at b ≥ 0.40.

### Barkley (excitable medium)

```latex
\begin{aligned}
\dot u &= \frac{1}{\varepsilon}\,u\,(1-u)\left(u - \frac{v+b}{a}\right) + D \nabla^2 u \\
\dot v &= u - v
\end{aligned}
```

Only u diffuses. Paintable parameter: a. a = 0.75, b = 0.02, ε = 0.02, D = 16, Δt ≤ 0.012.

| Preset | Start | Result |
| --- | --- | --- |
| Spiral | u = 1 in a strip 12 cells wide down the top half of the centre column; v = a/2 in the 12 cells to its left | a pair of rotating spirals by t = 20 (periodic boundaries) |
| Target | u = 1 in a disc of radius 5 at the centre | one expanding ring |

The Target preset was not run; it is the standard way to start a single wave. Painting u = 1 across a travelling wave breaks it and starts new spirals, which is the classroom demonstration.

## Numerics

Use forward Euler in time and a five-point Laplacian in space. Take as many substeps per frame as a fixed compute budget allows.

**Grid.** N × N, with N chosen from 100, 200 (default) or 300; changing N reapplies the current preset. Store u, v and each parameter map as flat `Vec<f32>`, with a second buffer for u and v swapped each step. Parameters that are not painted are scalars.

**Step.** For each cell: u\_new = u + Δt (f(u, v, p) + D\_u ∇²u), and the same for v with g and D\_v, where p is that cell’s parameter value. Compute the Laplacians of u and v in the same pass as the reaction terms.

**Time step.** Δt = min(model limit from the presets section, 0.2 / max(D\_u, D\_v)). This is fixed per model and parameter set; it does not change with the speed control.

**Substeps per frame.** Run steps until either the simulated time asked for by the speed control is reached or 12 ms of wall-clock time has been used, whichever comes first. Show the achieved simulated time per second in the status bar. Never shorten Δt to catch up.

**Boundaries.** Periodic (default) or no flux, chosen from a dropdown.

**Guards.** After each frame, if any value is NaN or infinite, pause and show “Simulation became unstable – reset or lower the speed”. Do not clamp values, except u, v ≥ 0 after painting.

**Performance target.** At least 2 × 10⁸ cell updates per second in Chrome on a typical student laptop, single-threaded. That is enough for the Brusselator spots preset (Δt = 0.005) at N = 200 to form its pattern within about 30 s. Use plain loops over slices, with the edges handled outside the inner loop, and no allocation per step.

## Interface

The layout matches the diffusion painter: controls on the left, the square canvas in the middle, and a right panel holding the probe plot and the stability panel. A status bar runs along the bottom.

**Canvas**

- A View dropdown shows u, v or the parameter map. Use viridis for species and greyscale for the map, with a colour bar.
- Each preset sets the display range for each species. An “Auto scale” checkbox uses the current minimum and maximum instead.
- Use nearest-neighbour texture filtering, and update the texture once per frame.

**Brush**

- A Target dropdown chooses u, v, the parameter map, or Probe.
- Left drag blends the target toward the brush value: x ← x + w (value − x), with w = exp(−d² / 2σ²) within 2σ of the pointer and σ = radius / 2. Right drag (or Shift + drag) blends toward the preset’s background value instead.
- The brush value defaults per target: u = 1 for Barkley; v = 0.25 for Gray–Scott; the upper preset value for a parameter map (b = 7 in the W preset).
- Stamp along the stroke between frames at spacing σ. Painting works while running and while paused. Draw the brush outline under the pointer.
- With Probe selected, a click places the probe marker on that cell.

**Parameter map.** Its slider sets the uniform background value. While the map is unpainted, moving the slider changes every cell. Once painted, the slider changes only unpainted cells. “Clear map” returns every cell to the slider value.

**Controls**, top to bottom

| Control | Widget | Notes |
| --- | --- | --- |
| Model | dropdown | Brusselator, Gray–Scott, Lengyel–Epstein, Barkley; switching applies the model’s first preset |
| Preset | dropdown | sets every parameter, the start state, the boundary and the display range |
| Play / Pause, Step, Reset | buttons; Space, S, R | Reset reapplies the current preset |
| Seed noise | button | adds uniform noise ±0.01 to u and v |
| Model parameters | one slider each, with a number box | ranges about 0.25× to 4× the preset value, logarithmic |
| D\_u, D\_v | log sliders |  |
| Speed | log slider | target simulated time per second |
| Brush target, radius (2–30 cells), value | dropdown, sliders |  |
| Boundary, Grid N, View, Auto scale | dropdowns, checkbox |  |
| Copy link | button | copies the shareable URL |

**Probe plot.** Plot u and v at the probe cell against time over the last 2,000 frames, using `egui_plot`. Clear it on Reset or when the probe moves.

**Status bar:** simulated time t, achieved simulated time per second, frames per second, the minimum and maximum of the field shown, and the cell and value under the pointer.

**Shareable URL.** Encode the model, preset, every scalar parameter, D\_u, D\_v, speed, N, boundary and view in the URL fragment, for example `#m=brusselator&p=spots&b=6&Du=4&Dv=32`. Read it on load. Unknown or invalid keys fall back to the preset’s values without an error. Painted maps and painted species are not encoded; presets that include a map (the W) rebuild it from the preset.

**W preset shape.** Fill the polygon with these vertices, in fractions of the grid width, measured from the top-left corner: (0.06, 0.10), (0.25, 0.10), (0.37, 0.62), (0.45, 0.30), (0.55, 0.30), (0.63, 0.62), (0.75, 0.10), (0.94, 0.10), (0.73, 0.92), (0.57, 0.92), (0.50, 0.62), (0.43, 0.92), (0.27, 0.92).

## Stability panel

The panel says what linear theory predicts for the uniform steady state at the current parameters, so students can compare it with the canvas.

**What it evaluates**

1. Steady state (u\*, v\*) from the model’s closed form: Brusselator (a, b/a); Gray–Scott (1, 0); Lengyel–Epstein (a/5, 1 + a²/25); Barkley (0, 0).
2. Jacobian J of the reaction terms at that point, by central differences with step 10⁻⁶ relative to the value. This way new models need no hand-written Jacobian.
3. For wavenumbers k from 0 to π (400 points), the largest real part of the eigenvalues of J − q(k) diag(D\_u, D\_v), with q(k) = 4 sin²(k/2). This is the discrete Laplacian along one axis, so the prediction matches the grid exactly. Use the 2 × 2 closed form from trace and determinant.

The parameter value used is the map’s background value. While the pointer is over the canvas, use that cell’s value instead, and label the panel “at cell (i, j)”.

**What it shows**

- A plot of growth rate against k, with the zero line and the fastest-growing k\* marked.
- One verdict line:

| Condition | Verdict |
| --- | --- |
| growth at k = 0 positive, with complex eigenvalues | Oscillates (well mixed) |
| growth at k = 0 positive, with real eigenvalues | Unstable even when well mixed |
| growth at k = 0 negative, and positive at some k > 0 | Turing pattern, wavelength 2π/k\* = … cells |
| negative everywhere | Stable uniform state |

- The steady state and the two eigenvalues at k = 0, to three significant figures.

**Model notes.** These are fixed text shown under the verdict:

- Gray–Scott: “The uniform state (1, 0) is always stable. Patterns here need a large enough seed; they are not Turing patterns from the uniform state.”
- Barkley: “The rest state is stable but excitable. A large enough disturbance launches a wave.”

## Architecture

RDDemo is one self-contained Cargo crate in a folder named `RDDemo`, started from the official `eframe_template`. The simulation code has no egui dependency, so it can be tested on its own.

```
RDDemo/
  Cargo.toml
  index.html                  # trunk entry point
  src/main.rs                 # native and web entry
  src/app.rs                  # UI, input, painting, texture
  src/url_state.rs            # fragment encode and decode
  src/sim/mod.rs
  src/sim/grid.rs             # fields, Laplacian, boundaries, brush
  src/sim/model.rs            # Model trait, ParamSpec, Preset
  src/sim/stability.rs        # steady state, Jacobian, dispersion
  src/sim/models/brusselator.rs
  src/sim/models/gray_scott.rs
  src/sim/models/lengyel_epstein.rs
  src/sim/models/barkley.rs
  .github/workflows/pages.yml
```

**Model trait**

```rust
pub trait Model {
    fn name(&self) -> &'static str;
    fn params(&self) -> &'static [ParamSpec];   // name, default, min, max
    fn painted_param(&self) -> usize;           // index into params
    fn diffusion(&self) -> (f32, f32);          // default D_u, D_v
    fn reaction(&self, u: f32, v: f32, p: &[f32], painted: f32) -> (f32, f32); // painted = this cell's map value
    fn steady_state(&self, p: &[f64]) -> (f64, f64);
    fn dt_limit(&self) -> f32;
    fn presets(&self) -> &'static [Preset];
    fn note(&self) -> Option<&'static str>;
}
```

The time step must be generic over the model, `fn step<M: Model>(m: &M, …)`, so `reaction` is inlined. Select the model with a `match` on an enum once per step, never through a `dyn Model` call per cell. A `Preset` holds its name, parameter values, D\_u, D\_v, boundary, display ranges and a start-state function.

Adding a model means one new file and one enum variant. The grid and brush code can be copied from the diffusion painter; do not set up a shared crate yet.

## Build and deployment

Build with trunk and deploy to GitHub Pages from a GitHub Actions workflow, assuming the repository is also named RDDemo.

- Native: `cargo run --release`.
- Web, local: `trunk serve`, then open the printed localhost address.
- Deployed: `https://<owner>.github.io/RDDemo/`.

**Workflow** (`.github/workflows/pages.yml`, on push to `main`)

1. Install stable Rust with the `wasm32-unknown-unknown` target, and install trunk.
2. Run `cargo test` (the simulation tests below).
3. Run `trunk build --release --public-url /RDDemo/`.
4. Upload `dist` with `actions/upload-pages-artifact` and publish it with `actions/deploy-pages`.

Set the repository’s Pages source to “GitHub Actions”. Cache the cargo registry and target folder.

If RDDemo later moves under a shared repository, only `--public-url` changes.

## Testing and acceptance

Unit tests check the numbers that students will compare against theory. A manual pass checks that every preset looks as described.

**Unit tests** (`cargo test`; mark the long pattern runs `#[ignore]` and run them in CI with `--include-ignored --release`)

1. Steady states: for every preset, both reaction terms at `steady_state` are below 10⁻⁵ in magnitude.
2. Uniform stays uniform: starting exactly at the steady state with no noise, 1,000 steps leave every cell within 10⁻⁶ of it, for every model and both boundaries.
3. Brusselator panel (a = 3, D\_u = 4, D\_v = 32): bisection on b finds the Turing threshold at (1 + a√(D\_u/D\_v))² = 4.243 ± 0.001. The verdict is “Oscillates” at b = 10.1. At b = 6 the wavelength is 12.1 ± 0.2 cells (the discrete-grid value; the continuum value is 12.2).
4. Lengyel–Epstein panel (a = 12, σ = 50, d = 1, D\_u = 1.5): the verdicts are Oscillates at b = 0.10, Turing at b = 0.20, and Stable at b = 0.50.
5. Pure diffusion (reaction switched off): the total is conserved to 10⁻⁵ relative over 10⁴ steps, for no-flux and periodic boundaries.
6. URL round trip: encode, then decode, gives back the same state for every preset.
7. Pattern runs at N = 100 (ignored by default):
   - Brusselator spots: the spatial standard deviation of u exceeds 1.0 by t = 100 (a reference run reached 1.95 by t = 50).
   - Lengyel–Epstein b = 0.50: the spatial standard deviation of u stays below 10⁻³ at t = 60.
   - Gray–Scott dividing spots: after 10,000 steps max v > 0.2, and more cells have v > 0.1 than at step 2,000.
   - Barkley spiral: max u > 0.9 at t = 60, so activity sustains itself.

**Manual acceptance**

- [ ] Every preset in the models section gives the result described there.
- [ ] W prepattern: spots appear inside the W only. Painting b = 7 elsewhere grows spots there too.
- [ ] The stability verdict matches the canvas for each Brusselator and Lengyel–Epstein preset.
- [ ] Probe on the Brusselator oscillating preset shows sustained oscillations; on the spots preset it settles.
- [ ] Barkley: dragging the brush across a travelling wave breaks it and starts new spirals.
- [ ] Copy link, then open it in a new tab: the same model, preset and parameters load.
- [ ] At N = 200, at least 30 frames per second with every model in Chrome on a typical student laptop. Works in Firefox and Safari.
- [ ] With each reaction parameter pushed to the end of its slider range, the simulation either keeps running or pauses with the instability message; it never freezes or shows garbage.
