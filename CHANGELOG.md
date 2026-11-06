# Changelog

## 0.3.8 — HTTP whitelist adapter (branch feat/http-adapter)

- **New** `src/adapters/`: `HttpToolAdapter` (GET-only, host allowlist, action map) + `WorldExecutor`.
- **Config** `adapters/http.json` targeting httpbin.org probe actions.
- **CLI** `http-probe` subcommand; `run --adapter http --adapter-config …`.
- **API** `Kernel::attach_http_adapter` / `open_with_http_adapter`.
- Default simulated path unchanged when no adapter is attached.
- Decision loop still selects classic actions (`observe`/`explore`/…); HTTP actions are executable when chosen or via `http-probe`.

## 0.3.7

See `RELEASE_NOTES_v0.3.7.md`.
