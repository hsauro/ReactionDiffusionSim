//! The `Model` trait, parameter specs and presets.

use super::Rng;

/// Numeric type the reaction terms are written against, so each model writes
/// its kinetics once and gets an `f32` version for the grid and an `f64`
/// version for the stability analysis.
pub trait Real:
    Copy
    + std::ops::Add<Output = Self>
    + std::ops::Sub<Output = Self>
    + std::ops::Mul<Output = Self>
    + std::ops::Div<Output = Self>
    + std::ops::Neg<Output = Self>
{
    fn c(x: f64) -> Self;
}

impl Real for f32 {
    #[inline(always)]
    fn c(x: f64) -> Self {
        x as Self
    }
}

impl Real for f64 {
    #[inline(always)]
    fn c(x: f64) -> Self {
        x
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    Periodic,
    NoFlux,
}

impl Boundary {
    pub const ALL: [Self; 2] = [Self::Periodic, Self::NoFlux];

    pub fn label(self) -> &'static str {
        match self {
            Self::Periodic => "Periodic",
            Self::NoFlux => "No flux",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Periodic => "periodic",
            Self::NoFlux => "noflux",
        }
    }

    pub fn from_key(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|b| b.key() == s)
    }
}

/// A scalar model parameter.
pub struct ParamSpec {
    /// Label shown in the UI.
    pub name: &'static str,
    /// Key used in the shareable URL.
    pub key: &'static str,
    pub default: f32,
    /// Slider range (logarithmic).
    pub min: f32,
    pub max: f32,
}

/// Everything a preset's start function may write.
pub struct StartCtx<'a> {
    pub n: usize,
    /// Scalar parameter values (the painted parameter holds the map background).
    pub params: &'a [f32],
    pub u: &'a mut [f32],
    pub v: &'a mut [f32],
    /// Already filled with the background value before the start function runs.
    pub map: &'a mut [f32],
    /// 1 where the map has been painted, 0 elsewhere. Cleared before the start function runs.
    pub painted: &'a mut [u8],
    pub rng: &'a mut Rng,
}

pub struct Preset {
    pub name: &'static str,
    /// Key used in the shareable URL.
    pub key: &'static str,
    pub params: &'static [f32],
    pub du: f32,
    pub dv: f32,
    pub boundary: Boundary,
    /// Display range for u and v.
    pub range_u: (f32, f32),
    pub range_v: (f32, f32),
    /// Default brush value when painting the parameter map.
    pub map_brush: f32,
    pub start: fn(&mut StartCtx<'_>),
}

pub trait Model {
    fn name(&self) -> &'static str;
    fn params(&self) -> &'static [ParamSpec];
    /// Index into `params` of the parameter that has a per-cell map.
    fn painted_param(&self) -> usize;
    /// Default `D_u`, `D_v`.
    fn diffusion(&self) -> (f32, f32);
    /// If `D_v` is fixed by the model's equations rather than set by a slider,
    /// return it here (Lengyel-Epstein: sigma * d * `D_u`; Barkley: 0).
    fn derived_dv(&self, _p: &[f32], _du: f32) -> Option<f32> {
        None
    }
    /// Reaction terms (f, g). `painted` is this cell's map value and replaces
    /// `p[painted_param()]`.
    fn reaction(&self, u: f32, v: f32, p: &[f32], painted: f32) -> (f32, f32);
    /// Set true when the kinetics move material from v to u at rate L(u) v (the term
    /// appears as -L v in g and +L v in f) and that term can be stiff. The step then
    /// takes it implicitly in v and moves exactly what v lost into u. Fixed points are
    /// the same as forward Euler, but u and v cannot overshoot during fast spikes.
    const IMPLICIT_TRANSFER: bool = false;
    /// The rate L(u) of that transfer (only used when `IMPLICIT_TRANSFER` is true).
    fn transfer_rate(&self, _u: f32, _p: &[f32], _painted: f32) -> f32 {
        0.0
    }
    /// Same as `reaction` in double precision, for the stability analysis.
    fn reaction64(&self, u: f64, v: f64, p: &[f64], painted: f64) -> (f64, f64);
    /// Uniform steady state, with `p[painted_param()]` holding the painted value.
    fn steady_state(&self, p: &[f64]) -> (f64, f64);
    fn dt_limit(&self) -> f32;
    fn presets(&self) -> &'static [Preset];
    fn note(&self) -> Option<&'static str> {
        None
    }
    /// Default simulated time per second of wall clock.
    fn default_speed(&self) -> f32;
    /// Default brush values for u and v.
    fn brush_defaults(&self) -> (f32, f32);
}

/// Steady state evaluated in `f32` with a given painted value (used by start functions).
pub fn steady_state_at<M: Model>(m: &M, params: &[f32], painted: f32) -> (f32, f32) {
    let mut p: Vec<f64> = params.iter().map(|&x| x as f64).collect();
    p[m.painted_param()] = painted as f64;
    let (u, v) = m.steady_state(&p);
    (u as f32, v as f32)
}

/// Fill u and v with the steady state for each cell's map value, plus uniform noise of +/- `amp`.
pub fn steady_plus_noise<M: Model>(m: &M, c: &mut StartCtx<'_>, amp: f32) {
    let mut last = f32::NAN;
    let mut ss = (0.0, 0.0);
    for i in 0..c.u.len() {
        let pm = c.map[i];
        if pm != last {
            ss = steady_state_at(m, c.params, pm);
            last = pm;
        }
        c.u[i] = ss.0 + c.rng.sym(amp);
        c.v[i] = ss.1 + c.rng.sym(amp);
    }
}
