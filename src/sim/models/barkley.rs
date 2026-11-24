//! Barkley excitable medium:
//! u' = (1/eps) u (1-u) (u - (v+b)/a) + D lap u,   v' = u - v.  Only u diffuses.

use crate::sim::model::{Boundary, Model, ParamSpec, Preset, Real, StartCtx};

pub struct Barkley;

const A: usize = 0;
const B: usize = 1;
const EPS: usize = 2;

static PARAMS: [ParamSpec; 3] = [
    ParamSpec {
        name: "a",
        key: "a",
        default: 0.75,
        min: 0.19,
        max: 3.0,
    },
    ParamSpec {
        name: "b",
        key: "b",
        default: 0.02,
        min: 0.005,
        max: 0.08,
    },
    ParamSpec {
        name: "\u{3b5}",
        key: "eps",
        default: 0.02,
        min: 0.005,
        max: 0.08,
    },
];

/// u = 1 in a strip 12 cells wide down the top half of the centre column,
/// v = a/2 in the 12 cells to its left.
fn start_spiral(c: &mut StartCtx<'_>) {
    let n = c.n;
    let mid = n / 2;
    let w = 12.min(mid);
    for i in 0..mid {
        for j in mid..(mid + w).min(n) {
            c.u[i * n + j] = 1.0;
        }
        for j in (mid - w)..mid {
            c.v[i * n + j] = 0.5 * c.map[i * n + j];
        }
    }
}

/// u = 1 in a disc at the centre.
fn start_target(c: &mut StartCtx<'_>) {
    let n = c.n;
    let r = TARGET_RADIUS;
    let cx = n as f32 / 2.0;
    for i in 0..n {
        for j in 0..n {
            let dx = j as f32 + 0.5 - cx;
            let dy = i as f32 + 0.5 - cx;
            if dx * dx + dy * dy <= r * r {
                c.u[i * n + j] = 1.0;
            }
        }
    }
}

/// Radius of the Target preset's disc, in cells.
pub const TARGET_RADIUS: f32 = 5.0;

const fn preset(name: &'static str, key: &'static str, start: fn(&mut StartCtx<'_>)) -> Preset {
    Preset {
        name,
        key,
        params: &[0.75, 0.02, 0.02],
        du: 16.0,
        dv: 0.0,
        boundary: Boundary::Periodic,
        range_u: (0.0, 1.0),
        range_v: (0.0, 0.7),
        map_brush: 0.4,
        start,
    }
}

static PRESETS: [Preset; 2] = [
    preset("Spiral", "spiral", start_spiral),
    preset("Target", "target", start_target),
];

#[inline(always)]
fn rhs<T: Real>(u: T, v: T, a: T, b: T, eps: T) -> (T, T) {
    let one = T::c(1.0);
    (u * (one - u) * (u - (v + b) / a) / eps, u - v)
}

impl Model for Barkley {
    fn name(&self) -> &'static str {
        "Barkley"
    }
    fn params(&self) -> &'static [ParamSpec] {
        &PARAMS
    }
    fn painted_param(&self) -> usize {
        A
    }
    fn diffusion(&self) -> (f32, f32) {
        (16.0, 0.0)
    }
    fn derived_dv(&self, _p: &[f32], _du: f32) -> Option<f32> {
        Some(0.0)
    }
    #[inline(always)]
    fn reaction(&self, u: f32, v: f32, p: &[f32], painted: f32) -> (f32, f32) {
        rhs(u, v, painted, p[B], p[EPS])
    }
    fn reaction64(&self, u: f64, v: f64, p: &[f64], painted: f64) -> (f64, f64) {
        rhs(u, v, painted, p[B], p[EPS])
    }
    fn steady_state(&self, _p: &[f64]) -> (f64, f64) {
        (0.0, 0.0)
    }
    fn dt_limit(&self) -> f32 {
        0.012
    }
    fn presets(&self) -> &'static [Preset] {
        &PRESETS
    }
    fn note(&self) -> Option<&'static str> {
        Some("The rest state is stable but excitable. A large enough disturbance launches a wave.")
    }
    fn default_speed(&self) -> f32 {
        10.0
    }
    fn brush_defaults(&self) -> (f32, f32) {
        (1.0, 0.4)
    }
}
