# Janet School Release Checkpoint

Use this file as the short release or handoff checkpoint for the current workspace state.

For the full procedure, use [docs/WINDOWS_ACCEPTANCE.md](docs/WINDOWS_ACCEPTANCE.md).

## Build Identity

- Build label:
- Date:
- Operator:
- Machine:
- Rust version:
- Git commit:

## Core Acceptance

- [ ] `cargo test` passed
- [ ] `cargo run -- validate-config` passed
- [ ] `cargo run -- inspect-runtime` passed
- [ ] `cargo run -- sync-gui-state` passed
- [ ] `cargo run -- serve-gui` opened the shell at `http://127.0.0.1:8787`
- [ ] FMI splash showed once and cleared correctly
- [ ] GUI controls, inputs, expanders, and scroll regions behaved normally

## Session Acceptance

- [ ] Mock smoke run completed successfully
- [ ] Smoke run produced a structured curriculum, not a five-question toy
- [ ] Session artifacts were written under `data/sessions/`
- [ ] `teacher_calls.jsonl` was written separately from Janet answer traces
- [ ] `telemetry.jsonl` was written
- [ ] `mcm_trace.jsonl` was written
- [ ] `analysis_report.md` was written
- [ ] `session_summary.json` was written

## Deterministic Integrity

- [ ] `config/mcm_config.json` still sets `deterministic_only` to `true`
- [ ] Skill governance was visible in the GUI
- [ ] Refusals remained meaningful events when present
- [ ] Analysis remained conservative and used candidate language where appropriate

## Local LLM Acceptance

- [ ] Not applicable for this checkpoint
- [ ] Runtime readiness checked
- [ ] Model path present
- [ ] Server binary present
- [ ] Endpoint ready or runtime launching intentionally enabled
- [ ] Local-LLM run completed successfully
- [ ] Local-LLM teacher calls were logged correctly

## Result

- [ ] Accepted for current v0.1 checkpoint
- [ ] Accepted for mock-only use
- [ ] Accepted for mock plus local-LLM use
- [ ] Not accepted yet

## Notes

- 
