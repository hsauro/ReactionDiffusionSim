//! Brusselator: u' = a - (b+1) u + u^2 v,  v' = b u - u^2 v.

use crate::sim::model::{steady_plus_noise, Boundary, Model, ParamSpec, Preset, Real, StartCtx};

pub struct Brusselator;

const A: usize = 0;
const B: usize = 1;

static PARAMS: [ParamSpec; 2] = [
    ParamSpec {
        name: "a",
        key: "a",
        default: 3.0,
        min: 0.75,
        max: 12.0,
    },
    ParamSpec {
        name: "b",
        key: "b",
        default: 6.0,
        min: 1.0,
        max: 30.0,
    },
];

/// W outline, in fractions of the grid width from the top-left corner.
pub const W_SHAPE: [(f32, f32); 13] = [
    (0.06, 0.10),
    (0.25, 0.10),
    (0.37, 0.62),
    (0.45, 0.30),
    (0.55, 0.30),
    (0.63, 0.62),
    (0.75, 0.10),
    (0.94, 0.10),
    (0.73, 0.92),
    (0.57, 0.92),
    (0.50, 0.62),
    (0.43, 0.92),
    (0.27, 0.92),
];

/// Even-odd point-in-polygon test.
pub fn inside_polygon(poly: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn start_noise(c: &mut StartCtx<'_>) {
    steady_plus_noise(&Brusselator, c, 0.01);
}

fn start_w(c: &mut StartCtx<'_>) {
    let n = c.n;
    for i in 0..n {
        for j in 0..n {
            let x = (j as f32 + 0.5) / n as f32;
            let y = (i as f32 + 0.5) / n as f32;
            if inside_polygon(&W_SHAPE, x, y) {
                c.map[i * n + j] = 7.0;
                c.painted[i * n + j] = 1;
            }
        }
    }
    steady_plus_noise(&Brusselator, c, 0.01);
}

static PRESETS: [Preset; 4] = [
    Preset {
        name: "Spots",
        key: "spots",
        params: &[3.0, 6.0],
        du: 4.0,
        dv: 32.0,
        boundary: Boundary::NoFlux,
        range_u: (0.0, 8.0),
        range_v: (0.0, 4.0),
        map_brush: 3.0,
        start: start_noise,
    },
    Preset {
        name: "Stripes",
        key: "stripes",
        params: &[3.0, 4.6],
        du: 4.0,
        dv: 32.0,
        boundary: Boundary::NoFlux,
        range_u: (1.0, 5.0),
        range_v: (1.0, 2.0),
        map_brush: 3.0,
        start: start_noise,
    },
    Preset {
        name: "Oscillating",
        key: "osc",
        params: &[3.0, 12.0],
        du: 4.0,
        dv: 4.0,
        boundary: Boundary::NoFlux,
        range_u: (0.0, 16.0),
        range_v: (0.0, 16.0),
        map_brush: 6.0,
        start: start_noise,
    },
    Preset {
        name: "W prepattern",
        key: "w",
        params: &[3.0, 3.0],
        du: 4.0,
        dv: 32.0,
        boundary: Boundary::NoFlux,
        range_u: (0.0, 8.0),
        range_v: (0.0, 4.0),
        map_brush: 7.0,
        start: start_w,
    },
];

#[inline(always)]
fn rhs<T: Real>(u: T, v: T, a: T, b: T) -> (T, T) {
    let u2v = u * u * v;
    (a - (b + T::c(1.0)) * u + u2v, b * u - u2v)
}

impl Model for Brusselator {
    fn name(&self) -> &'static str {
        "Brusselator"
    }
    fn params(&self) -> &'static [ParamSpec] {
        &PARAMS
    }
    fn painted_param(&self) -> usize {
        B
    }
    fn diffusion(&self) -> (f32, f32) {
        (4.0, 32.0)
    }
    #[inline(always)]
    fn reaction(&self, u: f32, v: f32, p: &[f32], painted: f32) -> (f32, f32) {
        rhs(u, v, p[A], painted)
    }
    fn reaction64(&self, u: f64, v: f64, p: &[f64], painted: f64) -> (f64, f64) {
        rhs(u, v, p[A], painted)
    }
    // u^2 v moves v into u and is stiff during relaxation spikes (large u).
    const IMPLICIT_TRANSFER: bool = true;
    #[inline(always)]
    fn transfer_rate(&self, u: f32, _p: &[f32], _painted: f32) -> f32 {
        u * u
    }
    fn steady_state(&self, p: &[f64]) -> (f64, f64) {
        (p[A], p[B] / p[A])
    }
    fn dt_limit(&self) -> f32 {
        0.005
    }
    fn presets(&self) -> &'static [Preset] {
        &PRESETS
    }
    fn default_speed(&self) -> f32 {
        10.0
    }
    fn brush_defaults(&self) -> (f32, f32) {
        (6.0, 3.0)
    }
}
