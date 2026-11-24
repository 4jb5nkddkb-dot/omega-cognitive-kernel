#[allow(unused_imports)]
use crate::prelude::*;

use alloc::collections::BTreeSet;
use serde::{Deserialize, Serialize};

use super::crypto::{ct_eq, hex_encode, hmac_sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionBoundary {
    pub allowed_actions: BTreeSet<String>,
}

impl ActionBoundary {
    pub fn new<I, S>(actions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let allowed_actions = actions.into_iter().map(Into::into).collect();
        Self { allowed_actions }
    }

    pub fn permits(&self, action: &str) -> bool {
        self.allowed_actions.contains(action)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizationToken {
    pub token_id: String,
    pub owner_id: String,
    pub epoch: u64,
    pub suggestion_id: String,
    pub action: String,
    pub args_hash: String,
    pub issued_at_step: u64,
    pub ttl_steps: u64,
    pub mac: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenBook {
    pub issued: Vec<AuthorizationToken>,
    pub consumed: BTreeSet<String>,
    next_seq: u64,
}

impl TokenBook {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(clippy::too_many_arguments)]
    fn compute_mac(
        key: &[u8],
        token_id: &str,
        owner_id: &str,
        epoch: u64,
        suggestion_id: &str,
        action: &str,
        args_hash: &str,
        issued_at_step: u64,
        ttl_steps: u64,
    ) -> String {
        let msg = format!(
            "{token_id}|{owner_id}|{epoch}|{suggestion_id}|{action}|{args_hash}|{issued_at_step}|{ttl_steps}"
        );
        hex_encode(&hmac_sha256(key, msg.as_bytes()))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn issue(
        &mut self,
        key: &[u8],
        owner_id: impl Into<String>,
        epoch: u64,
        suggestion_id: impl Into<String>,
        action: impl Into<String>,
        args_hash: impl Into<String>,
        current_step: u64,
        ttl_steps: u64,
    ) -> AuthorizationToken {
        self.next_seq += 1;
        let token_id = format!("tok-{epoch}-{}", self.next_seq);
        let owner_id = owner_id.into();
        let suggestion_id = suggestion_id.into();
        let action = action.into();
        let args_hash = args_hash.into();
        let mac = Self::compute_mac(
            key,
            &token_id,
            &owner_id,
            epoch,
            &suggestion_id,
            &action,
            &args_hash,
            current_step,
            ttl_steps,
        );
        let tok = AuthorizationToken {
            token_id,
            owner_id,
            epoch,
            suggestion_id,
            action,
            args_hash,
            issued_at_step: current_step,
            ttl_steps,
            mac,
        };
        self.issued.push(tok.clone());
        tok
    }

    #[allow(clippy::too_many_arguments)]
    pub fn consume(
        &mut self,
        key: &[u8],
        token_id: &str,
        owner_id: &str,
        epoch: u64,
        suggestion_id: &str,
        action: &str,
        args_hash: &str,
        current_step: u64,
    ) -> bool {
        if self.consumed.contains(token_id) {
            return false;
        }
        let Some(tok) = self.issued.iter().find(|t| t.token_id == token_id) else {
            return false;
        };
        if tok.owner_id != owner_id
            || tok.epoch != epoch
            || tok.suggestion_id != suggestion_id
            || tok.action != action
            || tok.args_hash != args_hash
        {
            return false;
        }
        if current_step > tok.issued_at_step.saturating_add(tok.ttl_steps) {
            return false;
        }
        let expected_mac = Self::compute_mac(
            key,
            &tok.token_id,
            &tok.owner_id,
            tok.epoch,
            &tok.suggestion_id,
            &tok.action,
            &tok.args_hash,
            tok.issued_at_step,
            tok.ttl_steps,
        );
        if !ct_eq(tok.mac.as_bytes(), expected_mac.as_bytes()) {
            return false;
        }
        self.consumed.insert(token_id.to_string());
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_single_use_and_bound_to_context() {
        let key = b"omega-secret-key-0123456789abcdef";
        let boundary = ActionBoundary::new(["write_file", "read_file"]);
        assert!(boundary.permits("write_file"));
        assert!(!boundary.permits("exec_shell"));

        let mut book = TokenBook::new();
        let tok = book.issue(key, "owner-1", 1, "sug-1", "write_file", "abc123", 10, 5);
        assert!(!book.consume(key, &tok.token_id, "owner-1", 1, "sug-1", "write_file", "bad", 11));
        assert!(!book.consume(key, &tok.token_id, "owner-1", 1, "sug-1", "write_file", "abc123", 16));
        assert!(book.consume(key, &tok.token_id, "owner-1", 1, "sug-1", "write_file", "abc123", 12));
        assert!(!book.consume(key, &tok.token_id, "owner-1", 1, "sug-1", "write_file", "abc123", 12));
    }
}
