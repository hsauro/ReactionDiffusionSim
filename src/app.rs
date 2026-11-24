//! UI, input, painting and the canvas texture.

use std::collections::VecDeque;

use egui::{Color32, ColorImage, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions, Vec2};
use egui_plot::{HLine, Legend, Line, Plot, PlotPoints, VLine};
use web_time::Instant;

use crate::sim::model::steady_state_at;
use crate::sim::{self, grid, stability, Boundary, Grid, Model as _, ModelKind, Rng, View};
use crate::url_state::{Settings, GRID_SIZES};
use crate::with_model;

/// Wall-clock time per frame that the simulation may use.
const FRAME_BUDGET_S: f64 = 0.012;
/// Probe history length, in frames.
const PROBE_FRAMES: usize = 2000;
/// Largest canvas side, in points.
const MAX_CANVAS_PX: f32 = 560.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BrushTarget {
    U,
    V,
    Map,
    Probe,
}

pub struct RdApp {
    s: Settings,
    grid: Grid,
    rng: Rng,
    running: bool,
    /// Simulated time.
    t: f64,
    /// Simulated time asked for but not yet stepped (always < one dt after a frame).
    pending: f64,
    unstable: bool,

    texture: Option<TextureHandle>,
    lut: [Color32; 256],
    auto_scale: bool,
    range_u: (f32, f32),
    range_v: (f32, f32),
    /// Range used for the last frame's image (for the colour bar and status bar).
    shown_range: (f32, f32),
    shown_min_max: (f32, f32),

    brush_target: BrushTarget,
    brush_radius: f32,
    /// Brush values for u, v and the map.
    brush_values: [f32; 3],
    /// Last stamped point of the current stroke, in grid units.
    last_stamp: Option<Pos2>,

    probe: (usize, usize),
    probe_hist: VecDeque<[f64; 3]>,
    hover: Option<(usize, usize)>,

    last_frame: Option<Instant>,
    fps: f64,
    sim_rate: f64,
    message: Option<(String, Instant)>,
}

