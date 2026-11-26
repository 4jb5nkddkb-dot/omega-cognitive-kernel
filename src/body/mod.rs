//! Body / sensor / actuator bus — edge-device channel layer.
//!
//! Minimal live implementation (no hardware I/O yet).
//!
//! Safety rules (non-negotiable):
//! - Never issues tokens or writes CAS.
//! - `write_authorized` is always `false` on every return packet from this module.
//! - Sensor `inject` is in-memory simulation only.
//! - Actuator `propose_command` returns a proposal payload only without mutating channel value.
//! - `apply_command` commits value only when called from a gated execute path.

use crate::prelude::*;
use alloc::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

impl ChannelKind {
    pub fn parse(raw: &str) -> core::result::Result<Self, String> {
        let normalized = raw.trim().to_ascii_lowercase();
        if normalized.is_empty() || normalized == "sensor" {
            Ok(Self::Sensor)
        } else if normalized == "actuator" {
            Ok(Self::Actuator)
        } else if normalized == "bidirectional" {
            Ok(Self::Bidirectional)
        } else {
            Err(format!(
                "kind must be one of [actuator, bidirectional, sensor], got '{normalized}'"
            ))
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sensor => "sensor",
            Self::Actuator => "actuator",
            Self::Bidirectional => "bidirectional",
        }
    }
}

/// Lifecycle status returned in [`BodyPacket`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyPacketStatus {
    Registered,
    Ok,
    Injected,
    Proposed,
    Applied,
    Rejected,
    UnknownChannel,
}

/// Structured actuator command proposal emitted before token-gated execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyCommandProposal {
    pub action: String,
    pub channel: String,
    pub value: Value,
    pub reason: String,
    pub kind: ChannelKind,
    pub applied: bool,
    pub ts_ms: u64,
}

/// Packet returned by every [`BodyBus`] operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyPacket {
    pub status: BodyPacketStatus,
    pub channel: String,
    pub kind: Option<ChannelKind>,
    pub value: Option<Value>,
    pub updated_at_ms: Option<u64>,
    pub write_authorized: bool,
    pub applied: bool,
    pub error: Option<String>,
    pub proposal: Option<BodyCommandProposal>,
}

/// Channel metadata snapshot returned by [`BodyBus::list_channels`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyChannelInfo {
    pub name: String,
    pub kind: ChannelKind,
    pub has_value: bool,
    pub updated_at_ms: Option<u64>,
}

/// Aggregate observability summary for [`BodyBus`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodySummary {
    pub channel_count: usize,
    pub sensors: usize,
    pub actuators: usize,
    pub bidirectional: usize,
    pub channels: Vec<BodyChannelInfo>,
    pub write_authorized: bool,
    pub note: &'static str,
}

#[derive(Debug, Clone)]
struct ChannelState {
    kind: ChannelKind,
    value: Option<Value>,
    updated_at_ms: Option<u64>,
    meta: BTreeMap<String, Value>,
    last_command: Option<BodyCommandProposal>,
}

/// Named in-memory channels for sensors and actuators.
///
/// Channels hold the last known value in memory. Nothing here authorizes
/// world writes or CAS mutations.
#[derive(Debug, Clone, Default)]
pub struct BodyBus {
    channels: BTreeMap<String, ChannelState>,
}

