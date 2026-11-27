# Vault Encryption (`src/safety/vault.rs`)

## Current Implementation

`Vault` provides authenticated, sealed storage for sensitive data:

- **Key derivation**: `Vault::from_secret` stretches a master passphrase (`>= 16` bytes) via `PBKDF2-HMAC-SHA256` (at least 10,000 iterations) to derive a 32-byte master key, which is then domain-separated to derive the v2 AEAD key.
- **Authenticated encryption (v2)**: Newly written v2 payloads use **XChaCha20-Poly1305** AEAD encryption. Any modification to the ciphertext, nonce, header (`OMEGAVLT` + version `2`), or authentication tag is rejected during `open`.
- **Automatic 24-byte CSPRNG nonce**: `Vault::seal` and `Vault::save_sealed` internally generate a **24-byte random nonce** using the operating system CSPRNG (`getrandom::getrandom`); callers do not need to and cannot pass a nonce.
- **Legacy v1 compatibility**: `Vault::open` and `Vault::load_sealed` retain read-only compatibility for legacy v1 payloads. Calling `save_sealed` again migrates the stored payload to the v2 format.

## Security Notice

- `Vault` and its cryptographic wrapper have **not yet undergone an independent external security audit**.
