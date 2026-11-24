#[allow(unused_imports)]
use crate::prelude::*;

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

use crate::error::{OmegaError, Result};

use super::crypto::{ct_eq, hmac_sha256, pbkdf2_hmac_sha256};

const VAULT_MAGIC: &[u8; 8] = b"OMEGAVLT";
const VAULT_VERSION: u8 = 1;
const HEADER_LEN: usize = 8 + 1 + 16;
const TAG_LEN: usize = 32;

#[derive(Debug, Clone)]
pub struct Vault {
    enc_key: [u8; 32],
    mac_key: [u8; 32],
}

impl Vault {
    pub fn from_secret(secret: &str, salt: &[u8], iterations: u32) -> Result<Self> {
        if secret.len() < 16 {
            return Err(OmegaError::Config(
                "vault secret must be at least 16 bytes".into(),
            ));
        }
        let master = pbkdf2_hmac_sha256(secret.as_bytes(), salt, iterations.max(10_000));
        let enc_key = hmac_sha256(&master, b"omega-vault-enc-v1");
        let mac_key = hmac_sha256(&master, b"omega-vault-mac-v1");
        Ok(Self { enc_key, mac_key })
    }

    fn keystream_xor(&self, nonce: &[u8; 16], data: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(data.len());
        let mut counter: u32 = 0;
        for chunk in data.chunks(32) {
            let mut input = [0u8; 20];
            input[..16].copy_from_slice(nonce);
            input[16..20].copy_from_slice(&counter.to_be_bytes());
            let block = hmac_sha256(&self.enc_key, &input);
            for (b, k) in chunk.iter().zip(block.iter()) {
                out.push(*b ^ *k);
            }
            counter = counter.wrapping_add(1);
        }
        out
    }

    pub fn seal(&self, plaintext: &[u8], nonce: &[u8; 16]) -> Vec<u8> {
        let ciphertext = self.keystream_xor(nonce, plaintext);
        let mut payload = Vec::with_capacity(HEADER_LEN + ciphertext.len() + TAG_LEN);
        payload.extend_from_slice(VAULT_MAGIC);
        payload.push(VAULT_VERSION);
        payload.extend_from_slice(nonce);
        payload.extend_from_slice(&ciphertext);
        let tag = hmac_sha256(&self.mac_key, &payload);
        payload.extend_from_slice(&tag);
        payload
    }

    pub fn open(&self, blob: &[u8]) -> Result<Vec<u8>> {
        if blob.len() < HEADER_LEN + TAG_LEN {
            return Err(OmegaError::Persist("vault blob truncated".into()));
        }
        if &blob[..8] != VAULT_MAGIC || blob[8] != VAULT_VERSION {
            return Err(OmegaError::Persist("invalid vault header".into()));
        }
        let split = blob.len() - TAG_LEN;
        let (body, tag) = blob.split_at(split);
        let expected_tag = hmac_sha256(&self.mac_key, body);
        if !ct_eq(tag, &expected_tag) {
            return Err(OmegaError::Persist("vault MAC verification failed".into()));
        }
        let mut nonce = [0u8; 16];
        nonce.copy_from_slice(&body[9..HEADER_LEN]);
        Ok(self.keystream_xor(&nonce, &body[HEADER_LEN..]))
    }

    pub fn save_sealed(&self, path: &Path, plaintext: &[u8], nonce: &[u8; 16]) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| OmegaError::Persist(e.to_string()))?;
        }
        let blob = self.seal(plaintext, nonce);
        fs::write(path, blob).map_err(|e| OmegaError::Persist(e.to_string()))
    }

    pub fn load_sealed(&self, path: &Path) -> Result<Vec<u8>> {
        let blob = fs::read(path).map_err(|e| OmegaError::Persist(e.to_string()))?;
        self.open(&blob)
    }
}

#[derive(Debug)]
pub struct StorageLock {
    lock_path: PathBuf,
}

impl StorageLock {
    pub fn acquire(lock_path: impl Into<PathBuf>) -> Result<Self> {
        let lock_path = lock_path.into();
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).map_err(|e| OmegaError::Persist(e.to_string()))?;
        }
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .map_err(|e| OmegaError::ExecutionBlocked(format!("storage lock busy: {e}")))?;
        Ok(Self { lock_path })
    }
}

impl Drop for StorageLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_roundtrip_and_tamper_detection() {
        let vault = Vault::from_secret("super-secret-passphrase-2026", b"omega-salt", 10_000)
            .expect("vault init");
        let nonce = [7u8; 16];
        let mut blob = vault.seal(b"omega-state-payload", &nonce);
        assert_eq!(vault.open(&blob).expect("open"), b"omega-state-payload");
        blob[HEADER_LEN] ^= 0x01;
        assert!(vault.open(&blob).is_err());
    }

    #[test]
    fn storage_lock_prevents_concurrent_acquire() {
        let mut path = std::env::temp_dir();
        path.push("omega-kernel-test-storage.lock");
        let _ = fs::remove_file(&path);

        let guard = StorageLock::acquire(&path).expect("first acquire");
        assert!(StorageLock::acquire(&path).is_err());
        drop(guard);
        assert!(StorageLock::acquire(&path).is_ok());
        let _ = fs::remove_file(&path);
    }
}
