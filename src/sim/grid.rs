//! Fields, the five-point Laplacian, boundaries, time stepping and the brush.

use super::model::{Boundary, Model, Preset, StartCtx};
use super::Rng;

pub struct Grid {
    pub n: usize,
    pub u: Vec<f32>,
    pub v: Vec<f32>,
    u2: Vec<f32>,
    v2: Vec<f32>,
    /// Per-cell value of the model's painted parameter.
    pub map: Vec<f32>,
    /// 1 where the map has been painted.
    pub painted: Vec<u8>,
    pub boundary: Boundary,
}

impl Grid {
    pub fn new(n: usize, boundary: Boundary) -> Self {
        let len = n * n;
        Self {
            n,
            u: vec![0.0; len],
            v: vec![0.0; len],
            u2: vec![0.0; len],
            v2: vec![0.0; len],
            map: vec![0.0; len],
            painted: vec![0; len],
            boundary,
        }
    }

    pub fn resize(&mut self, n: usize) {
        if n != self.n {
            *self = Self::new(n, self.boundary);
        }
    }

    pub fn any_painted(&self) -> bool {
        self.painted.iter().any(|&p| p != 0)
    }

    /// Set the map background: every cell while unpainted, otherwise only the unpainted cells.
    pub fn set_map_background(&mut self, value: f32) {
        for (m, &p) in self.map.iter_mut().zip(&self.painted) {
            if p == 0 {
                *m = value;
            }
        }
    }

    /// Return every map cell to `value` and forget what was painted.
    pub fn clear_map(&mut self, value: f32) {
        self.map.fill(value);
        self.painted.fill(0);
    }

    pub fn add_noise(&mut self, rng: &mut Rng, amp: f32) {
        for x in self.u.iter_mut().chain(self.v.iter_mut()) {
            *x += rng.sym(amp);
        }
    }

    /// True if every value of u and v is finite.
    pub fn is_finite(&self) -> bool {
        self.u.iter().chain(&self.v).all(|x| x.is_finite())
    }
}

/// Reset the grid to a preset's start state using the given scalar parameters.
pub fn apply_start<M: Model>(m: &M, preset: &Preset, params: &[f32], g: &mut Grid, rng: &mut Rng) {
    g.map.fill(params[m.painted_param()]);
    g.painted.fill(0);
    g.u.fill(0.0);
    g.v.fill(0.0);
    let mut ctx = StartCtx {
        n: g.n,
        params,
        u: &mut g.u,
        v: &mut g.v,
        map: &mut g.map,
        painted: &mut g.painted,
        rng,
    };
    (preset.start)(&mut ctx);
}

/// Flush values below 1e-15 to zero. A field decaying toward 0 (v in Gray-Scott)
/// otherwise produces subnormal floats (directly or in products such as u v^2),
/// which are very slow on x86, and
/// WebAssembly has no flush-to-zero mode.
#[inline(always)]
fn flush(x: f32) -> f32 {
    if x.abs() < 1e-15 { 0.0 } else { x }
}

/// Update one cell. `su`, `sv` are the sums of the four neighbours.
#[inline(always)]
#[expect(clippy::too_many_arguments)]
fn cell<M: Model>(
    m: &M,
    p: &[f32],
    u: f32,
    su: f32,
    v: f32,
    sv: f32,
    pm: f32,
    cu: f32,
    cv: f32,
    dt: f32,
) -> (f32, f32) {
    let (f, g) = m.reaction(u, v, p, pm);
    let mut un = u + dt * f + cu * (su - 4.0 * u);
    let mut vn = v + dt * g + cv * (sv - 4.0 * v);
    if M::IMPLICIT_TRANSFER {
        // The term L v leaves v and enters u. Take it implicitly in v and move
        // exactly the amount v lost into u, so u + v cannot overshoot.
        let l = dt * m.transfer_rate(u, p, pm);
        let v_new = (vn + l * v) / (1.0 + l);
        un += l * (v_new - v);
        vn = v_new;
    }
    (flush(un), flush(vn))
}