impl RdApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let s = Settings::decode(&initial_fragment());
        let n = s.n;
        let mut app = Self {
            grid: Grid::new(n, s.boundary),
            s,
            rng: Rng::new(0x5eed_1234),
            running: true,
            t: 0.0,
            pending: 0.0,
            unstable: false,
            texture: None,
            lut: viridis_lut(),
            auto_scale: false,
            range_u: (0.0, 1.0),
            range_v: (0.0, 1.0),
            shown_range: (0.0, 1.0),
            shown_min_max: (0.0, 1.0),
            brush_target: BrushTarget::U,
            brush_radius: 8.0,
            brush_values: [1.0, 1.0, 1.0],
            last_stamp: None,
            probe: (n / 2, n / 2),
            probe_hist: VecDeque::with_capacity(PROBE_FRAMES),
            hover: None,
            last_frame: None,
            fps: 0.0,
            sim_rate: 0.0,
            message: None,
        };
        app.load_preset_display();
        app.reset();
        app
    }

    // ------------------------------------------------------------------
    // State changes

    /// Apply a preset: every parameter, the start state, the boundary and the display range.
    fn apply_preset(&mut self, model: ModelKind, preset: usize) {
        let speed = self.s.speed;
        let same_model = model == self.s.model;
        self.s = Settings::from_preset(model, preset, self.s.n, self.s.view);
        if same_model {
            self.s.speed = speed;
        }
        self.load_preset_display();
        self.reset();
    }

    /// Display ranges and brush defaults from the current preset.
    fn load_preset_display(&mut self) {
        with_model!(self.s.model, m => {
            let ps = &m.presets()[self.s.preset];
            self.range_u = ps.range_u;
            self.range_v = ps.range_v;
            let (bu, bv) = m.brush_defaults();
            self.brush_values = [bu, bv, ps.map_brush];
        });
    }

    /// Restart from the current preset's start state with the current parameter values.
    fn reset(&mut self) {
        self.grid.resize(self.s.n);
        self.grid.boundary = self.s.boundary;
        with_model!(self.s.model, m => {
            let ps = &m.presets()[self.s.preset];
            grid::apply_start(m, ps, &self.s.params, &mut self.grid, &mut self.rng);
        });
        self.t = 0.0;
        self.pending = 0.0;
        self.unstable = false;
        let n = self.s.n;
        if self.probe.0 >= n || self.probe.1 >= n {
            self.probe = (n / 2, n / 2);
        }
        self.probe_hist.clear();
        self.record_probe();
    }

    fn painted_index(&self) -> usize {
        with_model!(self.s.model, m => m.painted_param())
    }

    fn effective_dv(&self) -> f32 {
        with_model!(self.s.model, m => sim::effective_dv(m, &self.s.params, self.s.du, self.s.dv))
    }

    fn dt(&self) -> f32 {
        let dv = self.effective_dv();
        with_model!(self.s.model, m => sim::time_step(m, self.s.du, dv))
    }

    /// Advance by `target` simulated time, within the frame budget. Returns the simulated time done.
    fn simulate(&mut self, target: f64) -> f64 {
        if self.unstable {
            return 0.0;
        }
        let start = Instant::now();
        self.pending += target;
        let dv = self.effective_dv();
        let du = self.s.du;
        let (steps, dt) = with_model!(self.s.model, m => {
            let dt = sim::time_step(m, du, dv);
            let steps = grid::run_steps(
                m,
                &mut self.grid,
                &self.s.params,
                du,
                dv,
                dt,
                &mut self.pending,
                || start.elapsed().as_secs_f64() < FRAME_BUDGET_S,
            );
            (steps, dt as f64)
        });
        // Never shorten dt or carry a backlog to catch up.
        self.pending = self.pending.min(dt);
        let done = steps as f64 * dt;
        self.t += done;
        if steps > 0 {
            if !self.grid.is_finite() {
                self.unstable = true;
                self.running = false;
            }
            self.record_probe();
        }
        done
    }

    fn record_probe(&mut self) {
        let k = self.probe.0 * self.s.n + self.probe.1;
        if k < self.grid.u.len() {
            if self.probe_hist.len() == PROBE_FRAMES {
                self.probe_hist.pop_front();
            }
            self.probe_hist
                .push_back([self.t, self.grid.u[k] as f64, self.grid.v[k] as f64]);
        }
    }

    /// Background values the right button paints toward: the steady state for u and v
    /// and the slider value for the map.
    fn background_values(&self) -> [f32; 3] {
        let pp = self.painted_index();
        let bg = self.s.params[pp];
        let (u, v) = with_model!(self.s.model, m => steady_state_at(m, &self.s.params, bg));
        [u, v, bg]
    }

    /// Stamp along the stroke from the last stamped point to `to` at spacing sigma.
    fn paint_stroke(&mut self, to: Pos2, toward_background: bool) {
        let idx = match self.brush_target {
            BrushTarget::U => 0,
            BrushTarget::V => 1,
            BrushTarget::Map => 2,
            BrushTarget::Probe => return,
        };
        let value = if toward_background {
            self.background_values()[idx]
        } else {
            self.brush_values[idx]
        };
        let sigma = (self.brush_radius / 2.0).max(0.5);
        let mut points = Vec::new();
        match self.last_stamp {
            None => points.push(to),
            Some(mut p) => {
                let mut d = to - p;
                while d.length() >= sigma {
                    p += d.normalized() * sigma;
                    points.push(p);
                    d = to - p;
                }
            }
        }
        let Some(&last) = points.last() else {
            return;
        };
        self.last_stamp = Some(last);

        let n = self.grid.n;
        let periodic = self.grid.boundary == Boundary::Periodic;
        let r = self.brush_radius;
        for p in points {
            match idx {
                0 => grid::stamp(&mut self.grid.u, n, periodic, p.x, p.y, r, value, None),
                1 => grid::stamp(&mut self.grid.v, n, periodic, p.x, p.y, r, value, None),
                _ => {
                    let mark = if toward_background {
                        None
                    } else {
                        Some(&mut self.grid.painted[..])
                    };
                    grid::stamp(&mut self.grid.map, n, periodic, p.x, p.y, r, value, mark);
                }
            }
        }
        // u, v >= 0 after painting.
        if idx < 2 {
            for x in self.grid.u.iter_mut().chain(self.grid.v.iter_mut()) {
                *x = x.max(0.0);
            }
        }
    }

    fn copy_link(&mut self, ctx: &egui::Context) {
        let frag = self.s.encode();
        #[cfg(target_arch = "wasm32")]
        let url = {
            let mut url = format!("#{frag}");
            if let Some(loc) = web_sys::window().map(|w| w.location()) {
                if let Ok(href) = loc.href() {
                    let base = href.split('#').next().unwrap_or_default();
                    url = format!("{base}#{frag}");
                }
                let _ = loc.set_hash(&frag);
            }
            url
        };
        #[cfg(not(target_arch = "wasm32"))]
        let url = format!("#{frag}");
        ctx.copy_text(url);
        self.message = Some(("Link copied".to_owned(), Instant::now()));
    }

    // ------------------------------------------------------------------
    // Drawing

    fn shown_field(&self) -> &[f32] {
        match self.s.view {
            View::U => &self.grid.u,
            View::V => &self.grid.v,
            View::Map => &self.grid.map,
        }
    }

    fn update_texture(&mut self, ctx: &egui::Context) {
        let n = self.grid.n;
        let field = self.shown_field();
        let (lo, hi) = grid::min_max(field);
        let (mut a, mut b) = match self.s.view {
            _ if self.auto_scale => (lo, hi),
            View::U => self.range_u,
            View::V => self.range_v,
            View::Map => (lo, hi),
        };
        if !(b - a).is_finite() || b - a <= 1e-12 {
            let mid = if a.is_finite() { a } else { 0.0 };
            let half = (mid.abs() * 0.5).max(1e-3);
            a = mid - half;
            b = mid + half;
        }
        let scale = 255.0 / (b - a);
        let grey = self.s.view == View::Map;
        let pixels: Vec<Color32> = field
            .iter()
            .map(|&x| {
                let t = ((x - a) * scale).clamp(0.0, 255.0);
                let i = if t.is_finite() { t as usize } else { 0 };
                if grey {
                    Color32::from_gray(i as u8)
                } else {
                    self.lut[i]
                }
            })
            .collect();
        self.shown_range = (a, b);
        self.shown_min_max = (lo, hi);
        let image = ColorImage::new([n, n], pixels);
        match &mut self.texture {
            Some(t) => t.set(image, TextureOptions::NEAREST),
            None => {
                self.texture = Some(ctx.load_texture("field", image, TextureOptions::NEAREST));
            }
        }
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("RDDemo");
        ui.label("Reaction\u{2013}diffusion on a 2D grid");
        ui.separator();

        // Model and preset.
        let mut model = self.s.model;
        egui::ComboBox::from_label("Model")
            .selected_text(with_model!(model, m => m.name()))
            .show_ui(ui, |ui| {
                for k in ModelKind::ALL {
                    ui.selectable_value(&mut model, k, with_model!(k, m => m.name()));
                }
            });
        if model != self.s.model {
            self.apply_preset(model, 0);
        }
        let presets = with_model!(self.s.model, m => m.presets());
        let mut preset = self.s.preset;
        egui::ComboBox::from_label("Preset")
            .selected_text(presets[preset].name)
            .show_ui(ui, |ui| {
                for (i, p) in presets.iter().enumerate() {
                    ui.selectable_value(&mut preset, i, p.name);
                }
            });
        if preset != self.s.preset {
            self.apply_preset(self.s.model, preset);
        }

        ui.horizontal(|ui| {
            let label = if self.running { "\u{23f8} Pause" } else { "\u{25b6} Play" };
            if ui.button(label).on_hover_text("Space").clicked() {
                self.toggle_run();
            }
            if ui.button("Step").on_hover_text("S").clicked() {
                self.step_once();
            }
            if ui.button("Reset").on_hover_text("R: restart the preset with the current parameters").clicked() {
                self.reset();
            }
        });
        if ui.button("Seed noise").on_hover_text("Add uniform noise \u{b1}0.01 to u and v").clicked() {
            self.grid.add_noise(&mut self.rng, 0.01);
        }

        ui.separator();
        ui.strong("Parameters");
        let specs = with_model!(self.s.model, m => m.params());
        let pp = self.painted_index();
        for (i, spec) in specs.iter().enumerate() {
            let mut x = self.s.params[i];
            let text = if i == pp {
                format!("{} (map)", spec.name)
            } else {
                spec.name.to_owned()
            };
            let r = ui.add(
                egui::Slider::new(&mut x, spec.min..=spec.max)
                    .logarithmic(true)
                    .max_decimals(4)
                    .text(text),
            );
            if r.changed() && x.is_finite() {
                self.s.params[i] = x;
                if i == pp {
                    self.grid.set_map_background(x);
                }
            }
        }
        ui.add(
            egui::Slider::new(&mut self.s.du, 0.01..=100.0)
                .logarithmic(true)
                .max_decimals(3)
                .text("D_u"),
        );
        let derived = with_model!(self.s.model, m => m.derived_dv(&self.s.params, self.s.du));
        match (derived, self.s.model) {
            (Some(dv), ModelKind::LengyelEpstein) => {
                ui.label(format!("D_v = \u{3c3} d D_u = {dv:.4}"));
            }
            (Some(dv), _) => {
                ui.label(format!("D_v = {dv} (v does not diffuse)"));
            }
            (None, _) => {
                ui.add(
                    egui::Slider::new(&mut self.s.dv, 0.01..=1000.0)
                        .logarithmic(true)
                        .max_decimals(3)
                        .text("D_v"),
                );
            }
        }
        if ui.button("Clear map").on_hover_text("Return every cell of the map to the slider value").clicked() {
            let bg = self.s.params[pp];
            self.grid.clear_map(bg);
        }

        ui.separator();
        ui.strong("Speed");
        let def = with_model!(self.s.model, m => m.default_speed());
        ui.add(
            egui::Slider::new(&mut self.s.speed, def / 100.0..=def * 100.0)
                .logarithmic(true)
                .max_decimals(3)
                .text("time / s"),
        );
        ui.label(format!("\u{394}t = {}", self.dt()));

        ui.separator();
        ui.strong("Brush");
        let map_name = specs[pp].name;
        let target_name = |t: BrushTarget| match t {
            BrushTarget::U => "u".to_owned(),
            BrushTarget::V => "v".to_owned(),
            BrushTarget::Map => format!("{map_name} map"),
            BrushTarget::Probe => "Probe".to_owned(),
        };
        egui::ComboBox::from_label("Target")
            .selected_text(target_name(self.brush_target))
            .show_ui(ui, |ui| {
                for t in [BrushTarget::U, BrushTarget::V, BrushTarget::Map, BrushTarget::Probe] {
                    ui.selectable_value(&mut self.brush_target, t, target_name(t));
                }
            });
        ui.add(egui::Slider::new(&mut self.brush_radius, 2.0..=30.0).text("radius (cells)"));
        let idx = match self.brush_target {
            BrushTarget::U => Some(0),
            BrushTarget::V => Some(1),
            BrushTarget::Map => Some(2),
            BrushTarget::Probe => None,
        };
        if let Some(idx) = idx {
            let drag_speed = (self.brush_values[idx].abs() * 0.01).max(0.001);
            let val = &mut self.brush_values[idx];
            let range = if idx == 2 {
                specs[pp].min..=specs[pp].max
            } else {
                0.0..=1e4
            };
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(val)
                        .speed(drag_speed)
                        .range(range)
                        .max_decimals(4),
                );
                ui.label("value");
            });
            ui.small("Left drag: paint value. Right or Shift drag: paint background.");
        } else {
            ui.small("Click the canvas to place the probe.");
        }

        ui.separator();
        ui.strong("Display");
        let mut boundary = self.s.boundary;
        egui::ComboBox::from_label("Boundary")
            .selected_text(boundary.label())
            .show_ui(ui, |ui| {
                for b in Boundary::ALL {
                    ui.selectable_value(&mut boundary, b, b.label());
                }
            });
        if boundary != self.s.boundary {
            self.s.boundary = boundary;
            self.grid.boundary = boundary;
        }
        let mut n = self.s.n;
        egui::ComboBox::from_label("Grid N")
            .selected_text(format!("{n} \u{d7} {n}"))
            .show_ui(ui, |ui| {
                for size in GRID_SIZES {
                    ui.selectable_value(&mut n, size, format!("{size} \u{d7} {size}"));
                }
            });
        if n != self.s.n {
            self.s.n = n;
            self.probe = (n / 2, n / 2);
            self.reset();
        }
        let view_name = |v: View| match v {
            View::U => "u".to_owned(),
            View::V => "v".to_owned(),
            View::Map => format!("{map_name} map"),
        };
        egui::ComboBox::from_label("View")
            .selected_text(view_name(self.s.view))
            .show_ui(ui, |ui| {
                for v in View::ALL {
                    ui.selectable_value(&mut self.s.view, v, view_name(v));
                }
            });
        ui.checkbox(&mut self.auto_scale, "Auto scale");

        ui.separator();
        if ui.button("\u{1f517} Copy link").on_hover_text("Copy a link to this setup").clicked() {
            self.copy_link(ui.ctx());
        }
    }

    fn toggle_run(&mut self) {
        if self.unstable {
            return;
        }
        self.running = !self.running;
    }

    fn step_once(&mut self) {
        self.running = false;
        let dt = self.dt() as f64;
        let target = (self.s.speed as f64 / 30.0).max(dt);
        self.simulate(target);
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let avail = ui.available_size();
        let bar_w = 70.0;
        // Capped so each cell stays small enough that the grid does not look blocky.
        let side = (avail.x - bar_w).min(avail.y).clamp(50.0, MAX_CANVAS_PX);
        let n = self.grid.n;

        ui.horizontal(|ui| {
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(side), Sense::click_and_drag());
            let cell = side / n as f32;
            let to_grid = |p: Pos2| Pos2::new((p.x - rect.left()) / cell, (p.y - rect.top()) / cell);
            let to_cell = |g: Pos2| -> Option<(usize, usize)> {
                let (i, j) = (g.y.floor(), g.x.floor());
                (i >= 0.0 && j >= 0.0 && (i as usize) < n && (j as usize) < n)
                    .then_some((i as usize, j as usize))
            };

            // Input: painting and probe.
            let (primary, secondary, shift) =
                ui.input(|i| (i.pointer.primary_down(), i.pointer.secondary_down(), i.modifiers.shift));
            if resp.is_pointer_button_down_on()
                && let Some(pos) = resp.interact_pointer_pos()
            {
                let g = to_grid(pos);
                if self.brush_target == BrushTarget::Probe {
                    if primary && let Some(c) = to_cell(g) && c != self.probe {
                        self.probe = c;
                        self.probe_hist.clear();
                        self.record_probe();
                    }
                } else if primary || secondary {
                    self.paint_stroke(g, secondary || shift);
                }
            } else {
                self.last_stamp = None;
            }
            self.hover = resp.hover_pos().and_then(|p| to_cell(to_grid(p)));

            self.update_texture(ui.ctx());
            let painter = ui.painter_at(rect);
            if let Some(tex) = &self.texture {
                painter.image(
                    tex.id(),
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }

            // Probe marker.
            let pc = rect.min
                + Vec2::new((self.probe.1 as f32 + 0.5) * cell, (self.probe.0 as f32 + 0.5) * cell);
            let marker = Stroke::new(2.0, Color32::from_rgb(255, 80, 80));
            painter.circle_stroke(pc, 5.0, marker);
            painter.line_segment([pc - Vec2::X * 9.0, pc - Vec2::X * 5.0], marker);
            painter.line_segment([pc + Vec2::X * 5.0, pc + Vec2::X * 9.0], marker);
            painter.line_segment([pc - Vec2::Y * 9.0, pc - Vec2::Y * 5.0], marker);
            painter.line_segment([pc + Vec2::Y * 5.0, pc + Vec2::Y * 9.0], marker);

            // Brush outline (2 sigma = radius).
            if self.brush_target != BrushTarget::Probe
                && let Some(p) = resp.hover_pos()
            {
                painter.circle_stroke(p, self.brush_radius * cell, Stroke::new(1.0, Color32::WHITE));
                painter.circle_stroke(p, self.brush_radius * cell + 1.0, Stroke::new(1.0, Color32::BLACK));
            }

            if self.unstable {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Simulation became unstable \u{2013} reset or lower the speed",
                    egui::FontId::proportional(18.0),
                    Color32::from_rgb(255, 90, 90),
                );
            }

            self.colour_bar(ui, side);
        });
    }

    fn colour_bar(&self, ui: &mut egui::Ui, height: f32) {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(60.0, height), Sense::hover());
        let painter = ui.painter_at(rect);
        let bar = Rect::from_min_size(rect.min + Vec2::new(4.0, 8.0), Vec2::new(14.0, height - 16.0));
        let steps = 64;
        for s in 0..steps {
            let f0 = s as f32 / steps as f32;
            let f1 = (s + 1) as f32 / steps as f32;
            let i = ((1.0 - f0) * 255.0) as usize;
            let col = if self.s.view == View::Map {
                Color32::from_gray(i as u8)
            } else {
                self.lut[i]
            };
            let r = Rect::from_x_y_ranges(bar.x_range(), (bar.top() + f0 * bar.height())..=(bar.top() + f1 * bar.height()));
            painter.rect_filled(r, 0.0, col);
        }
        let text_col = ui.visuals().text_color();
        let font = egui::FontId::proportional(12.0);
        let (lo, hi) = self.shown_range;
        let mid = 0.5 * (lo + hi);
        for (y, v) in [(bar.top(), hi), (bar.center().y, mid), (bar.bottom(), lo)] {
            painter.text(
                Pos2::new(bar.right() + 4.0, y),
                egui::Align2::LEFT_CENTER,
                stability::sig3(v as f64),
                font.clone(),
                text_col,
            );
        }
    }

    fn right_panel(&mut self, ui: &mut egui::Ui) {
        ui.strong(format!("Probe at cell ({}, {})", self.probe.0, self.probe.1));
        let u: PlotPoints<'_> = self.probe_hist.iter().map(|p| [p[0], p[1]]).collect();
        let v: PlotPoints<'_> = self.probe_hist.iter().map(|p| [p[0], p[2]]).collect();
        Plot::new("probe")
            .height(180.0)
            .legend(Legend::default())
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .x_axis_label("t")
            .show(ui, |pu| {
                pu.line(Line::new("u", u));
                pu.line(Line::new("v", v));
            });

        ui.separator();
        self.stability_panel(ui);
    }

    fn stability_panel(&self, ui: &mut egui::Ui) {
        let pp = self.painted_index();
        let (painted, at_cell) = match self.hover {
            Some((i, j)) => (self.grid.map[i * self.grid.n + j], Some((i, j))),
            None => (self.s.params[pp], None),
        };
        let dv = self.effective_dv();
        let d = with_model!(self.s.model, m => stability::analyze(m, &self.s.params, painted, self.s.du, dv));
        let map_name = with_model!(self.s.model, m => m.params()[pp].name);

        ui.strong("Linear stability of the uniform state");
        match at_cell {
            Some((i, j)) => ui.label(format!("at cell ({i}, {j}), {map_name} = {}", stability::sig3(painted as f64))),
            None => ui.label(format!("at background {map_name} = {}", stability::sig3(painted as f64))),
        };

        let gmax = d.growth.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let scale = gmax.abs().max(d.growth[0].abs()).max(1e-3);
        let floor = -3.0 * scale;
        let pts: PlotPoints<'_> = d
            .ks
            .iter()
            .zip(&d.growth)
            .map(|(&k, &g)| [k, g.max(floor)])
            .collect();
        Plot::new("dispersion")
            .height(180.0)
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .x_axis_label("wavenumber k")
            .y_axis_label("growth rate")
            .include_x(0.0)
            .include_x(std::f64::consts::PI)
            .include_y(0.0)
            .show(ui, |pu| {
                pu.hline(HLine::new("zero", 0.0).color(Color32::GRAY));
                pu.line(Line::new("growth", pts));
                if matches!(d.verdict, stability::Verdict::Turing { .. }) {
                    pu.vline(VLine::new("k*", d.k_star).color(Color32::from_rgb(255, 140, 0)));
                }
            });

        let colour = match d.verdict {
            stability::Verdict::Stable => Color32::from_rgb(90, 170, 90),
            stability::Verdict::Turing { .. } => Color32::from_rgb(255, 140, 0),
            _ => Color32::from_rgb(220, 80, 80),
        };
        ui.label(egui::RichText::new(d.verdict.text()).color(colour).strong().size(15.0));
        ui.label(format!(
            "Steady state: u* = {}, v* = {}",
            stability::sig3(d.steady.0),
            stability::sig3(d.steady.1)
        ));
        ui.label(format!(
            "Eigenvalues at k = 0: {},  {}",
            stability::fmt_eig(d.eig0[0]),
            stability::fmt_eig(d.eig0[1])
        ));
        if let Some(note) = with_model!(self.s.model, m => m.note()) {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(note).italics());
        }
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(format!("t = {:.2}", self.t));
            ui.separator();
            ui.label(format!("{:.2} time units/s", self.sim_rate));
            ui.separator();
            ui.label(format!("{:.0} fps", self.fps));
            ui.separator();
            let (lo, hi) = self.shown_min_max;
            ui.label(format!(
                "{} min {}  max {}",
                self.s.view.key(),
                stability::sig3(lo as f64),
                stability::sig3(hi as f64)
            ));
            if let Some((i, j)) = self.hover {
                ui.separator();
                let v = self.shown_field()[i * self.grid.n + j];
                ui.label(format!("cell ({i}, {j}) = {}", stability::sig3(v as f64)));
            }
            if self.unstable {
                ui.separator();
                ui.colored_label(
                    Color32::from_rgb(255, 90, 90),
                    "Simulation became unstable \u{2013} reset or lower the speed",
                );
            }
            if let Some((msg, when)) = &self.message
                && when.elapsed().as_secs_f32() < 2.5
            {
                ui.separator();
                ui.label(msg);
            }
        });
    }
}

