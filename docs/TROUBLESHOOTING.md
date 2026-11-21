# Troubleshooting Guide

Common issues and solutions when using Omega Cognitive Kernel.

## Installation Issues

### Rust version too old

**Problem:** Build fails with errors about unsupported Rust features.

**Solution:** Ensure you have Rust 1.74 or later:
```bash
rustup update
rustc --version  # Should be >= 1.74
```

### Python binding build fails

**Problem:** `maturin develop` fails with compilation errors.

**Solution:**
1. Ensure you have the correct Rust toolchain:
   ```bash
   rustup update stable
   ```
2. Install maturin in a virtual environment:
   ```bash
   python -m venv .venv
   source .venv/bin/activate  # On Windows: .venv\Scripts\activate
   pip install maturin
   ```
3. Try building with verbose output:
   ```bash
   maturin develop --manifest-path bindings/python/Cargo.toml --release -v
   ```

## Runtime Issues

### State file permission errors

**Problem:** `Permission denied` when saving state.

**Solution:** Ensure the state directory is writable:
```bash
# Check permissions
ls -la .omega/

# Create directory with correct permissions
mkdir -p .omega
chmod 755 .omega
```

### Audit log grows too large

**Problem:** `audit.jsonl` file becomes very large over time.

**Solution:** Implement log rotation or periodically archive old logs:
```bash
# Archive current audit log
mv .omega/audit.jsonl .omega/audit-$(date +%Y%m%d).jsonl

# The kernel will create a new audit.jsonl on next run
```

### Trajectory replay shows no data

**Problem:** `replay` command shows 0 rows.

**Solution:** Ensure trajectory dumping is enabled:
```toml
# In omega.toml
dump_trajectory = true
```

Or via CLI:
```bash
omega-kernel --config omega.toml run --steps 20
```

## Configuration Issues

### TOML parsing errors

**Problem:** Config file fails to parse.

**Solution:** Validate your TOML syntax:
```bash
# Use a TOML validator or check with Rust
cargo run --example config_and_dir
```

Common issues:
- Missing quotes around strings
- Incorrect indentation (TOML uses 2 or 4 spaces)
- Invalid date/time formats

### Self-modify not working

**Problem:** L1 patches are not being applied.

**Solution:** Check that self-modify is explicitly enabled:
```toml
# In omega.toml
self_modify_enabled = true
```

**Warning:** Only enable this for deliberate experiments. See [ARCHITECTURE.md](../ARCHITECTURE.md) for safety details.

## Performance Issues

### Slow step execution

**Problem:** Each kernel step takes too long.

**Solution:**
1. Disable trajectory dumping if not needed:
   ```toml
   dump_trajectory = false
   ```
2. Reduce checkpoint frequency:
   ```toml
   auto_checkpoint_on_patch = false
   ```
3. Use release build:
   ```bash
   cargo run --release --bin omega-kernel
   ```

### High memory usage

**Problem:** Kernel uses too much memory over time.

**Solution:**
1. Limit case memory capacity in code:
   ```rust
   let mut kernel = Kernel::open(None)?;
   kernel.inner.l0.memory.case.capacity = 128; // Default is 256
   ```
2. Periodically checkpoint and restart:
   ```bash
   omega-kernel checkpoint --label "before-cleanup"
   # Restart your application
   ```

## Python-Specific Issues

### Import error: No module named 'omega_cognitive_kernel_python'

**Problem:** Cannot import the Python module.

**Solution:**
1. Ensure the package is installed:
   ```bash
   pip list | grep omega
   ```
2. Reinstall if necessary:
   ```bash
   pip install --force-reinstall omega-cognitive-kernel
   ```
3. Check you're using the correct Python environment:
   ```bash
   which python
   python -c "import omega_cognitive_kernel_python; print('OK')"
   ```

### PyO3 linking errors

**Problem:** Build fails with linking errors.

**Solution:**
1. Ensure Python development headers are installed:
   ```bash
   # Ubuntu/Debian
   sudo apt-get install python3-dev
   
   # macOS
   brew install python3
   ```
2. Set PYO3_PYTHON environment variable:
   ```bash
   export PYO3_PYTHON=$(which python3)
   maturin develop --manifest-path bindings/python/Cargo.toml
   ```

## Getting Help

If your issue is not covered here:

1. Check the [GitHub Issues](https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel/issues)
2. Review the [documentation](../README.md)
3. Enable debug logging:
   ```bash
   RUST_LOG=debug cargo run --bin omega-kernel
   ```
4. File a new issue with:
   - Your OS and Rust version
   - Minimal reproduction steps
   - Full error message
   - Relevant configuration files

## See Also

- [INTEGRATING.md](./INTEGRATING.md) - Integration guide
- [ARCHITECTURE.md](../ARCHITECTURE.md) - Architecture overview
- [CLOSED_LOOP.md](./CLOSED_LOOP.md) - Safety and best practices