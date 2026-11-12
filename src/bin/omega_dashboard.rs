//! Omega Kernel Dashboard — desktop operator UI.
//!
//! ```bash
//! cargo run --release --features gui --bin omega-dashboard
//! ```

#![windows_subsystem = "windows"]

use eframe::egui::{self, Color32, RichText};
use omega_cognitive_kernel::api::Kernel;
use omega_cognitive_kernel::audit::AuditLog;
use omega_cognitive_kernel::config::KernelConfig;
use std::path::PathBuf;

fn setup_cjk_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let candidates = [
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\msyhbd.ttc",
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\simsun.ttc",
        r"C:\Windows\Fonts\msjh.ttc",
        r"C:\Windows\Fonts\seguiemj.ttf",
        r"C:\Windows\Fonts\segoeui.ttf",
        "/System/Library/Fonts/PingFang.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    ];
    let mut loaded = false;
    for path in candidates {
        if let Ok(data) = std::fs::read(path) {
            let name = if path.contains("seguiemj") || path.contains("segoeui") {
                "emoji_font"
            } else {
                "cjk_font"
            };
            fonts
                .font_data
                .insert(name.to_owned(), egui::FontData::from_owned(data));
            if name == "cjk_font" {
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "cjk_font".to_owned());
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "cjk_font".to_owned());
                loaded = true;
            } else {
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .push("emoji_font".to_owned());
            }
        }
    }
    if loaded {
        ctx.set_fonts(fonts);
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Dashboard,
    Run,
    Audit,
    Strategies,
    Settings,
}

struct DashboardApp {
    tab: Tab,
    kernel: Kernel,
    goal: String,
    steps_to_run: u32,
    last_log: Vec<String>,
    status_msg: String,
    error: Option<String>,
    config_path: Option<PathBuf>,
    self_modify: bool,
    dump_trajectory: bool,
}

impl DashboardApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_cjk_fonts(&cc.egui_ctx);
        let mut visuals = egui::Visuals::dark();
        visuals.window_rounding = 8.0.into();
        visuals.panel_fill = Color32::from_rgb(16, 18, 24);
        cc.egui_ctx.set_visuals(visuals);

        let state = std::env::temp_dir().join("omega-dashboard-state.json");
        let mut kernel = Kernel::with_state_path(&state);
        if state.exists() {
            let _ = kernel.load();
        }
        kernel.autosave = true;
        kernel.dump_trajectory = true;
        kernel.inner.l1.self_modify_enabled = false;

        Self {
            tab: Tab::Dashboard,
            kernel,
            goal: "maintain coherent self-model while acting in world".into(),
            steps_to_run: 8,
            last_log: Vec::new(),
            status_msg: "就绪 — 可运行步骤或查看状态".into(),
            error: None,
            config_path: None,
            self_modify: false,
            dump_trajectory: true,
        }
    }

    fn refresh_flags_from_kernel(&mut self) {
        self.self_modify = self.kernel.inner.l1.self_modify_enabled;
        self.dump_trajectory = self.kernel.dump_trajectory;
    }

    fn apply_flags_to_kernel(&mut self) {
        self.kernel.inner.l1.self_modify_enabled = self.self_modify;
        self.kernel.dump_trajectory = self.dump_trajectory;
    }

    fn run_steps(&mut self) {
        self.apply_flags_to_kernel();
        self.last_log.clear();
        self.error = None;
        let goal = self.goal.clone();
        let n = self.steps_to_run;
        let mut ok = 0u32;
        for i in 1..=n {
            match self.kernel.step(&goal) {
                Ok(r) => {
                    ok += 1;
                    let act = r
                        .trajectory
                        .actions
                        .first()
                        .map(|a| a.name.as_str())
                        .unwrap_or("-");
                    let line = format!(
                        "step {i:>2}  act={act:<10} patched={}  damp={:.2}  err={:.3}  fp={}",
                        r.l1.patched,
                        self.kernel.inner.l0.memory.cognitive_effect.error_damping,
                        self.kernel.inner.l0.memory.cas.avg_prediction_error,
                        r.l2.in_fixed_point
                    );
                    self.last_log.push(line);
                }