impl eframe::App for RdApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Timing.
        let now = Instant::now();
        let frame_dt = self
            .last_frame
            .map_or(1.0 / 60.0, |t| (now - t).as_secs_f64())
            .clamp(1e-4, 0.1);
        self.last_frame = Some(now);
        self.fps = 0.9 * self.fps + 0.1 / frame_dt;

        // Keyboard shortcuts, unless a text field has focus.
        if !ui.ctx().egui_wants_keyboard_input() {
            let (space, s, r) = ui.input(|i| {
                (
                    i.key_pressed(egui::Key::Space),
                    i.key_pressed(egui::Key::S),
                    i.key_pressed(egui::Key::R),
                )
            });
            if space {
                self.toggle_run();
            }
            if s {
                self.step_once();
            }
            if r {
                self.reset();
            }
        }

        // Simulate.
        if self.running && !self.unstable {
            let done = self.simulate(self.s.speed as f64 * frame_dt);
            self.sim_rate = 0.9 * self.sim_rate + 0.1 * done / frame_dt;
        } else {
            self.sim_rate *= 0.9;
        }

        egui::Panel::left("controls")
            .resizable(false)
            .exact_size(290.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.controls(ui));
            });
        egui::Panel::right("analysis")
            .resizable(true)
            .default_size(340.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.right_panel(ui));
            });
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::CentralPanel::default().show(ui, |ui| self.canvas(ui));

        if self.running || self.message.is_some() {
            ui.ctx().request_repaint();
        }
        if self
            .message
            .as_ref()
            .is_some_and(|(_, w)| w.elapsed().as_secs_f32() > 3.0)
        {
            self.message = None;
        }
    }
}

/// The URL fragment on the web; the first command-line argument natively.
fn initial_fragment() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.location().hash().ok())
            .unwrap_or_default()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::args().nth(1).unwrap_or_default()
    }
}

/// Viridis colour map (polynomial fit), 256 entries.
fn viridis_lut() -> [Color32; 256] {
    const C: [[f32; 3]; 7] = [
        [0.277_727_33, 0.005_407_344_5, 0.334_099_8],
        [0.105_093_04, 1.404_613_5, 1.384_590_2],
        [-0.330_861_83, 0.214_847_56, 0.095_095_16],
        [-4.634_230_5, -5.799_101, -19.332_441],
        [6.228_27, 14.179_933, 56.690_55],
        [4.776_385, -13.745_145, -65.353_03],
        [-5.435_456, 4.645_852_6, 26.312_435],
    ];
    let mut lut = [Color32::BLACK; 256];
    for (i, c) in lut.iter_mut().enumerate() {
        let t = i as f32 / 255.0;
        let mut rgb = [0u8; 3];
        for (ch, out) in rgb.iter_mut().enumerate() {
            let mut x = C[6][ch];
            for k in (0..6).rev() {
                x = C[k][ch] + t * x;
            }
            *out = (x.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
        *c = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    }
    lut
}
