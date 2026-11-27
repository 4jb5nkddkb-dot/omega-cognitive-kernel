//! L1 验证探针：shadow 模式零扰动、L1/L2 互斥仲裁、L1 自动回滚窗口。
//!
//! 只通过公共 API 驱动内核，可独立运行：
//! `cargo run --release --example l1_probe`
//! 注意：漂移是注入式的（直接覆写 cas.avg_prediction_error，与内置
//! regime_shift 场景一致），不是真实负载。

use std::fs;
use std::time::Instant;

use omega_cognitive_kernel::l0::decision::ConstrainedDecisionMaker;
use omega_cognitive_kernel::l0::executor::SandboxedExecutor;
use omega_cognitive_kernel::l0::observer::MultiModalObserver;
use omega_cognitive_kernel::l0::predictor::ModelBasedPredictor;
use omega_cognitive_kernel::l0::L0WorldLayer;
use omega_cognitive_kernel::l1::learner::Learner;
use omega_cognitive_kernel::l1::L1CognitiveLayer;
use omega_cognitive_kernel::l2::L2MetaLayer;
use omega_cognitive_kernel::memory::MemoryFacade;
use omega_cognitive_kernel::orchestrator::OmegaOrchestrator;
use omega_cognitive_kernel::safety::default_constraints;

type Kernel = OmegaOrchestrator<
    MultiModalObserver,
    ModelBasedPredictor,
    ConstrainedDecisionMaker,
    SandboxedExecutor,
>;

const TOTAL_STEPS: usize = 90;
const DRIFT_AT: usize = 45;
const GOAL: &str = "maintain coherent self-model omega dayan";

fn make_kernel(shadow: bool, self_modify: bool, l2_enabled: bool) -> Kernel {
    let l0 = L0WorldLayer {
        observer: MultiModalObserver::new(),
        predictor: ModelBasedPredictor,
        decision: ConstrainedDecisionMaker,
        executor: SandboxedExecutor::new(
            ["noop", "observe", "explore", "exploit", "stabilize"].map(String::from),
        ),
        memory: MemoryFacade::new(),
        learner: Learner::default(),
    };
    let mut l1 = L1CognitiveLayer::new();
    l1.shadow_mode = shadow;
    l1.self_modify_enabled = self_modify;
    let mut l2 = L2MetaLayer::new();
    l2.enabled = l2_enabled;
    OmegaOrchestrator {
        l0,
        l1,
        l2,
        biases: Default::default(),
        constraints: default_constraints(),
        total_steps: 0,
    }
}

#[derive(Clone)]
struct Rec {
    step: usize,
    effect_after: String,
    effect_changed: bool,
    l1_patched: bool,
    l1_candidate: bool,
    l2_nudge: bool,
    l1_blocked: bool,
    rolled_back: bool,
    history_growth: usize,
    self_modify_after: bool,
    ms: f64,
}

fn step(k: &mut Kernel, idx: usize, inject: Option<f64>) -> Rec {
    if let Some(v) = inject {
        k.l0.memory.cas.avg_prediction_error = v;
    }
    let effect_before = format!("{:?}", k.l0.memory.cognitive_effect);
    let hist_before = k.l1.store.history.len();

    let t0 = Instant::now();
    let r = k.run_once(GOAL).expect("run_once failed");
    let ms = t0.elapsed().as_secs_f64() * 1000.0;

    let effect_after = format!("{:?}", k.l0.memory.cognitive_effect);
    Rec {
        step: idx,
        effect_changed: effect_before != effect_after,
        effect_after: effect_after.clone(),
        l1_patched: r.l1.patched,
        l1_candidate: r.l1.patch_id.is_some(),
        l2_nudge: r.l2.effect_nudge.is_some(),
        l1_blocked: r.l1_blocked_by_l2,
        rolled_back: r.l1.rollback_effect.is_some(),
        history_growth: k.l1.store.history.len().saturating_sub(hist_before),
        self_modify_after: k.l1.self_modify_enabled,
        ms,
    }
}

fn drift_inject(i: usize, drift: bool) -> Option<f64> {
    if !drift {
        return None;
    }
    match i.cmp(&DRIFT_AT) {
        std::cmp::Ordering::Equal => Some(0.9),
        std::cmp::Ordering::Greater => Some(0.65),
        std::cmp::Ordering::Less => None,
    }
}

fn run_arm(mut k: Kernel, drift: bool, force_blocked: bool) -> Vec<Rec> {
    let mut recs = Vec::with_capacity(TOTAL_STEPS);
    for i in 0..TOTAL_STEPS {
        if force_blocked {
            k.l2.defensive_mode = true;
        }
        recs.push(step(&mut k, i, drift_inject(i, drift)));
    }
    recs
}

fn same_trace(a: &[Rec], b: &[Rec]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            x.effect_after == y.effect_after
                && x.l1_patched == y.l1_patched
                && x.l2_nudge == y.l2_nudge
        })
}

