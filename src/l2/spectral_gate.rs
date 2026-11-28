#[allow(unused_imports)]
use crate::prelude::*;

use crate::l1::spectral::{L1SpectralReflector, SpectralPatch};

/// 认知三重境界（显式状态遥测，用于审计 L0/L1/L2 闭环位置）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CognitiveRealm {
    /// 第一重：看山是山（稳态无为，L1 静默且 bias == 0.0，纯 KEEP_L0）
    FirstMountainKeepL0,
    /// 第二重：看山不是山（L1 已解构频谱，但 L2 识别为毛刺/未达门限，按兵不动）
    NotMountainDeconstruct,
    /// 第三重：看山还是山（L2 逆变换重塑趋势并注入/收敛 bias，向新稳态归位）
    StillMountainRestore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum L2State {
    Dormant,
    Active,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum L2Decision {
    KeepL0,
    InjectBias(f64),
}

#[derive(Debug, Clone, Copy)]
pub struct L2Config {
    /// 与 L1 对齐的窗口长度（用于匹配 bias 半衰期）
    pub window_size: usize,
    /// 进入 ACTIVE 的直流漂移门限 |dc_baseline - dc_anchor|
    pub enter_dc_drift: f64,
    /// 退出 ACTIVE 的直流漂移门限（迟滞下界，应小于 enter_dc_drift）
    pub exit_dc_drift: f64,
    /// 触发 ACTIVE 的低频有效能量占比门限 (1.0 - high_freq_ratio)
    pub enter_lf_ratio: f64,
    /// 从 ACTIVE 退回 DORMANT 需连续满足退出条件的步数
    pub dormant_confirm_steps: usize,
    /// DORMANT 态下 dc_anchor 的慢速 EMA 跟踪系数
    pub anchor_ema_alpha: f64,
    /// bias 指数衰减因子 gamma（例如 exp(-2.0 / window_size)）
    pub decay_gamma: f64,
    /// bias 注入增益 eta（建议与 1.0 - decay_gamma 同量级，保证 BIBO 有界）
    pub inject_eta: f64,
    /// 归零死区：DORMANT 且 |bias| < deadband_eps 时硬归 0.0
    pub deadband_eps: f64,
    /// 逆变换末端取最后 tail_len 个点做均值，抑制硬截断吉布斯振铃
    pub tail_avg_len: usize,
    /// Minimum AC energy near a zero anchor; prevents amplification of rounding
    /// noise while relative SNR handles changes of measurement units.
    pub min_ac_energy: f64,
    /// Relative low-frequency energy threshold for the second activation gate.
    pub enter_lf_snr: f64,
    /// High-frequency penalty applied to the injection gain.
    pub high_freq_penalty: f64,
    /// Absolute symmetric bias bound. It is in the input signal's units.
    pub max_bias: f64,
    /// Enable spectral permission for new expert proposals (ablation switch).
    pub control_enabled: bool,
    /// Enable spectral purity weighting of expert priors (ablation switch).
    pub purity_prior_enabled: bool,
    /// Enable one-shot statistic discount on confirmed activation.
    pub history_discount_enabled: bool,
    pub history_discount_factor: f64,
    /// Ablate only the reverse-sign braking term.
    pub reverse_brake_enabled: bool,
}

impl L2Config {
    pub fn with_window(window_size: usize) -> Self {
        let n = window_size.max(1) as f64;
        let decay_gamma = libm::exp(-2.0 / n);
        Self {
            window_size,
            enter_dc_drift: 0.15,
            exit_dc_drift: 0.05,
            enter_lf_ratio: 0.65,
            dormant_confirm_steps: 3,
            anchor_ema_alpha: 0.05,
            decay_gamma,
            inject_eta: 1.0 - decay_gamma,
            deadband_eps: 1e-4,
            tail_avg_len: 3,
            min_ac_energy: 1e-12,
            enter_lf_snr: 0.01,
            high_freq_penalty: 2.0,
            max_bias: 1.0,
            // Experimental interventions are opt-in. Keep the historical
            // spectral observer untouched unless a benchmark explicitly enables
            // the corresponding ablation component.
            control_enabled: false,
            purity_prior_enabled: false,
            history_discount_enabled: false,
            history_discount_factor: 0.85,
            reverse_brake_enabled: false,
        }
    }

    fn sanitized(mut self) -> Self {
        self.window_size = self.window_size.max(2);
        self.enter_dc_drift = self.enter_dc_drift.abs();
        self.exit_dc_drift = self.exit_dc_drift.abs().min(self.enter_dc_drift);
        self.enter_lf_ratio = self.enter_lf_ratio.clamp(0.0, 1.0);
        self.anchor_ema_alpha = self.anchor_ema_alpha.clamp(0.0, 1.0);
        self.decay_gamma = self.decay_gamma.clamp(0.0, 0.999999);
        self.inject_eta = self.inject_eta.max(0.0);
        self.min_ac_energy = self.min_ac_energy.max(0.0);
        self.enter_lf_snr = self.enter_lf_snr.max(0.0);
        self.high_freq_penalty = self.high_freq_penalty.max(0.0);
        self.max_bias = self.max_bias.max(1e-9);
        self.history_discount_factor = self.history_discount_factor.clamp(0.01, 1.0);
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct KernelStepOutput {
    pub realm: CognitiveRealm,
    pub state: L2State,
    pub decision: L2Decision,
    pub bias: f64,
    pub dc_anchor: f64,
    /// Dayan 收敛残差：最新观测相对 (dc_anchor + bias) 的未吸收误差
    pub convergence_residual: f64,
    pub dc_drift: f64,
    pub normalized_energy: f64,
    pub low_freq_ratio: f64,
    pub eta_eff: f64,
    /// This flag only marks the confirmed dormant-to-active transition. It never
    /// authorizes an L0 action by itself; history discount remains an opt-in
    /// experiment and all existing commit gates still apply.
    pub regime_detected: bool,
}

/// L2 元认知惊奇度门控 + bias 衰减引擎
pub struct L2CognitiveGate {
    cfg: L2Config,
    state: L2State,
    bias: f64,
    dc_anchor: f64,
    anchor_initialized: bool,
    calm_streak: usize,
}

impl L2CognitiveGate {
    pub fn new(cfg: L2Config) -> Self {
        Self {
            cfg: cfg.sanitized(),
            state: L2State::Dormant,
            bias: 0.0,
            dc_anchor: 0.0,
            anchor_initialized: false,
            calm_streak: 0,
        }
    }

    pub fn step(
        &mut self,
        reflector: &L1SpectralReflector,
        l0_rewards: &[f64],
    ) -> KernelStepOutput {
        let latest = l0_rewards.last().copied().unwrap_or(0.0);

        if !self.anchor_initialized
            && l0_rewards.len() >= self.cfg.window_size
            && self.cfg.window_size > 0
        {
            let win = &l0_rewards[l0_rewards.len() - self.cfg.window_size..];
            self.dc_anchor = win.iter().sum::<f64>() / (self.cfg.window_size as f64);
            self.anchor_initialized = true;
        }

        let patch_opt = if self.anchor_initialized {
            reflector.reflect_and_patch_anchored(l0_rewards, self.dc_anchor)
        } else {
            None
        };
        let mut dc_drift = 0.0;
        let mut normalized_energy = 0.0;
        let mut low_freq_ratio = 0.0;
        let mut eta_eff = 0.0;
        let mut regime_detected = false;

        let realm = match patch_opt {
            None => {
                if self.state == L2State::Active {
                    self.calm_streak += 1;
                    if self.calm_streak >= self.cfg.dormant_confirm_steps {
                        self.state = L2State::Dormant;
                        self.calm_streak = 0;
                    }
                }
                self.decay_bias_only();

                if self.state == L2State::Dormant && self.anchor_initialized {
                    self.dc_anchor = (1.0 - self.cfg.anchor_ema_alpha) * self.dc_anchor
                        + self.cfg.anchor_ema_alpha * latest;
                }

                if self.bias == 0.0 && self.state == L2State::Dormant {
                    CognitiveRealm::FirstMountainKeepL0
                } else {
                    CognitiveRealm::StillMountainRestore
                }
            }

            Some(patch) => {
                if !self.anchor_initialized {
                    self.dc_anchor = patch.dc_baseline;
                    self.anchor_initialized = true;
                }

                dc_drift = (patch.dc_baseline - self.dc_anchor).abs();
                normalized_energy = patch.normalized_energy;
                low_freq_ratio = 1.0 - patch.high_freq_ratio;
                let anchor_power = self.dc_anchor * self.dc_anchor;
                let low_freq_snr = (patch.total_ac * low_freq_ratio) / anchor_power.max(1.0);
                let significant_dc = dc_drift > self.cfg.enter_dc_drift;
                let significant_low_freq = low_freq_ratio > self.cfg.enter_lf_ratio
                    && patch.total_ac >= self.cfg.min_ac_energy
                    && low_freq_snr > self.cfg.enter_lf_snr;

                match self.state {
                    L2State::Dormant => {
                        if significant_dc || significant_low_freq {
                            self.state = L2State::Active;
                            self.calm_streak = 0;
                            regime_detected = significant_dc;
                            eta_eff = self.inject_from_patch(&patch);
                            CognitiveRealm::StillMountainRestore
                        } else {
                            self.decay_bias_only();
                            self.dc_anchor = (1.0 - self.cfg.anchor_ema_alpha) * self.dc_anchor
                                + self.cfg.anchor_ema_alpha * patch.dc_baseline;

                            if self.bias == 0.0 {
                                CognitiveRealm::NotMountainDeconstruct
                            } else {
                                CognitiveRealm::StillMountainRestore
                            }
                        }
                    }

                    L2State::Active => {
                        if dc_drift < self.cfg.exit_dc_drift && !significant_low_freq {
                            self.calm_streak += 1;
                            if self.calm_streak >= self.cfg.dormant_confirm_steps {
                                self.state = L2State::Dormant;
                                self.calm_streak = 0;
                                self.decay_bias_only();
                            } else {
                                eta_eff = self.inject_from_patch(&patch);
                            }
                        } else {
                            self.calm_streak = 0;
                            eta_eff = self.inject_from_patch(&patch);
                        }
                        CognitiveRealm::StillMountainRestore
                    }
                }
            }
        };

        self.clamp_bias();
        let decision = if realm != CognitiveRealm::StillMountainRestore {
            L2Decision::KeepL0
        } else {
            L2Decision::InjectBias(self.bias)
        };

        let convergence_residual = latest - (self.dc_anchor + self.bias);

        KernelStepOutput {
            realm,
            state: self.state,
            decision,
            bias: self.bias,
            dc_anchor: self.dc_anchor,
            convergence_residual,
            dc_drift,
            normalized_energy,
            low_freq_ratio,
            eta_eff,
            regime_detected: regime_detected && self.cfg.history_discount_enabled,
        }
    }

    fn inject_from_patch(&mut self, patch: &SpectralPatch) -> f64 {
        let restored = L1SpectralReflector::inverse_restore(&patch.filtered_spectrum);
        if restored.is_empty() {
            self.decay_bias_only();
            return 0.0;
        }
        let k = self.cfg.tail_avg_len.max(1).min(restored.len());
        let tail_avg = restored[restored.len() - k..].iter().sum::<f64>() / (k as f64);
        let trend_delta = tail_avg - self.dc_anchor;
        if !trend_delta.is_finite() {
            self.decay_bias_only();
            return 0.0;
        }
        let gamma = if self.cfg.reverse_brake_enabled
            && self.bias != 0.0
            && trend_delta * self.bias < 0.0
        {
            self.cfg.decay_gamma * self.cfg.decay_gamma
        } else {
            self.cfg.decay_gamma
        };
        let lf_ratio = 1.0 - patch.high_freq_ratio;
        let eta_eff = self.cfg.inject_eta * lf_ratio
            / (1.0 + self.cfg.high_freq_penalty * patch.high_freq_ratio);
        self.bias = gamma * self.bias + eta_eff * trend_delta;
        self.clamp_bias();
        eta_eff
    }

    fn clamp_bias(&mut self) {
        self.bias = if self.bias.is_finite() {
            self.bias.clamp(-self.cfg.max_bias, self.cfg.max_bias)
        } else {
            0.0
        };
    }

    pub fn configure_control(&mut self, enabled: bool) {
        self.cfg.control_enabled = enabled;
    }

    pub fn configure_ablation(
        &mut self,
        purity_prior: bool,
        reverse_brake: bool,
        history_discount: bool,
    ) {
        self.cfg.purity_prior_enabled = purity_prior;
        self.cfg.reverse_brake_enabled = reverse_brake;
        self.cfg.history_discount_enabled = history_discount;
    }

    pub fn permits_intervention(&self, output: &KernelStepOutput) -> bool {
        !self.cfg.control_enabled || output.realm == CognitiveRealm::StillMountainRestore
    }

    pub fn explore_prior(&self, output: &KernelStepOutput) -> [f64; 2] {
        if !self.cfg.purity_prior_enabled {
            return [0.1, 0.9];
        }
        let strength = (output.bias.abs() / self.cfg.max_bias).clamp(0.0, 1.0);
        [1.0 - strength, strength]
    }

    pub fn history_discount_factor(&self) -> f64 {
        self.cfg.history_discount_factor
    }

    fn decay_bias_only(&mut self) {
        self.bias *= self.cfg.decay_gamma;
        self.clamp_bias();
        if self.state == L2State::Dormant && self.bias.abs() < self.cfg.deadband_eps {
            self.bias = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_mountain_stays_keep_l0_on_steady_rewards() {
        let reflector = L1SpectralReflector::new(8, 1e-4, 1);
        let mut gate = L2CognitiveGate::new(L2Config::with_window(8));
        let out = gate.step(&reflector, &[0.5; 8]);
        assert_eq!(out.realm, CognitiveRealm::FirstMountainKeepL0);
        assert_eq!(out.state, L2State::Dormant);
        assert_eq!(out.decision, L2Decision::KeepL0);
        assert_eq!(out.bias, 0.0);
    }

    #[test]
    fn high_frequency_jitter_stays_in_second_realm_without_injecting_bias() {
        let reflector = L1SpectralReflector::new(8, 1e-4, 1);
        let mut gate = L2CognitiveGate::new(L2Config::with_window(8));
        let _ = gate.step(&reflector, &[0.0; 8]);
        let jitter = [0.5, -0.5, 0.5, -0.5, 0.5, -0.5, 0.5, -0.5];
        let out = gate.step(&reflector, &jitter);
        assert_eq!(out.realm, CognitiveRealm::NotMountainDeconstruct);
        assert_eq!(out.state, L2State::Dormant);
        assert_eq!(out.decision, L2Decision::KeepL0);
    }

    fn experiment_config() -> L2Config {
        L2Config {
            control_enabled: true,
            purity_prior_enabled: true,
            reverse_brake_enabled: true,
            history_discount_enabled: true,
            ..L2Config::with_window(8)
        }
    }

    #[test]
    fn experimental_components_are_disabled_until_explicitly_enabled() {
        let config = L2Config::with_window(8);
        assert!(!config.control_enabled);
        assert!(!config.purity_prior_enabled);
        assert!(!config.reverse_brake_enabled);
        assert!(!config.history_discount_enabled);
        let reflector = L1SpectralReflector::new(8, 1e-4, 1);
        let mut gate = L2CognitiveGate::new(config);
        let _ = gate.step(&reflector, &[0.0; 8]);
        let shift = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0];
        let output = gate.step(&reflector, &shift);
        assert!(
            !output.regime_detected,
            "history discount is not armed without opt-in"
        );
        assert!(gate.permits_intervention(&output));
        assert_eq!(gate.explore_prior(&output), [0.1, 0.9]);
    }

    #[test]
    fn regime_shift_activates_third_realm_and_injects_bounded_bias() {
        let reflector = L1SpectralReflector::new(8, 1e-4, 1);
        let mut gate = L2CognitiveGate::new(experiment_config());
        let _ = gate.step(&reflector, &[0.0; 8]);
        let shift = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0];
        let out = gate.step(&reflector, &shift);
        assert_eq!(out.realm, CognitiveRealm::StillMountainRestore);
        assert_eq!(out.state, L2State::Active);
        assert!(matches!(out.decision, L2Decision::InjectBias(b) if b > 0.0));
        assert!(out.dc_drift > 0.0);
        assert!(out.regime_detected);
    }

    #[test]
    fn tiny_slow_drift_stays_in_second_realm() {
        let reflector = L1SpectralReflector::new(8, 1e-4, 1);
        let mut gate = L2CognitiveGate::new(L2Config::with_window(8));
        let _ = gate.step(&reflector, &[1.0; 8]);
        let tiny = [1.0, 1.0, 1.0, 1.0, 1.2, 1.2, 1.2, 1.2];
        let out = gate.step(&reflector, &tiny);
        assert_eq!(out.realm, CognitiveRealm::NotMountainDeconstruct);
        assert_eq!(out.decision, L2Decision::KeepL0);
    }

    #[test]
    fn opposite_trend_brakes_and_bias_is_bounded() {
        let reflector = L1SpectralReflector::new(8, 1e-4, 1);
        let config = L2Config {
            max_bias: 0.2,
            ..experiment_config()
        };
        let mut gate = L2CognitiveGate::new(config);
        let _ = gate.step(&reflector, &[0.0; 8]);
        let up = gate.step(&reflector, &[0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0]);
        assert!(up.bias > 0.0 && up.bias <= 0.2);
        let down = gate.step(&reflector, &[0.0, 0.0, 0.0, 0.0, -1.0, -1.0, -1.0, -1.0]);
        assert!(down.bias.abs() <= 0.2);
        assert!(down.bias < up.bias);
    }
}
