//! Omega Kernel Dashboard — desktop operator UI.
//!
//! ```bash
//! cargo run --release --features gui --bin omega-dashboard
//! ```

#![windows_subsystem = "windows"]

use eframe::egui::{self, Color32, RichText};
use omega_cognitive_kernel::api::{Kernel, TrajectoryReplaySummary};
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
    Replay,
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
    replay_tail: usize,
    replay_summary: Option<TrajectoryReplaySummary>,
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
            replay_tail: 20,
            replay_summary: None,
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
                Err(e) => {
                    self.error = Some(format!("step {i} failed: {e}"));
                    break;
                }
            }
        }
        let _ = self.kernel.save();
        self.status_msg = format!("本批完成 {ok}/{n} 步");
        self.refresh_flags_from_kernel();
    }

    fn load_replay(&mut self) {
        match self.kernel.replay_trajectory(self.replay_tail) {
            Ok(sum) => {
                self.status_msg = format!("回放 {} 行", sum.rows);
                self.replay_summary = Some(sum);
                self.error = None;
            }
            Err(e) => {
                self.replay_summary = None;
                self.error = Some(format!("回放失败: {e}"));
            }
        }
    }

    fn load_state_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("打开 state.json")
            .add_filter("JSON", &["json"])
            .pick_file()
        {
            match Kernel::open(path.to_str()) {
                Ok(mut k) => {
                    k.autosave = true;
                    self.kernel = k;
                    self.refresh_flags_from_kernel();
                    self.replay_summary = None;
                    self.status_msg = format!("已加载 {}", path.display());
                    self.error = None;
                }
                Err(e) => self.error = Some(format!("加载失败: {e}")),
            }
        }
    }

    fn load_config_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("打开 omega.toml")
            .add_filter("TOML", &["toml"])
            .pick_file()
        {
            match KernelConfig::load(&path) {
                Ok(cfg) => {
                    self.kernel.apply_config(&cfg);
                    self.config_path = Some(path.clone());
                    self.refresh_flags_from_kernel();
                    self.status_msg = format!("已应用配置 {}", path.display());
                    self.error = None;
                }
                Err(e) => self.error = Some(format!("配置失败: {e}")),
            }
        }
    }
}

impl eframe::App for DashboardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading(RichText::new("Omega Kernel").strong().size(22.0));
                ui.label(RichText::new("认知运行时控制台").weak().size(14.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(&self.status_msg)
                            .small()
                            .color(Color32::from_rgb(140, 180, 255)),
                    );
                });
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Dashboard, "📊 仪表盘");
                ui.selectable_value(&mut self.tab, Tab::Run, "▶ 运行");
                ui.selectable_value(&mut self.tab, Tab::Audit, "📜 审计");
                ui.selectable_value(&mut self.tab, Tab::Replay, "🔁 回放");
                ui.selectable_value(&mut self.tab, Tab::Strategies, "🎯 策略");
                ui.selectable_value(&mut self.tab, Tab::Settings, "⚙️ 设置");
            });
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("state: {}", self.kernel.state_path.display()));
                ui.separator();
                let st = self.kernel.status();
                ui.label(format!(
                    "schema={}  steps={}  err={:.3}  self_modify={}",
                    st.schema_version,
                    st.total_steps,
                    st.avg_prediction_error,
                    st.self_modify_enabled
                ));
                if let Some(ref e) = self.error {
                    ui.colored_label(Color32::from_rgb(255, 120, 120), e);
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Dashboard => self.ui_dashboard(ui),
            Tab::Run => self.ui_run(ui),
            Tab::Audit => self.ui_audit(ui),
            Tab::Replay => self.ui_replay(ui),
            Tab::Strategies => self.ui_strategies(ui),
            Tab::Settings => self.ui_settings(ui),
        });
    }
}

