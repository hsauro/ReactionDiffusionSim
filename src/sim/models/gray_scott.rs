//! Gray-Scott: u' = -u v^2 + F (1 - u),  v' = u v^2 - (F + k) v.

use crate::sim::model::{Boundary, Model, ParamSpec, Preset, Real, StartCtx};

pub struct GrayScott;

const F: usize = 0;
const K: usize = 1;

static PARAMS: [ParamSpec; 2] = [
    ParamSpec {
        name: "F",
        key: "F",
        default: 0.0367,
        min: 0.0035,
        max: 0.24,
    },
    ParamSpec {
        name: "k",
        key: "k",
        default: 0.0649,
        min: 0.0135,
        max: 0.25,
    },
];

/// u = 1, v = 0, with a 20 x 20 square at the centre set to u = 0.5, v = 0.25,
/// plus uniform noise of +/- 0.02 on both (v kept >= 0).
fn start_seed(c: &mut StartCtx<'_>) {
    let n = c.n;
    c.u.fill(1.0);
    c.v.fill(0.0);
    let half = 10.min(n / 2);
    let lo = n / 2 - half;
    let hi = n / 2 + half;
    for i in lo..hi {
        for j in lo..hi {
            c.u[i * n + j] = 0.5;
            c.v[i * n + j] = 0.25;
        }
    }
    for k in 0..n * n {
        c.u[k] = (c.u[k] + c.rng.sym(0.02)).max(0.0);
        c.v[k] = (c.v[k] + c.rng.sym(0.02)).max(0.0);
    }
}

const fn preset(name: &'static str, key: &'static str, params: &'static [f32], map_brush: f32) -> Preset {
    Preset {
        name,
        key,
        params,
        du: 0.16,
        dv: 0.08,
        boundary: Boundary::Periodic,
        range_u: (0.0, 1.0),
        range_v: (0.0, 0.5),
        map_brush,
        start: start_seed,
    }
}

static PRESETS: [Preset; 4] = [
    preset("Dividing spots", "spots", &[0.0367, 0.0649], 0.06),
    preset("Coral", "coral", &[0.060, 0.062], 0.0367),
    preset("Maze", "maze", &[0.040, 0.060], 0.06),
    preset("Unsteady spots", "unsteady", &[0.014, 0.054], 0.04),
];

#[inline(always)]
fn rhs<T: Real>(u: T, v: T, f: T, k: T) -> (T, T) {
    let uv2 = u * v * v;
    (-uv2 + f * (T::c(1.0) - u), uv2 - (f + k) * v)
}

impl Model for GrayScott {
    fn name(&self) -> &'static str {
        "Gray\u{2013}Scott"
    }
    fn params(&self) -> &'static [ParamSpec] {
        &PARAMS
    }
    fn painted_param(&self) -> usize {
        F
    }
    fn diffusion(&self) -> (f32, f32) {
        (0.16, 0.08)
    }
    #[inline(always)]
    fn reaction(&self, u: f32, v: f32, p: &[f32], painted: f32) -> (f32, f32) {
        rhs(u, v, painted, p[K])
    }
    fn reaction64(&self, u: f64, v: f64, p: &[f64], painted: f64) -> (f64, f64) {
        rhs(u, v, painted, p[K])
    }
    fn steady_state(&self, _p: &[f64]) -> (f64, f64) {
        (1.0, 0.0)
    }
    fn dt_limit(&self) -> f32 {
        1.0
    }
    fn presets(&self) -> &'static [Preset] {
        &PRESETS
    }
    fn note(&self) -> Option<&'static str> {
        Some(
            "The uniform state (1, 0) is always stable. Patterns here need a large enough seed; \
             they are not Turing patterns from the uniform state.",
        )
    }
    fn default_speed(&self) -> f32 {
        2000.0
    }
    fn brush_defaults(&self) -> (f32, f32) {
        (0.5, 0.25)
    }
}
