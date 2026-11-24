//! The control state that can be shared as a URL fragment, and its encoding,
//! e.g. `m=brusselator&p=spots&a=3&b=6&Du=4&Dv=32&speed=10&N=200&bc=noflux&view=u&cmap=viridis`.

use crate::colormap::Colormap;
use crate::sim::{Boundary, Model as _, ModelKind, View};
use crate::with_model;

pub const GRID_SIZES: [usize; 3] = [100, 200, 300];

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub model: ModelKind,
    pub preset: usize,
    /// Scalar parameters; the painted parameter's entry is the map background.
    pub params: Vec<f32>,
    pub du: f32,
    pub dv: f32,
    pub speed: f32,
    pub n: usize,
    pub boundary: Boundary,
    pub view: View,
    pub colormap: Colormap,
}

impl Settings {
    /// The settings a preset sets, keeping grid size, view and colours.
    pub fn from_preset(model: ModelKind, preset: usize, n: usize, view: View, colormap: Colormap) -> Self {
        with_model!(model, m => {
            let ps = &m.presets()[preset];
            Self {
                model,
                preset,
                params: ps.params.to_vec(),
                du: ps.du,
                dv: ps.dv,
                speed: m.default_speed(),
                n,
                boundary: ps.boundary,
                view,
                colormap,
            }
        })
    }

    pub fn encode(&self) -> String {
        with_model!(self.model, m => {
            let mut s = format!("m={}&p={}", self.model.key(), m.presets()[self.preset].key);
            for (spec, v) in m.params().iter().zip(&self.params) {
                s += &format!("&{}={}", spec.key, v);
            }
            s += &format!(
                "&Du={}&Dv={}&speed={}&N={}&bc={}&view={}&cmap={}",
                self.du,
                self.dv,
                self.speed,
                self.n,
                self.boundary.key(),
                self.view.key(),
                self.colormap.key()
            );
            s
        })
    }

    /// Decode a fragment (with or without a leading '#'). Unknown or invalid
    /// keys fall back to the preset's values without an error.
    pub fn decode(fragment: &str) -> Self {
        let frag = fragment.trim_start_matches('#');
        let pairs: Vec<(&str, &str)> = frag
            .split('&')
            .filter_map(|kv| kv.split_once('='))
            .collect();
        let get = |key: &str| pairs.iter().rev().find(|(k, _)| *k == key).map(|(_, v)| *v);
        let num = |key: &str| {
            get(key)
                .and_then(|v| v.parse::<f32>().ok())
                .filter(|x| x.is_finite())
        };

        let model = get("m").and_then(ModelKind::from_key).unwrap_or(ModelKind::Brusselator);
        let n = get("N")
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|n| GRID_SIZES.contains(n))
            .unwrap_or(200);
        let view = get("view").and_then(View::from_key).unwrap_or(View::U);
        let colormap = get("cmap").and_then(Colormap::from_key).unwrap_or(Colormap::Viridis);

        with_model!(model, m => {
            let preset = get("p")
                .and_then(|key| m.presets().iter().position(|p| p.key == key))
                .unwrap_or(0);
            let mut s = Self::from_preset(model, preset, n, view, colormap);
            for (i, spec) in m.params().iter().enumerate() {
                if let Some(x) = num(spec.key).filter(|&x| x >= spec.min && x <= spec.max) {
                    s.params[i] = x;
                }
            }
            if let Some(x) = num("Du").filter(|&x| x > 1e-4 && x < 1e4) {
                s.du = x;
            }
            if let Some(x) = num("Dv").filter(|&x| (0.0..1e4).contains(&x)) {
                s.dv = x;
            }
            if let Some(x) = num("speed").filter(|&x| x > 0.0 && x < 1e7) {
                s.speed = x;
            }
            if let Some(b) = get("bc").and_then(Boundary::from_key) {
                s.boundary = b;
            }
            s
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_every_preset() {
        for model in ModelKind::ALL {
            let count = with_model!(model, m => m.presets().len());
            for preset in 0..count {
                let mut s = Settings::from_preset(model, preset, 300, View::V, Colormap::Magma);
                s.speed *= 1.7;
                s.du *= 1.1;
                s.boundary = Boundary::Periodic;
                let back = Settings::decode(&format!("#{}", s.encode()));
                assert_eq!(s, back, "round trip failed for {model:?} preset {preset}");
            }
        }
    }

    #[test]
    fn invalid_keys_fall_back() {
        let s = Settings::decode("#m=brusselator&p=nonsense&b=abc&a=1e9&N=17&zzz=1&bc=x");
        assert_eq!(s, Settings::from_preset(ModelKind::Brusselator, 0, 200, View::U, Colormap::Viridis));
        let s = Settings::decode("");
        assert_eq!(s, Settings::from_preset(ModelKind::Brusselator, 0, 200, View::U, Colormap::Viridis));
    }

    #[test]
    fn example_fragment() {
        let s = Settings::decode("#m=brusselator&p=spots&b=6.5&Du=4&Dv=32");
        assert_eq!(s.params, vec![3.0, 6.5]);
        assert_eq!(s.boundary, Boundary::NoFlux);
    }
}
