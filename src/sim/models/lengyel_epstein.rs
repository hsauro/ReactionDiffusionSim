//! Lengyel-Epstein (CIMA reaction):
//! u' = a - u - 4uv/(1+u^2) + `D_u` lap u,
//! v' = sigma [ b (u - uv/(1+u^2)) + d `D_u` lap v ].
//!
//! The v diffusion constant is therefore sigma * d * `D_u`, set by the parameters.

use crate::sim::model::{steady_plus_noise, Boundary, Model, ParamSpec, Preset, Real, StartCtx};

pub struct LengyelEpstein;

const A: usize = 0;
const B: usize = 1;
const SIGMA: usize = 2;
const D: usize = 3;

static PARAMS: [ParamSpec; 4] = [
    ParamSpec {
        name: "a",
        key: "a",
        default: 12.0,
        min: 3.0,
        max: 48.0,
    },
    ParamSpec {
        name: "b",
        key: "b",
        default: 0.2,
        min: 0.02,
        max: 2.0,
    },
    ParamSpec {
        name: "\u{3c3}",
        key: "sigma",
        default: 50.0,
        min: 12.5,
        max: 200.0,
    },
    ParamSpec {
        name: "d",
        key: "d",
        default: 1.0,
        min: 0.25,
        max: 4.0,
    },
];

fn start_noise(c: &mut StartCtx<'_>) {
    steady_plus_noise(&LengyelEpstein, c, 0.01);
}

const fn preset(name: &'static str, key: &'static str, params: &'static [f32], map_brush: f32) -> Preset {
    Preset {
        name,
        key,
        params,
        du: 1.5,
        dv: 75.0,
        boundary: Boundary::Periodic,
        range_u: (0.0, 6.0),
        range_v: (5.5, 8.5),
        map_brush,
        start: start_noise,
    }
}

static PRESETS: [Preset; 3] = [
    preset("Maze", "maze", &[12.0, 0.20, 50.0, 1.0], 0.5),
    preset("Spots", "spots", &[12.0, 0.30, 50.0, 1.0], 0.5),
    preset("Stable", "stable", &[12.0, 0.50, 50.0, 1.0], 0.2),
];

#[inline(always)]
fn rhs<T: Real>(u: T, v: T, a: T, b: T, sigma: T) -> (T, T) {
    let w = u * v / (T::c(1.0) + u * u);
    (a - u - T::c(4.0) * w, sigma * b * (u - w))
}

impl Model for LengyelEpstein {
    fn name(&self) -> &'static str {
        "Lengyel\u{2013}Epstein"
    }
    fn params(&self) -> &'static [ParamSpec] {
        &PARAMS
    }
    fn painted_param(&self) -> usize {
        B
    }
    fn diffusion(&self) -> (f32, f32) {
        (1.5, 75.0)
    }
    fn derived_dv(&self, p: &[f32], du: f32) -> Option<f32> {
        Some(p[SIGMA] * p[D] * du)
    }
    #[inline(always)]
    fn reaction(&self, u: f32, v: f32, p: &[f32], painted: f32) -> (f32, f32) {
        rhs(u, v, p[A], painted, p[SIGMA])
    }
    fn reaction64(&self, u: f64, v: f64, p: &[f64], painted: f64) -> (f64, f64) {
        rhs(u, v, p[A], painted, p[SIGMA])
    }
    fn steady_state(&self, p: &[f64]) -> (f64, f64) {
        let a = p[A];
        (a / 5.0, 1.0 + a * a / 25.0)
    }
    fn dt_limit(&self) -> f32 {
        0.0025
    }
    fn presets(&self) -> &'static [Preset] {
        &PRESETS
    }
    fn default_speed(&self) -> f32 {
        4.0
    }
    fn brush_defaults(&self) -> (f32, f32) {
        (5.0, 8.0)
    }
}