impl DashboardApp {
    fn stat_card(&self, ui: &mut egui::Ui, title: &str, value: &str, accent: Color32) {
        egui::Frame::none()
            .fill(Color32::from_rgb(28, 30, 38))
            .rounding(8.0)
            .inner_margin(14.0)
            .show(ui, |ui| {
                ui.set_min_width(120.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(title).small().weak());
                    ui.label(RichText::new(value).size(22.0).color(accent).strong());
                });
            });
    }

    fn ui_dashboard(&mut self, ui: &mut egui::Ui) {
        let st = self.kernel.status();
        ui.heading("仪表盘");
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            self.stat_card(
                ui,
                "Schema",
                &st.schema_version.to_string(),
                Color32::from_rgb(160, 200, 255),
            );
            ui.add_space(10.0);
            self.stat_card(
                ui,
                "总步数",
                &st.total_steps.to_string(),
                Color32::from_rgb(80, 160, 255),
            );
            ui.add_space(10.0);
            self.stat_card(
                ui,
                "预测误差",
                &format!("{:.3}", st.avg_prediction_error),
                Color32::from_rgb(255, 180, 80),
            );
            ui.add_space(10.0);
            self.stat_card(
                ui,
                "阻尼",
                &format!("{:.3}", st.error_damping),
                Color32::from_rgb(100, 200, 140),
            );
            ui.add_space(10.0);
            self.stat_card(
                ui,
                "策略数",
                &st.strategy_count.to_string(),
                Color32::from_rgb(180, 140, 255),
            );
            ui.add_space(10.0);
            self.stat_card(
                ui,
                "自修改",
                if st.self_modify_enabled { "开" } else { "关" },
                if st.self_modify_enabled {
                    Color32::from_rgb(255, 120, 100)
                } else {
                    Color32::from_rgb(100, 200, 140)
                },
            );
        });

        ui.add_space(16.0);
        ui.heading("L2 / 固定点");
        ui.label(format!(
            "in_fp={}   dwell={}   align={:.3}   max_risk={:.2}",
            st.in_fixed_point, st.dwell_steps, st.last_alignment_score, st.max_risk
        ));
        ui.label(format!(
            "explore_bonus={:.3}   risk_tolerance={:.3}   modules={}   cases={}",
            st.explore_bonus, st.risk_tolerance, st.active_modules, st.case_count
        ));

        ui.add_space(16.0);
        ui.horizontal(|ui| {
            if ui.button("刷新状态").clicked() {
                self.refresh_flags_from_kernel();
                self.status_msg = "已刷新".into();
            }
            if ui.button("保存 state").clicked() {
                match self.kernel.save() {
                    Ok(()) => self.status_msg = "已保存".into(),
                    Err(e) => self.error = Some(format!("{e}")),
                }
            }
            if ui.button("去运行 →").clicked() {
                self.tab = Tab::Run;
            }
            if ui.button("去回放 →").clicked() {
                self.tab = Tab::Replay;
            }
        });
    }

    fn ui_run(&mut self, ui: &mut egui::Ui) {
        ui.heading("运行循环");
        ui.label(RichText::new("默认 self_modify=关。需要演化规则时请到「设置」打开。").weak());
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("目标 goal:");
            ui.add(
                egui::TextEdit::singleline(&mut self.goal)
                    .desired_width(520.0)
                    .hint_text("high-level goal"),
            );
        });
        ui.horizontal(|ui| {
            ui.label("步数:");
            ui.add(egui::DragValue::new(&mut self.steps_to_run).range(1..=200));
            if ui
                .button(RichText::new(format!("▶ 运行 {} 步", self.steps_to_run)).strong())
                .clicked()
            {
                self.run_steps();
            }
        });

        ui.add_space(12.0);
        ui.heading("本批输出");
        egui::ScrollArea::vertical()
            .max_height(360.0)
            .show(ui, |ui| {
                if self.last_log.is_empty() {
                    ui.label("(尚未运行)");
                } else {
                    for line in &self.last_log {
                        ui.monospace(line);
                    }
                }
            });
    }

    fn ui_audit(&mut self, ui: &mut egui::Ui) {
        ui.heading("审计");
        let log = AuditLog::beside_state(&self.kernel.state_path);
        match log.summarize() {
            Ok(s) => {
                ui.label(format!("log: {}", log.path.display()));
                ui.label(format!(
                    "events={}  steps={}  patches_ok={}  patches_rej={}  ckpts={}  rollbacks={}",
                    s.total_events,
                    s.steps,
                    s.patches_accepted,
                    s.patches_rejected,
                    s.checkpoints,
                    s.rollbacks
                ));
                if !s.action_counts.is_empty() {
                    ui.add_space(8.0);
                    ui.label("动作分布:");
                    for (name, n) in &s.action_counts {
                        ui.label(format!("  {name:<12} {n}"));
                    }
                }
            }
            Err(e) => {
                ui.colored_label(Color32::from_rgb(255, 120, 120), format!("{e}"));
            }
        }

        ui.add_space(12.0);
        ui.heading("最近审计事件");
        match self.kernel.audit_tail(30) {
            Ok(rows) => {
                egui::ScrollArea::vertical()
                    .max_height(280.0)
                    .show(ui, |ui| {
                        if rows.is_empty() {
                            ui.label("(空)");
                        }
                        for r in rows.iter().rev() {
                            ui.monospace(format!("{}  {:?}", r.ts.to_rfc3339(), r.event));
                        }
                    });
            }
            Err(e) => {
                ui.colored_label(Color32::from_rgb(255, 120, 120), format!("{e}"));
            }
        }
    }

    fn ui_replay(&mut self, ui: &mut egui::Ui) {
        ui.heading("轨迹回放");
        ui.label(
            RichText::new("离线汇总 trajectory.jsonl，不重跑世界循环。与 CLI `omega-kernel replay` 同源。")
                .weak(),
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("tail:");
            ui.add(egui::DragValue::new(&mut self.replay_tail).range(5..=200));
            if ui.button(RichText::new("加载回放").strong()).clicked() {
                self.load_replay();
            }
        });

        ui.add_space(12.0);
        match &self.replay_summary {
            None => {
                ui.label("(尚未加载 — 先运行若干步并确保 dump_trajectory 开启)");
            }
            Some(sum) => {
                ui.label(format!("path: {}", sum.path));
                ui.label(format!(
                    "rows={}  patches={}",
                    sum.rows, sum.patches
                ));
                if let (Some(a), Some(b)) = (sum.first_avg_err, sum.last_avg_err) {
                    ui.label(format!("avg_err: {a:.4} → {b:.4}"));
                }
                if !sum.action_counts.is_empty() {
                    ui.add_space(8.0);
                    ui.label("动作分布:");
                    for (name, n) in &sum.action_counts {
                        ui.label(format!("  {name:<12} {n}"));
                    }
                }
                ui.add_space(10.0);
                ui.heading("尾部步骤");
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for r in &sum.tail {
                            let err = r
                                .avg_err
                                .map(|e| format!("{e:.3}"))
                                .unwrap_or_else(|| "-".into());
                            ui.monospace(format!(
                                "step {:>4}  act={:<10}  patched={:<5}  avg_err={}",
                                r.step, r.action, r.patched, err
                            ));
                        }
                    });
            }
        }
    }

    fn ui_strategies(&mut self, ui: &mut egui::Ui) {
        ui.heading("策略记忆");
        let strategies = self.kernel.inner.l0.memory.strategy.retrieve("", 50);
        if strategies.is_empty() {
            ui.label("(尚无策略 — 先运行若干步)");
            return;
        }
        ui.label(format!("共 {} 条（按权重排序）", strategies.len()));
        ui.add_space(6.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("名称").strong().monospace());
                ui.add_space(40.0);
                ui.label(RichText::new("权重").strong());
                ui.add_space(16.0);
                ui.label(RichText::new("成功").strong());
                ui.add_space(8.0);
                ui.label(RichText::new("失败").strong());
                ui.add_space(8.0);
                ui.label(RichText::new("空闲步").strong());
            });
            ui.separator();
            for s in strategies {
                ui.horizontal(|ui| {
                    ui.monospace(format!("{:<12}", s.name));
                    ui.label(format!("{:.3}", s.weight));
                    ui.label(format!("{}", s.success_count));
                    ui.label(format!("{}", s.fail_count));
                    ui.label(format!("{}", s.idle_steps));
                });
            }
        });
    }

    fn ui_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("设置");
        ui.add_space(8.0);

        ui.checkbox(
            &mut self.self_modify,
            "启用自修改 (self_modify_enabled) — 默认关闭",
        );
        ui.checkbox(&mut self.dump_trajectory, "落盘轨迹 trajectory.jsonl");
        if ui.button("应用开关到 Kernel").clicked() {
            self.apply_flags_to_kernel();
            self.status_msg = format!(
                "已应用 self_modify={} dump={}",
                self.self_modify, self.dump_trajectory
            );
        }

        ui.add_space(12.0);
        ui.label(format!("当前 state: {}", self.kernel.state_path.display()));
        ui.horizontal(|ui| {
            if ui.button("打开 state…").clicked() {
                self.load_state_dialog();
            }
            if ui.button("加载 omega.toml…").clicked() {
                self.load_config_dialog();
            }
            if ui.button("另存 state…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("保存 state")
                    .set_file_name("state.json")
                    .save_file()
                {
                    self.kernel.state_path = path.clone();
                    match self.kernel.save() {
                        Ok(()) => self.status_msg = format!("已保存到 {}", path.display()),
                        Err(e) => self.error = Some(format!("{e}")),
                    }
                }
            }
        });

        if let Some(ref p) = self.config_path {
            ui.label(format!("上次配置: {}", p.display()));
        }

        ui.add_space(16.0);
        ui.separator();
        ui.label(
            RichText::new(
                "说明：\n\
• Dashboard 只操作本地 Kernel，不上传数据。\n\
• 自修改关闭时 L1 不会提交规则补丁。\n\
• 轨迹与审计写在 state 同目录；回放页读取 trajectory.jsonl。\n\
• 完整产品化前请保持 self_modify=false。",
            )
            .small(),
        );
        ui.label(
            RichText::new("Omega Dashboard v0.5.1  ·  MIT")
                .weak()
                .small(),
        );
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1080.0, 700.0])
            .with_min_inner_size([800.0, 520.0])
            .with_title("Omega Kernel — 认知运行时控制台"),
        ..Default::default()
    };
    eframe::run_native(
        "Omega Dashboard",
        options,
        Box::new(|cc| Ok(Box::new(DashboardApp::new(cc)))),
    )
}
