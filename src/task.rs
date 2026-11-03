//! Minimal real-ish task environment: nonstationary 2-armed bandit (S3).

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
    pub error_decreased: bool,
}

fn arm_for_action(action: &str) -> usize {
    match action {
        "stabilize" | "exploit" => 0,
        "explore" => 1,
        _ => 0,
    }
}

pub fn run_bandit_task(kernel: &mut Kernel, episodes: u32, steps_per_episode: u32) -> Result<BanditReport> {
    let mut total_reward = 0.0;
    let mut best_hits = 0u32;
    let mut total_choices = 0u32;
    let early_err = kernel.inner.l0.memory.cas.avg_prediction_error;

    for ep in 0..episodes {
        let best = if ep < episodes / 2 { 0 } else { 1 };
        let mut ep_reward = 0.0;

        for _ in 0..steps_per_episode {
            let r = kernel.step("maximize bandit reward under uncertainty")?;
            let action = r
                .trajectory
                .actions
                .first()
                .map(|a| a.name.as_str())
                .unwrap_or("observe");
            let arm = arm_for_action(action);
            let p = if arm == best { 0.85 } else { 0.25 };
            let reward = if (kernel.inner.total_steps as f64 * 0.17).sin().abs() < p {
                1.0
            } else {
                0.0
            };
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
        final_damping: kernel.inner.l0.memory.cognitive_effect.error_damping,
        error_decreased: final_err < early_err || early_err == 0.0,
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
        let report = run_bandit_task(&mut k, 4, 3).unwrap();
        assert_eq!(report.episodes, 4);
        assert!(report.mean_reward >= 0.0);
        let _ = std::fs::remove_file(&path);
    }
}
