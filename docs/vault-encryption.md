# Vault Encryption (`src/safety/vault.rs`)

## Current Implementation

`Vault` uses pure-Rust, hand-written cryptographic primitives implemented in `src/safety/crypto.rs`:

- **Key derivation**: A master passphrase (`>= 16` bytes) is stretched via `PBKDF2-HMAC-SHA256` (at least 10,000 iterations) to derive a master key, which is then domain-separated to derive `enc_key` and `mac_key`.
- **Authenticated encryption**: Plaintext is encrypted using an `HMAC-SHA256` counter-mode XOR keystream followed by an **Encrypt-then-MAC** tag.
- **Explicit 16-byte nonce**: Callers must explicitly pass a **16-byte `nonce`**; `Vault` does **not** invoke the operating system random number generator internally. Any modification to the ciphertext, nonce, header, or tag is rejected during `open`.

## Security Notice

- The current source code does **not** use `XChaCha20-Poly1305`.
- The built-in cryptographic primitives in `src/safety/crypto.rs` have **not yet undergone an independent external security audit**.
- Migrating to `XChaCha20-Poly1305` with automatic OS-RNG nonce generation is only an optional future upgrade direction.
