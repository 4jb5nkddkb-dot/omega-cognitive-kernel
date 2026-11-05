//! Minimal task environment: nonstationary 2-armed bandit.
//! External reward is written back into strategy memory so the task loop is closed.

use crate::api::Kernel;
use crate::error::Result;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct BanditReport {
    pub episodes: u32,
    pub steps_per_episode: u32,
    pub mean_reward: f64,
    pub best_arm_rate: f64,
    pub final_damping: f64,
    pub final_explore_bonus: f64,
    pub final_risk_tolerance: f64,
    pub error_decreased: bool,
    pub patches_accepted: u64,
    pub seed: u64,
}

struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed | 1 }
    }

    fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.state >> 8) as f64 / (1u64 << 56) as f64
    }
}

fn arm_for_action(action: &str) -> usize {
    match action {
        "stabilize" | "exploit" => 0,
        "explore" => 1,
        _ => 0,
    }
}

pub fn run_bandit_task(
    kernel: &mut Kernel,
    episodes: u32,
    steps_per_episode: u32,
) -> Result<BanditReport> {
    run_bandit_task_seeded(kernel, episodes, steps_per_episode, 405123076)
}

pub fn run_bandit_task_seeded(
    kernel: &mut Kernel,
    episodes: u32,
    steps_per_episode: u32,
    seed: u64,
) -> Result<BanditReport> {
    let mut rng = Rng::new(seed);
    let mut total_reward = 0.0;
    let mut best_hits = 0u32;
    let mut total_choices = 0u32;
    let mut patches = 0u64;
    let early_err = kernel.inner.l0.memory.cas.avg_prediction_error;

    for ep in 0..episodes {
        // Non-stationary: best arm flips halfway (and once more late if many episodes).
        let best = if episodes >= 4 && ep >= (episodes * 3) / 4 {
            0
        } else if ep < episodes / 2 {
            0
        } else {
            1
        };
        let mut ep_reward = 0.0;

        for _ in 0..steps_per_episode {
            let r = kernel.step("maximize bandit reward under uncertainty")?;
            if r.l1.patched {
                patches += 1;
            }
            let action = r
                .trajectory
                .actions
                .first()
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "observe".into());
            let arm = arm_for_action(&action);
            let p = if arm == best { 0.85 } else { 0.25 };
            let reward = if rng.next_f64() < p { 1.0 } else { 0.0 };

            kernel.inner.l0.memory.record_reward(&action, reward);

            ep_reward += reward;
            total_reward += reward;
            total_choices += 1;
            if arm == best {
                best_hits += 1;
            }
        }

        kernel.audit_task("bandit2", ep_reward, steps_per_episode)?;
    }

    let final_err = kernel.inner.l0.memory.cas.avg_prediction_error;
    let effect = &kernel.inner.l0.memory.cognitive_effect;
    Ok(BanditReport {
        episodes,
        steps_per_episode,
        mean_reward: if total_choices == 0 {
            0.0
        } else {
            total_reward / total_choices as f64
        },
        best_arm_rate: if total_choices == 0 {
            0.0
        } else {
            best_hits as f64 / total_choices as f64
        },
        final_damping: effect.error_damping,
        final_explore_bonus: effect.explore_bonus,
        final_risk_tolerance: effect.risk_tolerance,
        error_decreased: final_err < early_err || early_err == 0.0,
        patches_accepted: patches,
        seed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;
    use uuid::Uuid;

    #[test]
    fn bandit_runs() {
        let path = temp_dir().join(format!("omega-bandit-{}.json", Uuid::new_v4()));
        let mut k = Kernel::with_state_path(&path);
        k.autosave = false;
        let report = run_bandit_task_seeded(&mut k, 4, 3, 42).unwrap();
        assert_eq!(report.episodes, 4);
        assert!(report.mean_reward >= 0.0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn reward_feeds_strategy_memory() {
        let path = temp_dir().join(format!("omega-rw-{}.json", Uuid::new_v4()));
        let mut k = Kernel::with_state_path(&path);
        k.autosave = false;
        k.auto_checkpoint_on_patch = false;
        let before = k.inner.l0.memory.strategy.retrieve("", 20).len();
        let _ = run_bandit_task_seeded(&mut k, 3, 4, 7).unwrap();
        let after = k.inner.l0.memory.strategy.retrieve("", 20);
        assert!(after.len() >= before);
        assert!(after.iter().any(|s| s.success_count + s.fail_count > 0));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn seeded_bandit_is_deterministic() {
        let p1 = temp_dir().join(format!("omega-d1-{}.json", Uuid::new_v4()));
        let p2 = temp_dir().join(format!("omega-d2-{}.json", Uuid::new_v4()));
        let mut k1 = Kernel::with_state_path(&p1);
        let mut k2 = Kernel::with_state_path(&p2);
        k1.autosave = false;
        k2.autosave = false;
        k1.auto_checkpoint_on_patch = false;
        k2.auto_checkpoint_on_patch = false;
        let r1 = run_bandit_task_seeded(&mut k1, 5, 3, 12345).unwrap();
        let r2 = run_bandit_task_seeded(&mut k2, 5, 3, 12345).unwrap();
        assert_eq!(r1.mean_reward, r2.mean_reward);
        assert_eq!(r1.best_arm_rate, r2.best_arm_rate);
        let _ = std::fs::remove_file(&p1);
        let _ = std::fs::remove_file(&p2);
    }
}
