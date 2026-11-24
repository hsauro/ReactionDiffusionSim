//! Tests of the numbers students compare against theory.
//! Long pattern runs are `#[ignore]`d; run them with
//! `cargo test --release -- --include-ignored`.

use super::grid::{apply_start, min_max, run_steps, std_dev};
use super::model::{Boundary, Model, ParamSpec, Preset, StartCtx};
use super::stability::{analyze, Verdict};
use super::*;
use crate::with_model;

fn scalar_params(p: &[f32]) -> Vec<f64> {
    p.iter().map(|&x| x as f64).collect()
}

#[test]
fn steady_states_are_steady() {
    for kind in ModelKind::ALL {
        with_model!(kind, m => {
            for ps in m.presets() {
                // Check at the background value and at the brush value of the map.
                for painted in [ps.params[m.painted_param()], ps.map_brush] {
                    let mut p = scalar_params(ps.params);
                    p[m.painted_param()] = painted as f64;
                    let (u, v) = m.steady_state(&p);
                    let (f, g) = m.reaction64(u, v, &p, painted as f64);
                    assert!(
                        f.abs() < 1e-5 && g.abs() < 1e-5,
                        "{} {}: f = {f}, g = {g}",
                        m.name(),
                        ps.name
                    );
                }
            }
        });
    }
}

fn uniform_stays_uniform<M: Model>(m: &M, boundary: Boundary) {
    for ps in m.presets() {
        let params = ps.params;
        let bg = params[m.painted_param()];
        let (u0, v0) = model::steady_state_at(m, params, bg);
        let mut g = Grid::new(40, boundary);
        g.map.fill(bg);
        g.u.fill(u0);
        g.v.fill(v0);
        let du = ps.du;
        let dv = effective_dv(m, params, du, ps.dv);
        let dt = time_step(m, du, dv);
        for _ in 0..1000 {
            grid::step(m, &mut g, params, du, dv, dt);
        }
        for k in 0..g.u.len() {
            assert!(
                (g.u[k] - u0).abs() <= 1e-6 * u0.abs().max(1.0)
                    && (g.v[k] - v0).abs() <= 1e-6 * v0.abs().max(1.0),
                "{} {} {boundary:?}: cell {k} drifted to ({}, {}) from ({u0}, {v0})",
                m.name(),
                ps.name,
                g.u[k],
                g.v[k]
            );
        }
    }
}

#[test]
fn uniform_stays_uniform_all() {
    for kind in ModelKind::ALL {
        for b in Boundary::ALL {
            with_model!(kind, m => uniform_stays_uniform(m, b));
        }
    }
}

const A_BR: f32 = 3.0;

fn brusselator_verdict(b: f32) -> stability::Dispersion {
    analyze(&Brusselator, &[A_BR, b], b, 4.0, 32.0)
}

#[test]
fn brusselator_panel() {
    // Turing threshold by bisection on b.
    let (mut lo, mut hi) = (3.0f32, 6.0f32);
    assert!(matches!(brusselator_verdict(lo).verdict, Verdict::Stable));
    assert!(matches!(brusselator_verdict(hi).verdict, Verdict::Turing { .. }));
    for _ in 0..40 {
        let mid = 0.5 * (lo + hi);
        if matches!(brusselator_verdict(mid).verdict, Verdict::Turing { .. }) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let threshold = 0.5 * (lo + hi) as f64;
    let expected = (1.0 + 3.0 * (4.0f64 / 32.0).sqrt()).powi(2); // 4.2463
    assert!(
        (threshold - expected).abs() < 1e-3,
        "Turing threshold {threshold}, expected {expected}"
    );

    assert_eq!(brusselator_verdict(10.1).verdict, Verdict::Oscillates);

    // Just above onset the fastest-growing wavelength is the critical one:
    // continuum 2 pi / sqrt(a / sqrt(Du Dv)) = 12.2, discrete grid 12.1.
    // At b = 6 the fastest-growing mode is shorter, about 10.7 cells.
    for (b, expected) in [(4.25, 12.1), (6.0, 10.7)] {
        match brusselator_verdict(b).verdict {
            Verdict::Turing { wavelength } => assert!(
                (wavelength - expected).abs() <= 0.2,
                "wavelength at b = {b} is {wavelength}, expected {expected}"
            ),
            v => panic!("expected Turing at b = {b}, got {v:?}"),
        }
    }
}

#[test]
fn lengyel_epstein_panel() {
    let check = |b: f32| {
        let p = [12.0, b, 50.0, 1.0];
        let dv = effective_dv(&LengyelEpstein, &p, 1.5, 0.0);
        analyze(&LengyelEpstein, &p, b, 1.5, dv).verdict
    };
    assert_eq!(check(0.10), Verdict::Oscillates);
    assert!(matches!(check(0.20), Verdict::Turing { .. }), "{:?}", check(0.20));
    assert_eq!(check(0.50), Verdict::Stable);
}

/// A model with no reaction, for testing diffusion alone.
struct NoReaction;

static NO_PARAMS: [ParamSpec; 1] = [ParamSpec {
    name: "p",
    key: "p",
    default: 1.0,
    min: 0.1,
    max: 10.0,
}];

fn no_start(_: &mut StartCtx<'_>) {}

static NO_PRESETS: [Preset; 1] = [Preset {
    name: "none",
    key: "none",
    params: &[1.0],
    du: 1.0,
    dv: 0.5,
    boundary: Boundary::Periodic,
    range_u: (0.0, 1.0),
    range_v: (0.0, 1.0),
    map_brush: 1.0,
    start: no_start,
}];

impl Model for NoReaction {
    fn name(&self) -> &'static str {
        "none"
    }
    fn params(&self) -> &'static [ParamSpec] {
        &NO_PARAMS
    }
    fn painted_param(&self) -> usize {
        0
    }
    fn diffusion(&self) -> (f32, f32) {
        (1.0, 0.5)
    }
    fn reaction(&self, _u: f32, _v: f32, _p: &[f32], _painted: f32) -> (f32, f32) {
        (0.0, 0.0)
    }
    fn reaction64(&self, _u: f64, _v: f64, _p: &[f64], _painted: f64) -> (f64, f64) {
        (0.0, 0.0)
    }
    fn steady_state(&self, _p: &[f64]) -> (f64, f64) {
        (0.0, 0.0)
    }
    fn dt_limit(&self) -> f32 {
        0.2
    }
    fn presets(&self) -> &'static [Preset] {
        &NO_PRESETS
    }
    fn default_speed(&self) -> f32 {
        1.0
    }
    fn brush_defaults(&self) -> (f32, f32) {
        (1.0, 1.0)
    }
}

