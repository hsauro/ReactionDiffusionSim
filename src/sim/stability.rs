//! Linear stability of the uniform steady state: Jacobian by central
//! differences and the dispersion relation on the discrete grid.

use super::Model;

/// Number of wavenumbers sampled between 0 and pi.
pub const SAMPLES: usize = 400;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Verdict {
    Oscillates,
    Unstable,
    Turing { wavelength: f64 },
    Stable,
}

impl Verdict {
    pub fn text(&self) -> String {
        match self {
            Self::Oscillates => "Oscillates (well mixed)".to_owned(),
            Self::Unstable => "Unstable even when well mixed".to_owned(),
            Self::Turing { wavelength } => {
                format!("Turing pattern, wavelength 2\u{3c0}/k* = {wavelength:.1} cells")
            }
            Self::Stable => "Stable uniform state".to_owned(),
        }
    }
}

/// An eigenvalue as (real, imaginary).
pub type Eig = (f64, f64);

#[derive(Clone, Debug)]
pub struct Dispersion {
    pub steady: (f64, f64),
    pub jacobian: [[f64; 2]; 2],
    /// Sampled wavenumbers and the largest real part of the eigenvalues at each.
    pub ks: Vec<f64>,
    pub growth: Vec<f64>,
    /// Fastest-growing k > 0 (refined between samples) and its growth rate.
    pub k_star: f64,
    pub max_growth: f64,
    pub eig0: [Eig; 2],
    pub verdict: Verdict,
}

/// Discrete Laplacian symbol along one axis: q(k) = 4 sin^2(k/2).
pub fn q_of_k(k: f64) -> f64 {
    let s = (0.5 * k).sin();
    4.0 * s * s
}

/// Eigenvalues of a 2x2 matrix from trace and determinant.
pub fn eig2(a: [[f64; 2]; 2]) -> [Eig; 2] {
    let tr = a[0][0] + a[1][1];
    let det = a[0][0] * a[1][1] - a[0][1] * a[1][0];
    let disc = 0.25 * tr * tr - det;
    if disc >= 0.0 {
        let s = disc.sqrt();
        [(0.5 * tr + s, 0.0), (0.5 * tr - s, 0.0)]
    } else {
        let s = (-disc).sqrt();
        [(0.5 * tr, s), (0.5 * tr, -s)]
    }
}

/// Largest real part of the eigenvalues of J - q(k) diag(du, dv).
pub fn growth_at(j: [[f64; 2]; 2], du: f64, dv: f64, k: f64) -> f64 {
    let q = q_of_k(k);
    let a = [[j[0][0] - q * du, j[0][1]], [j[1][0], j[1][1] - q * dv]];
    eig2(a)[0].0
}

/// Jacobian of the reaction terms at (u, v) by central differences,
/// step 1e-6 relative to the value (absolute 1e-6 for values below 1).
pub fn jacobian<M: Model>(m: &M, p: &[f64], painted: f64, u: f64, v: f64) -> [[f64; 2]; 2] {
    let hu = 1e-6 * u.abs().max(1.0);
    let hv = 1e-6 * v.abs().max(1.0);
    let (fu1, gu1) = m.reaction64(u + hu, v, p, painted);
    let (fu0, gu0) = m.reaction64(u - hu, v, p, painted);
    let (fv1, gv1) = m.reaction64(u, v + hv, p, painted);
    let (fv0, gv0) = m.reaction64(u, v - hv, p, painted);
    [
        [(fu1 - fu0) / (2.0 * hu), (fv1 - fv0) / (2.0 * hv)],
        [(gu1 - gu0) / (2.0 * hu), (gv1 - gv0) / (2.0 * hv)],
    ]
}

/// Analyse the uniform steady state for scalar parameters `params`, with the
/// painted parameter set to `painted`, and effective diffusion constants.
pub fn analyze<M: Model>(m: &M, params: &[f32], painted: f32, du: f32, dv: f32) -> Dispersion {
    let mut p: Vec<f64> = params.iter().map(|&x| x as f64).collect();
    let painted = painted as f64;
    p[m.painted_param()] = painted;
    let (du, dv) = (du as f64, dv as f64);
    let steady = m.steady_state(&p);
    let j = jacobian(m, &p, painted, steady.0, steady.1);

    let ks: Vec<f64> = (0..SAMPLES)
        .map(|i| std::f64::consts::PI * i as f64 / (SAMPLES - 1) as f64)
        .collect();
    let growth: Vec<f64> = ks.iter().map(|&k| growth_at(j, du, dv, k)).collect();

    // Fastest-growing k > 0, refined by golden-section search around the best sample.
    let mut best = 1;
    for i in 2..SAMPLES {
        if growth[i] > growth[best] {
            best = i;
        }
    }
    let lo = ks[best - 1];
    let hi = ks[(best + 1).min(SAMPLES - 1)];
    let (k_star, max_growth) = golden_max(|k| growth_at(j, du, dv, k), lo, hi);

    let eig0 = eig2(j);
    let g0 = eig0[0].0;
    let complex0 = eig0[0].1 != 0.0;
    let verdict = if g0 > 0.0 {
        if complex0 {
            Verdict::Oscillates
        } else {
            Verdict::Unstable
        }
    } else if max_growth > 0.0 {
        Verdict::Turing {
            wavelength: 2.0 * std::f64::consts::PI / k_star,
        }
    } else {
        Verdict::Stable
    };

    Dispersion {
        steady,
        jacobian: j,
        ks,
        growth,
        k_star,
        max_growth,
        eig0,
        verdict,
    }
}

fn golden_max(f: impl Fn(f64) -> f64, mut a: f64, mut b: f64) -> (f64, f64) {
    let r = 0.5 * (5f64.sqrt() - 1.0);
    let mut c = b - r * (b - a);
    let mut d = a + r * (b - a);
    let mut fc = f(c);
    let mut fd = f(d);
    for _ in 0..80 {
        if fc > fd {
            b = d;
            d = c;
            fd = fc;
            c = b - r * (b - a);
            fc = f(c);
        } else {
            a = c;
            c = d;
            fc = fd;
            d = a + r * (b - a);
            fd = f(d);
        }
    }
    let k = 0.5 * (a + b);
    (k, f(k))
}

/// Format a number to three significant figures.
pub fn sig3(x: f64) -> String {
    if x == 0.0 || !x.is_finite() {
        return format!("{x}");
    }
    let mag = x.abs().log10().floor() as i32;
    if !(-3..=5).contains(&mag) {
        return format!("{x:.2e}");
    }
    let decimals = (2 - mag).max(0) as usize;
    format!("{x:.decimals$}")
}

/// Format an eigenvalue.
pub fn fmt_eig(e: Eig) -> String {
    if e.1 == 0.0 {
        sig3(e.0)
    } else if e.1 > 0.0 {
        format!("{} + {}i", sig3(e.0), sig3(e.1))
    } else {
        format!("{} - {}i", sig3(e.0), sig3(-e.1))
    }
}