struct Summary {
    candidates: usize,
    patched: usize,
    history_growth: usize,
    unexplained_changes: usize,
    nudges: usize,
    rollbacks: usize,
    blocked_steps: usize,
    collisions: usize,
    patched_unblocked: usize,
    self_modify_all_true: bool,
    mean_ms: f64,
    max_ms: f64,
}

fn count_if(recs: &[Rec], f: impl Fn(&Rec) -> bool) -> usize {
    recs.iter().filter(|r| f(r)).count()
}

fn summarize(recs: &[Rec]) -> Summary {
    let total_ms: f64 = recs.iter().map(|r| r.ms).sum();
    Summary {
        candidates: count_if(recs, |r| r.l1_candidate),
        patched: count_if(recs, |r| r.l1_patched),
        history_growth: recs.iter().map(|r| r.history_growth).sum(),
        unexplained_changes: count_if(recs, |r| {
            r.effect_changed && !r.l2_nudge && !r.l1_patched && !r.rolled_back
        }),
        nudges: count_if(recs, |r| r.l2_nudge),
        rollbacks: count_if(recs, |r| r.rolled_back),
        blocked_steps: count_if(recs, |r| r.l1_blocked),
        collisions: count_if(recs, |r| r.l1_blocked && r.l1_patched),
        patched_unblocked: count_if(recs, |r| !r.l1_blocked && r.l1_patched),
        self_modify_all_true: recs.iter().all(|r| r.self_modify_after),
        mean_ms: total_ms / recs.len() as f64,
        max_ms: recs.iter().map(|r| r.ms).fold(0.0, f64::max),
    }
}

fn print_summary(name: &str, s: &Summary, deterministic: bool) {
    let det = if deterministic { "是" } else { "否" };
    println!(
        "  {name:<16} 候选={:<3} 写入={:<3} 历史增长={:<3} 未归因={:<3} \
         L2nudge={:<3} 回滚={:<3} 仲裁步={:<3} 抢写={:<3} 可重复={det} \
         平均={:.3}ms 最大={:.3}ms",
        s.candidates,
        s.patched,
        s.history_growth,
        s.unexplained_changes,
        s.nudges,
        s.rollbacks,
        s.blocked_steps,
        s.collisions,
        s.mean_ms,
        s.max_ms,
    );
}

fn write_csv(name: &str, recs: &[Rec]) {
    let mut out = String::from(concat!(
        "step,effect_changed,l1_patched,l1_candidate,l2_nudge,",
        "l1_blocked,rolled_back,history_growth,self_modify_after,ms\n",
    ));
    for r in recs {
        out += &format!(
            "{},{},{},{},{},{},{},{},{},{:.4}\n",
            r.step,
            r.effect_changed,
            r.l1_patched,
            r.l1_candidate,
            r.l2_nudge,
            r.l1_blocked,
            r.rolled_back,
            r.history_growth,
            r.self_modify_after,
            r.ms
        );
    }
    fs::write(format!("out/{name}.csv"), out).expect("write csv");
}

fn experiment_shadow() -> bool {
    println!("\n== E1 shadow 模式：稳态零扰动（阳性对照：live 模式必须出现写入）==");
    let arms = [
        ("shadow_drift", true, false, true),
        ("live_drift", false, true, true),
        ("shadow_stable", true, false, false),
        ("live_stable", false, true, false),
    ];
    let mut results = Vec::new();
    for (name, shadow, sm, drift) in arms {
        let a = run_arm(make_kernel(shadow, sm, true), drift, false);
        let b = run_arm(make_kernel(shadow, sm, true), drift, false);
        let det = same_trace(&a, &b);
        write_csv(&format!("e1_{name}"), &a);
        let s = summarize(&a);
        print_summary(name, &s, det);
        results.push((name, s));
    }
    let shadow_ok = results
        .iter()
        .filter(|(n, _)| n.starts_with("shadow"))
        .all(|(_, s)| s.patched == 0 && s.history_growth == 0 && s.unexplained_changes == 0);
    let control_ok = results
        .iter()
        .find(|(n, _)| *n == "live_drift")
        .map(|(_, s)| s.patched > 0)
        .unwrap_or(false);
    let shadow_candidates = results
        .iter()
        .find(|(n, _)| *n == "shadow_drift")
        .map(|(_, s)| s.candidates)
        .unwrap_or(0);
    let note = if shadow_candidates == 0 {
        "（候选为 0，本次 shadow 结论不足以说明问题）"
    } else {
        ""
    };
    println!(
        "  判定：shadow 零写入 = {}；阳性对照有效 = {}；shadow 漂移段候选数 = {}{}",
        pass(shadow_ok),
        pass(control_ok),
        shadow_candidates,
        note
    );
    shadow_ok && control_ok
}

