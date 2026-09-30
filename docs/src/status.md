# Status

**beta** — core works and is dogfooded in anger; surface may shift before stable.

## Implemented

| Command | Status | Notes |
|---|---|---|
| `vampiro prove` | stable | cross-language composition proofs (call/module/effect/law/retry/resource/trust boundaries) |
| `vampiro check` | stable | incremental composition checks |

## In progress

- Docs conformance round (DDL-u8x epic); law/effect boundary expansion.

## Mapped to specs

- `openspec/` proposals validated strict in CI; exported issue graph checked by `scripts/check_planning.py`; epistemic gate via `dont check` (vampiro-bf6).

## Dogfooding

- Runs dont (gate) + openspec (strict) + planned pretender/wai/ddl installs per `versions.ddl.toml`
- vampiro's own CI runs `just ci` (fmt, clippy, tests, build-release)
