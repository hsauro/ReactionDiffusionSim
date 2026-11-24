//! The About window: what the models are and how to use the app.

use egui::{CollapsingHeader, RichText};

use crate::sim::ModelKind;

const REPO: &str = "https://github.com/hsauro/ReactionDiffusionSim";

pub fn show(ctx: &egui::Context, open: &mut bool, current: ModelKind) {
    egui::Window::new("About RDDemo")
        .open(open)
        .default_width(600.0)
        .default_height(560.0)
        .vscroll(true)
        .resizable(true)
        .show(ctx, |ui| contents(ui, current));
}

fn eq(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).monospace());
}

fn section(ui: &mut egui::Ui, title: &str, open: bool, body: impl FnOnce(&mut egui::Ui)) {
    CollapsingHeader::new(RichText::new(title).strong())
        .default_open(open)
        .show(ui, body);
}

fn contents(ui: &mut egui::Ui, current: ModelKind) {
    ui.label(
        "RDDemo simulates two chemical species, u and v, that react with each other and \
         diffuse across a square grid. When the inhibitor spreads faster than the activator, \
         a uniform mixture can become unstable and break up into spots, stripes or mazes \
         (Turing patterns). Excitable media instead carry travelling waves and spirals.",
    );
    ui.add_space(4.0);
    ui.label(
        "Each model has one parameter that can vary from cell to cell (the parameter map). \
         Paint it to confine a pattern to a region. The right-hand panel shows what linear \
         stability theory predicts for the uniform state, so you can compare theory with \
         the canvas.",
    );
    ui.add_space(8.0);

    section(ui, "Brusselator", current == ModelKind::Brusselator, |ui| {
        eq(ui, "du/dt = a - (b+1) u + u\u{b2} v + D_u \u{2207}\u{b2}u");
        eq(ui, "dv/dt = b u - u\u{b2} v       + D_v \u{2207}\u{b2}v");
        ui.label(
            "A hypothetical autocatalytic reaction scheme proposed by Prigogine and Lefever \
             (1968) in Brussels. The step 2u + v \u{2192} 3u makes u an autocatalytic \
             activator, and v is used up by it.",
        );
        ui.label(
            "Uniform steady state: u* = a, v* = b/a. Without diffusion it oscillates \
             (a Hopf bifurcation) when b > 1 + a\u{b2}. With diffusion, Turing patterns \
             appear when b > (1 + a\u{221a}(D_u/D_v))\u{b2}, which needs D_v well above D_u.",
        );
        ui.label(
            "Try: Spots and Stripes (Turing patterns); Oscillating (equal diffusion, \
             b above the Hopf threshold); W prepattern (b is above the Turing threshold \
             only inside the W, so spots form only there). Paint b = 7 elsewhere to grow \
             new spots.",
        );
    });

    section(ui, "Gray\u{2013}Scott", current == ModelKind::GrayScott, |ui| {
        eq(ui, "du/dt = -u v\u{b2} + F (1 - u) + D_u \u{2207}\u{b2}u");
        eq(ui, "dv/dt =  u v\u{b2} - (F + k) v + D_v \u{2207}\u{b2}v");
        ui.label(
            "Cubic autocatalysis, u + 2v \u{2192} 3v, in a continuously fed reactor \
             (Gray and Scott, 1983). F is the feed rate of u, and k is the rate at which v \
             is removed. Pearson (1993) mapped the remarkable range of patterns it produces \
             as F and k vary.",
        );
        ui.label(
            "The uniform state u = 1, v = 0 is always linearly stable, so these patterns \
             are not Turing patterns: they need a large enough seed (the square at the \
             centre) and then spread.",
        );
        ui.label(
            "Try: Dividing spots (spots grow and split), Coral, Maze, Unsteady spots. \
             Paint v = 0.25 to start new structures, or paint the F map to change the \
             behaviour region by region.",
        );
    });

    section(ui, "Lengyel\u{2013}Epstein (CIMA reaction)", current == ModelKind::LengyelEpstein, |ui| {
        eq(ui, "du/dt = a - u - 4uv/(1+u\u{b2}) + D_u \u{2207}\u{b2}u");
        eq(ui, "dv/dt = \u{3c3} [ b (u - uv/(1+u\u{b2})) + d D_u \u{2207}\u{b2}v ]");
        ui.label(
            "A two-variable model (Lengyel and Epstein, 1991) of the chlorine dioxide\u{2013}\
             iodine\u{2013}malonic acid (CIMA) reaction, in which Turing patterns were first \
             seen in a real chemical system (Castets, Dulos, Boissonade and De Kepper, 1990; \
             Ouyang and Swinney, 1991). u is iodide (the activator) and v is chlorite \
             (the inhibitor). Starch in the gel binds iodide and slows it down; \u{3c3} \
             captures this, so v effectively diffuses \u{3c3}\u{b7}d times faster than u.",
        );
        ui.label(
            "Uniform steady state: u* = a/5, v* = 1 + a\u{b2}/25. For a = 12, \u{3c3} = 50, \
             d = 1: oscillation for b \u{2264} 0.10, Turing patterns for b from about 0.15 \
             to 0.35, and a stable uniform state for b \u{2265} 0.40.",
        );
        ui.label("Try: Maze (b = 0.2), Spots (b = 0.3), Stable (b = 0.5).");
    });

    section(ui, "Barkley (excitable medium)", current == ModelKind::Barkley, |ui| {
        eq(ui, "du/dt = (1/\u{3b5}) u (1 - u) (u - (v + b)/a) + D \u{2207}\u{b2}u");
        eq(ui, "dv/dt = u - v");
        ui.label(
            "A fast, simple model of an excitable medium (Barkley, 1991). u is a fast \
             excitation variable and v a slow recovery variable; only u diffuses. The rest \
             state is stable, but a disturbance that pushes u above the threshold \
             (v + b)/a sets off a wave, after which the medium is briefly refractory. \
             The same behaviour gives the waves and spirals of the Belousov\u{2013}\
             Zhabotinsky reaction and of electrical activity in heart muscle.",
        );
        ui.label(
            "Try: Spiral (a broken wave front curls into a pair of rotating spirals); \
             Target (one expanding ring). Drag the brush (u = 1) across a travelling wave \
             to break it and start new spirals. Smaller \u{3b5} gives sharper fronts.",
        );
    });

    section(ui, "The stability panel", false, |ui| {
        ui.label(
            "For the current parameters the panel finds the uniform steady state, computes \
             the Jacobian J of the reaction terms there (by central differences), and for \
             each wavenumber k from 0 to \u{3c0} finds the largest real part of the \
             eigenvalues of J - q(k) diag(D_u, D_v), with q(k) = 4 sin\u{b2}(k/2). This is \
             the grid's own Laplacian, so the prediction matches the simulation exactly.",
        );
        ui.label(
            "Growth above zero at k = 0 means the well-mixed system is unstable (it \
             oscillates if the eigenvalues are complex). Growth below zero at k = 0 but \
             above zero at some k > 0 is a Turing instability; the fastest-growing \
             wavelength 2\u{3c0}/k* sets the initial pattern spacing. Hover over the \
             canvas to analyse the parameter value at that cell.",
        );
    });

    section(ui, "Using the controls", false, |ui| {
        ui.label("\u{2022} Space: play/pause.  S: step.  R: reset (restart with the current parameters).");
        ui.label("\u{2022} Choose a preset again to restore its parameter values.");
        ui.label(
            "\u{2022} Brush target u, v or the parameter map: left drag paints the brush value, \
             right drag (or Shift + drag) paints back toward the background value.",
        );
        ui.label("\u{2022} Brush target Probe: click a cell to plot u and v there over time.");
        ui.label(
            "\u{2022} The parameter-map slider sets the background value; cells you have \
             painted keep their values. Clear map resets every cell.",
        );
        ui.label("\u{2022} Copy link copies a URL that reopens this model, preset and parameters.");
    });

    section(ui, "Numerics", false, |ui| {
        ui.label(
            "Forward Euler in time with a five-point Laplacian on a grid of unit spacing. \
             The time step is the smaller of the model's limit and 0.2 / max(D_u, D_v); \
             the speed control changes how many steps run per frame, never the step size. \
             In the Brusselator the u\u{b2}v transfer between v and u is taken implicitly, \
             which keeps strong oscillations from overshooting.",
        );
    });

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Source:");
        ui.hyperlink(REPO);
    });
    ui.small("Built with Rust and egui; runs in the browser as WebAssembly.");
}