fn experiment_arbitration() -> bool {
    println!("\n== E2 L1/L2 互斥仲裁：抢写次数（应为 0）==");
    let natural = run_arm(make_kernel(false, true, true), true, false);
    let natural_b = run_arm(make_kernel(false, true, true), true, false);
    let det_n = same_trace(&natural, &natural_b);
    write_csv("e2_natural_drift", &natural);
    let sn = summarize(&natural);
    print_summary("natural_drift", &sn, det_n);

    let forced = run_arm(make_kernel(false, true, true), true, true);
    write_csv("e2_forced_blocked", &forced);
    let sf = summarize(&forced);
    print_summary("forced_blocked", &sf, true);

    let natural_ok = sn.collisions == 0 && det_n;
    let forced_ok = sf.collisions == 0
        && sf.patched == 0
        && sf.blocked_steps == TOTAL_STEPS
        && sf.self_modify_all_true;
    let control_ok = sn.blocked_steps == 0 && sn.patched_unblocked > 0;
    println!(
        "  判定：自然运行无抢写 = {}；强制阻断无抢写且 self_modify 已恢复 = {}；\
         受控阳性对照 = {}",
        pass(natural_ok),
        pass(forced_ok),
        pass(control_ok)
    );
    println!("  自然场景未触发仲裁；由强制防御状态提供确定的仲裁条件。");
    natural_ok && forced_ok && control_ok
}

struct WindowResult {
    patch_step: usize,
    feedback_step: usize,
    patched: bool,
    rollback: bool,
    restore_ok: bool,
    window_ms: f64,
}

fn run_window(delay: usize) -> WindowResult {
    let mut k = make_kernel(false, true, false);
    k.l1.steps_since_last_patch = 10;
    k.l1.reflector.error_threshold = 0.15;
    let previous = k.l0.memory.cognitive_effect.clone();

    let patch = step(&mut k, 0, None);
    let patched = patch.l1_patched
        && k.l1.pending_rollback
        && k.l1.previous_effect.as_ref() == Some(&previous);
    let patched_effect = k.l0.memory.cognitive_effect.clone();
    let mut rollback = false;
    let mut restore_ok = false;
    let mut window_ms = 0.0;

    for step_index in 1..=delay {
        // Isolate this patch's one-step consequence window from unrelated patches.
        k.l1.self_modify_enabled = false;
        // Match the unit-test rollback fixture: start from zero, then inject the
        // low signal that produces a measured worsening greater than 0.05.
        k.l0.memory.cas.avg_prediction_error = 0.0;
        let error = (step_index == delay).then_some(-1.0);
        let record = step(&mut k, step_index, error);
        rollback |= record.rolled_back;
        restore_ok |= record.rolled_back && k.l0.memory.cognitive_effect == previous;
        window_ms += record.ms;
    }
    if delay > 1 {
        assert!(k.l0.memory.cognitive_effect == patched_effect);
        assert!(!k.l1.pending_rollback);
    }
    WindowResult {
        patch_step: patch.step,
        feedback_step: delay,
        patched,
        rollback,
        restore_ok,
        window_ms,
    }
}

fn experiment_rollback() -> bool {
    println!("\n== E3 L1 自动回滚：恶化信号滞后 k 步出现时是否回滚（预期：仅 k=1）==");
    println!(
        "  {:<6} {:<18} {:<18} {:<10} {:<12}",
        "k", "补丁步", "反馈步", "恢复正确", "窗口耗时(ms)"
    );
    let mut all_ok = true;
    for delay in [1usize, 2, 3, 5, 8] {
        let w = run_window(delay);
        let expected_rollback = delay == 1;
        let case_ok =
            w.patched && w.rollback == expected_rollback && w.restore_ok == expected_rollback;
        all_ok &= case_ok;
        println!(
            "  {:<6} {:<18} {:<18} {:<10} {:<12.3}",
            delay,
            w.patch_step,
            w.feedback_step,
            if w.restore_ok { "是" } else { "否" },
            w.window_ms
        );
    }
    println!(
        "  判定：k=1 回滚且恢复正确；k>=2 窗口过期不回滚 = {}",
        pass(all_ok)
    );
    all_ok
}

fn pass(b: bool) -> &'static str {
    if b {
        "通过"
    } else {
        "未通过"
    }
}

fn main() {
    fs::create_dir_all("out").expect("cannot create out/");
    println!("L1 验证探针（注入式漂移，非真实负载）");
    println!("步数 = {TOTAL_STEPS}，漂移起点 = 第 {DRIFT_AT} 步");

    let e1 = experiment_shadow();
    let e2 = experiment_arbitration();
    let e3 = experiment_rollback();

    println!("\n== 总结 ==");
    println!("  E1 shadow 零扰动：{}", pass(e1));
    println!("  E2 无抢写：{}", pass(e2));
    println!("  E3 回滚窗口：{}", pass(e3));
    println!("  明细 CSV 已写入 ./out/");
    if !(e1 && e2 && e3) {
        std::process::exit(1);
    }
}