#[test]
fn pure_diffusion_conserves_total() {
    for b in Boundary::ALL {
        let n = 50;
        let mut g = Grid::new(n, b);
        let mut rng = Rng::new(7);
        for k in 0..n * n {
            g.u[k] = rng.unit();
            g.v[k] = rng.unit();
        }
        // A sharp feature at an edge and a corner, to exercise the boundaries.
        g.u[0] = 5.0;
        g.v[n - 1] = 5.0;
        let total = |f: &[f32]| f.iter().map(|&x| x as f64).sum::<f64>();
        let (tu, tv) = (total(&g.u), total(&g.v));
        let (du, dv) = (1.0, 0.5);
        let dt = time_step(&NoReaction, du, dv);
        for _ in 0..10_000 {
            grid::step(&NoReaction, &mut g, &[1.0], du, dv, dt);
        }
        let (eu, ev) = ((total(&g.u) - tu) / tu, (total(&g.v) - tv) / tv);
        assert!(eu.abs() < 1e-5 && ev.abs() < 1e-5, "{b:?}: relative change {eu}, {ev}");
    }
}

/// Start a preset at grid size n and run it for `t_end` simulated time (or `steps`).
fn run_preset<M: Model>(m: &M, preset: usize, n: usize, steps: Option<usize>, t_end: f64) -> Grid {
    let ps = &m.presets()[preset];
    let mut g = Grid::new(n, ps.boundary);
    let mut rng = Rng::new(12345);
    apply_start(m, ps, ps.params, &mut g, &mut rng);
    let du = ps.du;
    let dv = effective_dv(m, ps.params, du, ps.dv);
    let dt = time_step(m, du, dv);
    let mut pending = match steps {
        Some(s) => s as f64 * dt as f64 + 0.5 * dt as f64,
        None => t_end + 0.5 * dt as f64,
    };
    run_steps(m, &mut g, ps.params, du, dv, dt, &mut pending, || true);
    assert!(g.is_finite(), "{} {} became non-finite", m.name(), ps.name);
    g
}

#[test]
#[ignore = "long pattern run"]
fn pattern_brusselator_spots() {
    let g = run_preset(&Brusselator, 0, 100, None, 100.0);
    let sd = std_dev(&g.u);
    assert!(sd > 1.0, "std dev of u at t = 100 is {sd}");
}

#[test]
#[ignore = "long pattern run"]
fn pattern_lengyel_epstein_stable() {
    let g = run_preset(&LengyelEpstein, 2, 100, None, 60.0);
    let sd = std_dev(&g.u);
    assert!(sd < 1e-3, "std dev of u at t = 60 is {sd}");
}

