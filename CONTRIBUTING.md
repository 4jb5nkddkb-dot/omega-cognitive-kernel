# Contributing to Omega Cognitive Kernel

Thank you for your interest in contributing to Omega Cognitive Kernel! This document provides guidelines and instructions for contributing.

## Code of Conduct

Be respectful and constructive. We're building a tool for safe, auditable AI systems, and our community should reflect those values.

## Getting Started

### Prerequisites

- **Rust 1.74+** (for Rust development)
- **Python 3.10+** (for Python bindings)
- **Git**
- **Docker** (optional, for testing manylinux wheels)

### Setting Up Your Development Environment

1. **Clone the repository:**
   ```bash
   git clone https://github.com/4jb5nkddkb-dot/omega-cognitive-kernel.git
   cd omega-cognitive-kernel
   ```

2. **Install Rust toolchain:**
   ```bash
   rustup update stable
   rustup component add rustfmt clippy
   ```

3. **Set up Python environment (optional):**
   ```bash
   python -m venv .venv
   source .venv/bin/activate  # On Windows: .venv\Scripts\activate
   pip install maturin pytest
   ```

4. **Build the project:**
   ```bash
   cargo build --release
   ```

5. **Run tests:**
   ```bash
   # Rust tests
   cargo test --locked
   
   # Python tests (if working on bindings)
   maturin develop --manifest-path bindings/python/Cargo.toml --release
   python -m pytest
   ```

## Development Workflow

### 1. Create a Feature Branch

```bash
git checkout main
git pull origin main
git checkout -b feature/your-feature-name
```

Branch naming conventions:
- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation changes
- `refactor/` - Code refactoring
- `test/` - Test additions or improvements
- `ci/` - CI/CD changes

### 2. Make Your Changes

#### Code Style

**Rust:**
- Follow standard Rust conventions
- Run `cargo fmt` before committing
- Ensure `cargo clippy --locked --all-targets -- -D warnings` passes
- Add tests for new functionality
- Update documentation for public APIs

**Python:**
- Follow PEP 8
- Use type hints where appropriate
- Add docstrings to public functions
- Ensure `pytest` passes

#### Commit Messages

Use clear, descriptive commit messages following the [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>(<scope>): <subject>

<body>

<footer>
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Test additions or changes
- `chore`: Build process or auxiliary tool changes
- `ci`: CI/CD changes

Examples:
```
feat(l2): add consequence-based defensive mode

Implement defensive mode that activates after consecutive negative
consequences, reducing L2 intervention aggressiveness.

Closes #123
```

```
docs(readme): clarify version mapping

Distinguish between source version and repository tags to avoid
confusion about which version to use.
```