/// One forward-Euler step: x_new = x + dt (reaction + D lap x).
pub fn step<M: Model>(m: &M, g: &mut Grid, p: &[f32], du: f32, dv: f32, dt: f32) {
    let n = g.n;
    if n < 3 {
        return;
    }
    let periodic = g.boundary == Boundary::Periodic;
    let (cu, cv) = (du * dt, dv * dt);
    let Grid {
        u, v, u2, v2, map, ..
    } = g;

    for i in 0..n {
        let up = if i == 0 {
            if periodic { n - 1 } else { 0 }
        } else {
            i - 1
        };
        let dn = if i == n - 1 {
            if periodic { 0 } else { n - 1 }
        } else {
            i + 1
        };
        let r = i * n;
        let uc = &u[r..r + n];
        let uu = &u[up * n..up * n + n];
        let ud = &u[dn * n..dn * n + n];
        let vc = &v[r..r + n];
        let vu = &v[up * n..up * n + n];
        let vd = &v[dn * n..dn * n + n];
        let pm = &map[r..r + n];
        let uo = &mut u2[r..r + n];
        let vo = &mut v2[r..r + n];

        // Left edge.
        let jl = if periodic { n - 1 } else { 0 };
        let (a, b) = cell(
            m,
            p,
            uc[0],
            uc[jl] + uc[1] + uu[0] + ud[0],
            vc[0],
            vc[jl] + vc[1] + vu[0] + vd[0],
            pm[0],
            cu,
            cv,
            dt,
        );
        uo[0] = a;
        vo[0] = b;

        // Interior: plain loop over equal-length slices, no wrap logic, so the
        // bounds checks are removed and the loop can be vectorised.
        {
            let k = n - 2;
            let (ul, uc1, ur) = (&uc[..k], &uc[1..=k], &uc[2..]);
            let (vl, vc1, vr) = (&vc[..k], &vc[1..=k], &vc[2..]);
            let (uu1, ud1, vu1, vd1) = (&uu[1..=k], &ud[1..=k], &vu[1..=k], &vd[1..=k]);
            let pm1 = &pm[1..=k];
            let (uo1, vo1) = (&mut uo[1..=k], &mut vo[1..=k]);
            for j in 0..k {
                let su = ul[j] + ur[j] + uu1[j] + ud1[j];
                let sv = vl[j] + vr[j] + vu1[j] + vd1[j];
                let (a, b) = cell(m, p, uc1[j], su, vc1[j], sv, pm1[j], cu, cv, dt);
                uo1[j] = a;
                vo1[j] = b;
            }
        }

        // Right edge.
        let e = n - 1;
        let jr = if periodic { 0 } else { e };
        let (a, b) = cell(
            m,
            p,
            uc[e],
            uc[e - 1] + uc[jr] + uu[e] + ud[e],
            vc[e],
            vc[e - 1] + vc[jr] + vu[e] + vd[e],
            pm[e],
            cu,
            cv,
            dt,
        );
        uo[e] = a;
        vo[e] = b;
    }
    std::mem::swap(u, u2);
    std::mem::swap(v, v2);
}

/// Take steps of `dt` while `pending` simulated time remains and `keep_going()` is true.
/// Returns the number of steps taken; `pending` is reduced by the time simulated.
#[expect(clippy::too_many_arguments)]
pub fn run_steps<M: Model>(
    m: &M,
    g: &mut Grid,
    p: &[f32],
    du: f32,
    dv: f32,
    dt: f32,
    pending: &mut f64,
    mut keep_going: impl FnMut() -> bool,
) -> usize {
    let mut steps = 0;
    while *pending >= dt as f64 {
        step(m, g, p, du, dv, dt);
        *pending -= dt as f64;
        steps += 1;
        if !keep_going() {
            break;
        }
    }
    steps
}

/// Blend `field` toward `value` around (cx, cy) in grid units (cell (i, j) has its
/// centre at (j + 0.5, i + 0.5)). Weight w = exp(-d^2 / 2 sigma^2) within 2 sigma,
/// sigma = radius / 2. Cells with w > 0.05 are marked in `mark` if given.
#[expect(clippy::too_many_arguments)]
pub fn stamp(
    field: &mut [f32],
    n: usize,
    periodic: bool,
    cx: f32,
    cy: f32,
    radius: f32,
    value: f32,
    mut mark: Option<&mut [u8]>,
) {
    let sigma = (radius / 2.0).max(0.25);
    let reach = 2.0 * sigma;
    let inv = 1.0 / (2.0 * sigma * sigma);
    let ni = n as i64;
    let j0 = (cx - reach).floor() as i64;
    let j1 = (cx + reach).ceil() as i64;
    let i0 = (cy - reach).floor() as i64;
    let i1 = (cy + reach).ceil() as i64;
    for i in i0..=i1 {
        let ii = if periodic {
            i.rem_euclid(ni)
        } else if i < 0 || i >= ni {
            continue;
        } else {
            i
        };
        let dy = i as f32 + 0.5 - cy;
        for j in j0..=j1 {
            let jj = if periodic {
                j.rem_euclid(ni)
            } else if j < 0 || j >= ni {
                continue;
            } else {
                j
            };
            let dx = j as f32 + 0.5 - cx;
            let d2 = dx * dx + dy * dy;
            if d2 > reach * reach {
                continue;
            }
            let w = (-d2 * inv).exp();
            let k = ii as usize * n + jj as usize;
            field[k] += w * (value - field[k]);
            if let Some(mark) = mark.as_deref_mut()
                && w > 0.05
            {
                mark[k] = 1;
            }
        }
    }
}

/// Minimum and maximum of a field.
pub fn min_max(field: &[f32]) -> (f32, f32) {
    field
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &x| {
            (lo.min(x), hi.max(x))
        })
}

/// Spatial standard deviation of a field.
pub fn std_dev(field: &[f32]) -> f64 {
    let n = field.len() as f64;
    let mean = field.iter().map(|&x| x as f64).sum::<f64>() / n;
    let var = field
        .iter()
        .map(|&x| {
            let d = x as f64 - mean;
            d * d
        })
        .sum::<f64>()
        / n;
    var.sqrt()
}
