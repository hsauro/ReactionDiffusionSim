//! Simulation core. No egui dependency, so it can be tested on its own.

pub mod grid;
pub mod model;
pub mod models;
pub mod stability;

#[cfg(test)]
mod tests;

pub use grid::Grid;
pub use model::{Boundary, Model, ParamSpec, Preset, StartCtx};
pub use models::{Barkley, Brusselator, GrayScott, LengyelEpstein};

/// Small, fast, seedable random number generator (`SplitMix64`).
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Uniform in [-amp, amp).
    pub fn sym(&mut self, amp: f32) -> f32 {
        amp * (2.0 * self.unit() - 1.0)
    }
}

/// The built-in models. Dispatch happens with a `match` once per frame
/// (see [`with_model!`]), never through `dyn Model` per cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelKind {
    Brusselator,
    GrayScott,
    LengyelEpstein,
    Barkley,
}

impl ModelKind {
    pub const ALL: [Self; 4] = [
        Self::Brusselator,
        Self::GrayScott,
        Self::LengyelEpstein,
        Self::Barkley,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Brusselator => "brusselator",
            Self::GrayScott => "grayscott",
            Self::LengyelEpstein => "lengyelepstein",
            Self::Barkley => "barkley",
        }
    }

    pub fn from_key(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.key() == s)
    }
}

/// Run `$body` with `$m` bound to a reference to the concrete model.
#[macro_export]
macro_rules! with_model {
    ($kind:expr, $m:ident => $body:expr) => {
        match $kind {
            $crate::sim::ModelKind::Brusselator => {
                let $m = &$crate::sim::Brusselator;
                $body
            }
            $crate::sim::ModelKind::GrayScott => {
                let $m = &$crate::sim::GrayScott;
                $body
            }
            $crate::sim::ModelKind::LengyelEpstein => {
                let $m = &$crate::sim::LengyelEpstein;
                $body
            }
            $crate::sim::ModelKind::Barkley => {
                let $m = &$crate::sim::Barkley;
                $body
            }
        }
    };
}

/// Which field the canvas shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    U,
    V,
    Map,
}

impl View {
    pub const ALL: [Self; 3] = [Self::U, Self::V, Self::Map];

    pub fn key(self) -> &'static str {
        match self {
            Self::U => "u",
            Self::V => "v",
            Self::Map => "map",
        }
    }

    pub fn from_key(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.key() == s)
    }
}

/// Time step for a model at the given diffusion constants:
/// min(model limit, 0.2 / max(`D_u`, `D_v`)).
pub fn time_step<M: Model>(m: &M, du: f32, dv: f32) -> f32 {
    let dmax = du.max(dv);
    if dmax > 0.0 {
        m.dt_limit().min(0.2 / dmax)
    } else {
        m.dt_limit()
    }
}

/// The effective `D_v`: derived from the parameters for models that fix it.
pub fn effective_dv<M: Model>(m: &M, p: &[f32], du: f32, dv: f32) -> f32 {
    m.derived_dv(p, du).unwrap_or(dv)
}
