//! Colour schemes for the species display. Each is a 256-entry RGB table,
//! built by linear interpolation between published anchor colours
//! (viridis uses a polynomial fit).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Colormap {
    Viridis,
    Magma,
    Inferno,
    Plasma,
    Turbo,
    CoolWarm,
    Greys,
}

impl Colormap {
    pub const ALL: [Self; 7] = [
        Self::Viridis,
        Self::Magma,
        Self::Inferno,
        Self::Plasma,
        Self::Turbo,
        Self::CoolWarm,
        Self::Greys,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Viridis => "Viridis",
            Self::Magma => "Magma",
            Self::Inferno => "Inferno",
            Self::Plasma => "Plasma",
            Self::Turbo => "Turbo",
            Self::CoolWarm => "Cool\u{2013}warm",
            Self::Greys => "Greyscale",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Viridis => "viridis",
            Self::Magma => "magma",
            Self::Inferno => "inferno",
            Self::Plasma => "plasma",
            Self::Turbo => "turbo",
            Self::CoolWarm => "coolwarm",
            Self::Greys => "greys",
        }
    }

    pub fn from_key(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.key() == s)
    }

    /// 256 RGB entries from low to high.
    pub fn table(self) -> [[u8; 3]; 256] {
        match self {
            Self::Viridis => viridis(),
            Self::Magma => from_stops(&[
                0x000004, 0x1c1044, 0x4f127b, 0x812581, 0xb5367a, 0xe55064, 0xfb8761, 0xfec287,
                0xfcfdbf,
            ]),
            Self::Inferno => from_stops(&[
                0x000004, 0x1f0c48, 0x550f6d, 0x88226a, 0xba3655, 0xe35933, 0xf98e09, 0xf9cb35,
                0xfcffa4,
            ]),
            Self::Plasma => from_stops(&[
                0x0d0887, 0x41049d, 0x6a00a8, 0x8f0da4, 0xb12a90, 0xcc4778, 0xe16462, 0xf2844b,
                0xfca636, 0xfcce25, 0xf0f921,
            ]),
            Self::Turbo => from_stops(&[
                0x30123b, 0x4145ab, 0x4675ed, 0x39a2fc, 0x1bcfd4, 0x24eca6, 0x61fc6c, 0xa4fc3b,
                0xd1e834, 0xf3c63a, 0xfe9b2d, 0xf36315, 0xd93806, 0xb11901, 0x7a0403,
            ]),
            Self::CoolWarm => from_stops(&[
                0x3b4cc0, 0x6788ee, 0x9abbff, 0xc9d7f0, 0xedd1c2, 0xf7a889, 0xe26952, 0xb40426,
            ]),
            Self::Greys => from_stops(&[0x000000, 0xffffff]),
        }
    }
}

fn from_stops(stops: &[u32]) -> [[u8; 3]; 256] {
    let rgb = |c: u32| [(c >> 16) as f32, ((c >> 8) & 0xff) as f32, (c & 0xff) as f32];
    let segments = (stops.len() - 1) as f32;
    let mut out = [[0u8; 3]; 256];
    for (i, px) in out.iter_mut().enumerate() {
        let x = i as f32 / 255.0 * segments;
        let k = (x.floor() as usize).min(stops.len() - 2);
        let t = x - k as f32;
        let (a, b) = (rgb(stops[k]), rgb(stops[k + 1]));
        for ch in 0..3 {
            px[ch] = (a[ch] + t * (b[ch] - a[ch])).round() as u8;
        }
    }
    out
}

/// Viridis, polynomial fit.
fn viridis() -> [[u8; 3]; 256] {
    const C: [[f32; 3]; 7] = [
        [0.277_727_33, 0.005_407_344_5, 0.334_099_8],
        [0.105_093_04, 1.404_613_5, 1.384_590_2],
        [-0.330_861_83, 0.214_847_56, 0.095_095_16],
        [-4.634_230_5, -5.799_101, -19.332_441],
        [6.228_27, 14.179_933, 56.690_55],
        [4.776_385, -13.745_145, -65.353_03],
        [-5.435_456, 4.645_852_6, 26.312_435],
    ];
    let mut out = [[0u8; 3]; 256];
    for (i, px) in out.iter_mut().enumerate() {
        let t = i as f32 / 255.0;
        for (ch, o) in px.iter_mut().enumerate() {
            let mut x = C[6][ch];
            for k in (0..6).rev() {
                x = C[k][ch] + t * x;
            }
            *o = (x.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    out
}