#[test]
#[ignore = "long pattern run"]
fn pattern_gray_scott_dividing_spots() {
    let count = |g: &Grid| g.v.iter().filter(|&&x| x > 0.1).count();
    let early = count(&run_preset(&GrayScott, 0, 100, Some(2000), 0.0));
    let g = run_preset(&GrayScott, 0, 100, Some(10_000), 0.0);
    let (_, vmax) = min_max(&g.v);
    assert!(vmax > 0.2, "max v = {vmax}");
    assert!(count(&g) > early, "cells with v > 0.1: {} at 10k vs {early} at 2k", count(&g));
}

#[test]
#[ignore = "long pattern run"]
fn pattern_barkley_spiral() {
    let g = run_preset(&Barkley, 0, 100, None, 60.0);
    let (_, umax) = min_max(&g.u);
    assert!(umax > 0.9, "max u at t = 60 is {umax}");
}

#[test]
#[ignore = "long pattern run"]
fn pattern_barkley_target_launches_wave() {
    // Not in the spec's list: checks that the Target disc is big enough to start a wave.
    let g = run_preset(&Barkley, 1, 100, None, 3.0);
    let excited = g.u.iter().filter(|&&x| x > 0.5).count();
    let (_, umax) = min_max(&g.u);
    assert!(umax > 0.9 && excited > 200, "max u {umax}, excited cells {excited}");
}

/// Prints field statistics for every preset (for choosing display ranges).
/// Run with `cargo test --release preset_report -- --ignored --nocapture`.
#[test]
#[ignore = "report, not a check"]
fn preset_report() {
    fn report<M: Model>(m: &M) {
        for (i, ps) in m.presets().iter().enumerate() {
            let du = ps.du;
            let dv = effective_dv(m, ps.params, du, ps.dv);
            let dt = time_step(m, du, dv) as f64;
            let t = if dt >= 1.0 { 10_000.0 } else { 60.0 };
            let g = run_preset(m, i, 100, None, t);
            let (ul, uh) = min_max(&g.u);
            let (vl, vh) = min_max(&g.v);
            let d = analyze(m, ps.params, ps.params[m.painted_param()], du, dv);
            #[expect(clippy::print_stdout)]
            {
                println!(
                    "{:16} {:15} t={t:6} u [{ul:.3}, {uh:.3}] v [{vl:.3}, {vh:.3}] sd(u)={:.3}  {}",
                    m.name(),
                    ps.name,
                    std_dev(&g.u),
                    d.verdict.text()
                );
            }
        }
    }
    report(&Brusselator);
    report(&GrayScott);
    report(&LengyelEpstein);
    report(&Barkley);
}

/// Cell updates per second for each model at N = 200.
/// Run with `cargo test --release throughput -- --ignored --nocapture`.
#[test]
#[ignore = "report, not a check"]
fn throughput() {
    fn bench<M: Model>(m: &M) {
        let ps = &m.presets()[0];
        let n = 200;
        let mut g = Grid::new(n, ps.boundary);
        let mut rng = Rng::new(1);
        apply_start(m, ps, ps.params, &mut g, &mut rng);
        let du = ps.du;
        let dv = effective_dv(m, ps.params, du, ps.dv);
        let dt = time_step(m, du, dv);
        let steps = 2000;
        let start = web_time::Instant::now();
        for _ in 0..steps {
            grid::step(m, &mut g, ps.params, du, dv, dt);
        }
        let secs = start.elapsed().as_secs_f64();
        #[expect(clippy::print_stdout)]
        {
            println!("{:16} {:.2e} cell updates/s", m.name(), (steps * n * n) as f64 / secs);
        }
    }
    bench(&Brusselator);
    bench(&GrayScott);
    bench(&LengyelEpstein);
    bench(&Barkley);
}

/// The Brusselator stays finite across its slider ranges, including strong
/// relaxation oscillations (b >> 1 + a^2) that broke plain forward Euler.
#[test]
#[ignore = "long pattern run"]
fn brusselator_sweep_stays_finite() {
    let m = &Brusselator;
    let n = 64;
    for &(du, dv) in &[(4.0f32, 32.0f32), (4.0, 4.0), (0.01, 1000.0), (100.0, 100.0)] {
        for &a in &[0.75f32, 1.5, 3.0, 6.0, 12.0] {
            let mut bad = Vec::new();
            for &b in &[1.0f32, 3.0, 6.0, 10.0, 12.0, 20.0, 30.0] {
                let params = [a, b];
                let mut g = Grid::new(n, Boundary::NoFlux);
                let mut rng = Rng::new(3);
                apply_start(m, &m.presets()[0], &params, &mut g, &mut rng);
                let dt = time_step(m, du, dv);
                let mut pending = 60.0;
                run_steps(m, &mut g, &params, du, dv, dt, &mut pending, || true);
                if !g.is_finite() {
                    bad.push(b);
                }
            }
            assert!(bad.is_empty(), "Du={du} Dv={dv} a={a}: blows up at b = {bad:?}");
        }
    }
}

