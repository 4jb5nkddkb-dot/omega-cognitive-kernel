# Vault encryption and migration

New Vault writes use **XChaCha20-Poly1305**, an authenticated-encryption-with-associated-data (AEAD) construction. `Vault::seal` and `Vault::save_sealed` generate a fresh 24-byte nonce from the operating system random source and store it in the blob; callers no longer provide a nonce. The file header and nonce are authenticated along with the ciphertext. A modified ciphertext, nonce, tag, or header is rejected during `open`.

## Existing version-1 blobs

Vault format version 1 used an HMAC-derived keystream and a separate HMAC tag. The current implementation retains a **read-only compatibility path** for authenticated v1 blobs. `Vault::load_sealed` can read them, but all new calls to `seal` or `save_sealed` write version 2. There is no automatic rewrite simply from reading a file.

To migrate a file, keep the same secret, salt, and PBKDF2 iteration count used to create its `Vault`, back up the old file, load its plaintext, and save it again. For example:

```rust
let backup = path.with_extension("v1.backup");
std::fs::copy(&path, &backup)?;
let plaintext = vault.load_sealed(&path)?; // Reads v1 or v2.
vault.save_sealed(&path, &plaintext)?;    // Writes v2 with a generated nonce.
```

Keep the backup until the new file has been reopened and verified. If the original Vault secret or KDF parameters are unavailable, the old blob cannot be migrated. Do not treat v1 as a format for new writes; its decryptor exists only to ease conversion.
