# Security Policy

## Scope

Omega Cognitive Kernel is a research prototype for auditable, online strategy adaptation. Security reports are especially valuable for issues that could cause the runtime to execute unintended code, bypass policy or tool gates, corrupt or lose persisted state, disclose sensitive state through audit/replay output, or apply an unbounded or unauthorized policy change.

The public core is designed around a bounded `RuleEffect` policy surface. L1 self-modification is disabled by default, checkpoints and rollback are available, and audit/trajectory persistence is intended to make transitions inspectable. These safeguards reduce risk but do not make an embedding application secure by themselves.

## Reporting a vulnerability

Please do not open a public GitHub issue for an unpatched security vulnerability or include sensitive reproduction details in a public pull request.

Use GitHub's **Private Vulnerability Reporting** or **Security Advisories** feature for this repository when it is available. If private reporting is not enabled, contact the repository maintainers privately through the GitHub account that owns the repository and request a confidential security review. Do not send credentials, production data, or other secrets in the initial report.

A useful report should include:

- A concise description of the impact and affected component.
- The affected commit, release, tag, feature flags, operating system, Rust/Python version, and integration mode.
- Minimal reproduction steps or a self-contained proof of concept.
- Whether the issue requires custom observers/executors, network access, GUI mode, async mode, or a particular state file.
- Relevant logs, audit events, error messages, or sanitized state excerpts.
- Any suggested mitigation or whether the issue is already publicly known.

Please allow maintainers reasonable time to investigate and prepare a fix before public disclosure. Do not use a vulnerability report to test against systems or data that you do not own or have permission to assess.

## Security boundaries for integrators

The kernel cannot enforce the complete security policy of its host application. Integrators remain responsible for:

- Authentication, authorization, and access control around state, audit, checkpoint, and dashboard files.
- File-system permissions and protection of `state.json`, `audit.jsonl`, `trajectory.jsonl`, and `checkpoints/`.
- Network egress, URL validation, SSRF prevention, TLS configuration, and HTTP response limits for network adapters.
- The allow-list and sandbox policy for tools and executors.
- Sanitizing observations, task goals, external rewards, and other untrusted input.
- Preventing sensitive information from being written to audit or trajectory records.
- Isolation of separate tenants, workspaces, or kernel instances.

The default `SandboxedExecutor` and the structured `RuleEffect` are safety mechanisms, not a replacement for host-level sandboxing or least-privilege execution.

## Supported versions and fixes

The repository is a research prototype and may contain unreleased changes on `main`. When reporting an issue, include the exact commit or package version. Security fixes may be released as a patch version, documented in `CHANGELOG.md`, and backported only when maintainers determine that the affected release is still appropriate to support.

Until a fix is available, users should prefer the default `self_modify_enabled = false`, keep L2 interventions disabled when a fully frozen policy surface is required, restrict executor/tool capabilities, protect persisted files, and use checkpoints plus audit/replay for operational review.

## Public disclosure

After a fix or mitigation is available, maintainers may publish a coordinated advisory describing the impact, affected versions, mitigation, and fixed versions. Reporter credit is given when requested and when doing so does not disclose private information.