impl BodyBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        name: &str,
        kind: ChannelKind,
        meta: Option<BTreeMap<String, Value>>,
    ) -> core::result::Result<BodyPacket, String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("channel name must be non-empty".to_string());
        }
        let key = trimmed.to_string();
        let existing = self.channels.get(&key);
        let value = existing.and_then(|c| c.value.clone());
        let updated_at_ms = existing.and_then(|c| c.updated_at_ms);
        let last_command = existing.and_then(|c| c.last_command.clone());

        self.channels.insert(
            key.clone(),
            ChannelState {
                kind,
                value,
                updated_at_ms,
                meta: meta.unwrap_or_default(),
                last_command,
            },
        );
        Ok(self.packet(&key, BodyPacketStatus::Registered, None))
    }

    pub fn unregister(&mut self, name: &str) -> bool {
        self.channels.remove(name).is_some()
    }

    pub fn channel_meta(&self, name: &str) -> Option<&BTreeMap<String, Value>> {
        self.channels.get(name).map(|c| &c.meta)
    }

    pub fn list_channels(&self) -> Vec<BodyChannelInfo> {
        self.channels
            .iter()
            .map(|(name, ch)| BodyChannelInfo {
                name: name.clone(),
                kind: ch.kind,
                has_value: ch.value.is_some(),
                updated_at_ms: ch.updated_at_ms,
            })
            .collect()
    }

    /// Read last known value for a channel (observation only).
    pub fn read(&self, name: &str) -> BodyPacket {
        if !self.channels.contains_key(name) {
            return self.packet(
                name,
                BodyPacketStatus::UnknownChannel,
                Some("not_registered"),
            );
        }
        self.packet(name, BodyPacketStatus::Ok, None)
    }

    /// Simulate a sensor reading (in-memory only).
    ///
    /// Allowed for `Sensor` and `Bidirectional` channels. Does not touch the world.
    pub fn inject(&mut self, name: &str, value: Value, timestamp_ms: u64) -> BodyPacket {
        let Some(ch) = self.channels.get_mut(name) else {
            return self.packet(
                name,
                BodyPacketStatus::UnknownChannel,
                Some("not_registered"),
            );
        };
        if ch.kind == ChannelKind::Actuator {
            return self.packet(
                name,
                BodyPacketStatus::Rejected,
                Some("inject_not_allowed_on_actuator"),
            );
        }
        ch.value = Some(value);
        ch.updated_at_ms = Some(timestamp_ms);
        self.packet(name, BodyPacketStatus::Injected, None)
    }

    /// Build an actuator command proposal without mutating the channel value.
    pub fn propose_command(
        &mut self,
        name: &str,
        value: Value,
        reason: &str,
        timestamp_ms: u64,
    ) -> BodyPacket {
        let Some(ch) = self.channels.get_mut(name) else {
            return self.packet(
                name,
                BodyPacketStatus::UnknownChannel,
                Some("not_registered"),
            );
        };
        if ch.kind == ChannelKind::Sensor {
            return self.packet(
                name,
                BodyPacketStatus::Rejected,
                Some("command_not_allowed_on_sensor"),
            );
        }
        let proposal = BodyCommandProposal {
            action: "body_command".to_string(),
            channel: name.to_string(),
            value,
            reason: reason.to_string(),
            kind: ch.kind,
            applied: false,
            ts_ms: timestamp_ms,
        };
        ch.last_command = Some(proposal.clone());
        let mut pkt = self.packet(name, BodyPacketStatus::Proposed, None);
        pkt.proposal = Some(proposal);
        pkt
    }

    /// Commit actuator value after a token-gated execute step.
    ///
    /// Still never issues tokens or writes CAS; `write_authorized` stays `false`.
    pub fn apply_command(&mut self, name: &str, value: Value, timestamp_ms: u64) -> BodyPacket {
        let Some(ch) = self.channels.get_mut(name) else {
            return self.packet(
                name,
                BodyPacketStatus::UnknownChannel,
                Some("not_registered"),
            );
        };
        if ch.kind == ChannelKind::Sensor {
            return self.packet(
                name,
                BodyPacketStatus::Rejected,
                Some("command_not_allowed_on_sensor"),
            );
        }
        let proposal = BodyCommandProposal {
            action: "body_command".to_string(),
            channel: name.to_string(),
            value: value.clone(),
            reason: String::new(),
            kind: ch.kind,
            applied: true,
            ts_ms: timestamp_ms,
        };
        ch.value = Some(value);
        ch.updated_at_ms = Some(timestamp_ms);
        ch.last_command = Some(proposal);
        let mut pkt = self.packet(name, BodyPacketStatus::Applied, None);
        pkt.write_authorized = false;
        pkt.applied = true;
        pkt
    }

    pub fn summary(&self) -> BodySummary {
        let mut sensors = 0usize;
        let mut actuators = 0usize;
        let mut bidirectional = 0usize;
        for ch in self.channels.values() {
            match ch.kind {
                ChannelKind::Sensor => sensors += 1,
                ChannelKind::Actuator => actuators += 1,
                ChannelKind::Bidirectional => bidirectional += 1,
            }
        }
        BodySummary {
            channel_count: self.channels.len(),
            sensors,
            actuators,
            bidirectional,
            channels: self.list_channels(),
            write_authorized: false,
            note: "body_bus_gated_via_suggestion",
        }
    }

   fn packet(&self, name: &str, status: BodyPacketStatus, error: Option<&str>) -> BodyPacket {
        let ch = self.channels.get(name);
        BodyPacket {
            status,
            channel: name.to_string(),
            kind: ch.map(|c| c.kind),
            value: ch.and_then(|c| c.value.clone()),
            updated_at_ms: ch.and_then(|c| c.updated_at_ms),
            write_authorized: false,
            applied: false,
            error: error.map(str::to_string),
            proposal: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn register_list_and_unregister_channels() {
        let mut bus = BodyBus::new();
        assert!(bus.register("  ", ChannelKind::Sensor, None).is_err());

        let pkt = bus
            .register("temp", ChannelKind::Sensor, None)
            .expect("register sensor");
        assert_eq!(pkt.status, BodyPacketStatus::Registered);
        assert!(!pkt.write_authorized);

        bus.register("valve", ChannelKind::Actuator, None)
            .expect("register actuator");
        let list = bus.list_channels();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "temp");
        assert_eq!(list[1].name, "valve");

        assert!(bus.unregister("temp"));
        assert!(!bus.unregister("temp"));
    }

    #[test]
    fn sensor_inject_and_reject_actuator() {
        let mut bus = BodyBus::new();
        bus.register("temp", ChannelKind::Sensor, None).unwrap();
        bus.register("motor", ChannelKind::Actuator, None).unwrap();

        let injected = bus.inject("temp", json!(23.5), 1_000);
        assert_eq!(injected.status, BodyPacketStatus::Injected);
        assert_eq!(injected.value, Some(json!(23.5)));
        assert!(!injected.write_authorized);

        let rejected = bus.inject("motor", json!(1.0), 1_001);
        assert_eq!(rejected.status, BodyPacketStatus::Rejected);
        assert_eq!(
            rejected.error.as_deref(),
            Some("inject_not_allowed_on_actuator")
        );
        assert!(!rejected.write_authorized);
    }

    #[test]
    fn propose_does_not_mutate_until_apply_and_never_authorizes_writes() {
        let mut bus = BodyBus::new();
        bus.register("temp", ChannelKind::Sensor, None).unwrap();
        bus.register("servo", ChannelKind::Bidirectional, None)
            .unwrap();

        let sensor_cmd = bus.propose_command("temp", json!(10), "test", 2_000);
        assert_eq!(sensor_cmd.status, BodyPacketStatus::Rejected);
        assert_eq!(
            sensor_cmd.error.as_deref(),
            Some("command_not_allowed_on_sensor")
        );

        let proposed = bus.propose_command("servo", json!(45), "align", 2_001);
        assert_eq!(proposed.status, BodyPacketStatus::Proposed);
        assert!(proposed.value.is_none());
        assert!(proposed.proposal.is_some());
        assert!(!proposed.write_authorized);

        let applied = bus.apply_command("servo", json!(45), 2_002);
        assert_eq!(applied.status, BodyPacketStatus::Applied);
        assert!(applied.applied);
        assert_eq!(applied.value, Some(json!(45)));
        assert!(!applied.write_authorized);

        let summary = bus.summary();
        assert_eq!(summary.sensors, 1);
        assert_eq!(summary.bidirectional, 1);
        assert!(!summary.write_authorized);
    }
}
/// Supported channel directions on [`BodyBus`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    Sensor,
    Actuator,
    Bidirectional,
}
