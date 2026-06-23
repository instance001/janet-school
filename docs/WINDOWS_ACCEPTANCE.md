# Janet School Windows Acceptance

## Purpose

This document is the repeatable Windows acceptance pass for Janet School v0.1.

Use it when you want to answer a simple question:

"Does this workspace currently satisfy the standalone Windows acceptance bar from the build outline?"

This is not a deep operator manual. It is a pass/fail run sheet.

## Acceptance Scope

This pass is designed to cover the Windows-facing v0.1 requirements that matter most in practice:

- the project builds on Windows
- the standalone Rust app runs
- the GUI shell auto-opens in a browser in standalone mode
- a mock smoke run completes
- the deterministic MCM remains deterministic-only
- telemetry and reports are written
- teacher calls are logged separately
- skill governance is visible
- the local LLM teacher path has a clear readiness and execution check

## Preflight

Before starting, confirm:

- you are on Windows
- you can open PowerShell
- this repository is present locally
- Rust is installed and `cargo` works
- the project root is your current directory

Optional for local-LLM acceptance:

- a GGUF exists in `models/`
- `runtime/llama-server.exe` exists
- the configured endpoint is reachable or runtime launching is enabled

## Fast Pass Commands

Run these in order from the project root:

```powershell
cargo test
cargo run -- validate-config
cargo run -- inspect-runtime
cargo run -- sync-gui-state
cargo run -- serve-gui
```

Janet School should auto-open `http://127.0.0.1:8787` after `serve-gui` starts. If it does not, open that URL manually.

Optional power-user variant:

```powershell
cargo run -- serve-gui --no-browser
```

Use that variant only when you intentionally want the GUI server without automatic browser launch.

## Pass 1: Build And Test

Command:

```powershell
cargo test
```

Pass if:

- the command exits successfully
- the Rust suite reports all tests passing

This is the fastest check that:

- curriculum validation is working
- deterministic MCM behavior is covered
- analyzer sections are covered
- GUI state/export/save-setup behavior is covered
- local-LLM acceptance tests are covered

## Pass 2: Config Loads Cleanly

Command:

```powershell
cargo run -- validate-config
```

Pass if:

- the command exits successfully
- config JSON prints without error
- `teacher.backend` is shown
- session paths are present
- `mcm.deterministic_only` is `true`

Fail if:

- any config file is missing
- JSON parsing fails
- required paths are malformed

## Pass 3: Runtime Readiness Snapshot

Command:

```powershell
cargo run -- inspect-runtime
```

Pass if:

- the command exits successfully
- the configured runtime path, server binary path, model path, and endpoint are shown

For `mock`, `endpoint_ready` may be `false` and that is fine.

For `local-llm`, acceptance is stronger if:

- `runtime_path_exists` is `true`
- `server_binary_exists` is `true`
- `model_path_exists` is `true`
- `endpoint_ready` is `true` or runtime launching is intentionally enabled

## Pass 4: GUI State Sync

Command:

```powershell
cargo run -- sync-gui-state
```

Pass if:

- the command exits successfully
- the JSON contains current setup, skills, latest-session, analysis, and export surfaces
- the splash configuration is present
- control actions such as `run_session` and `generate_curriculum` are present

This confirms the GUI shell can be populated from backend artifacts.

## Pass 5: GUI Opens

Command:

```powershell
cargo run -- serve-gui
```

Then confirm Janet School opens:

```text
http://127.0.0.1:8787
```

Pass if:

- the browser opens automatically in standalone mode, or the page can be opened manually if auto-launch is blocked
- the FMI splash appears once at startup
- the splash clears automatically after about 3000 ms or can be skipped
- the Janet School interface loads afterward
- panels render without broken layout
- dropdowns, inputs, expanders, and scroll regions behave normally
- tooltips appear on hover or focus for interactive controls

Fail if:

- the page does not load
- the splash never clears
- the shell loads with missing state and cannot recover with `Sync State`

## Pass 6: Mock Smoke Run

Recommended CLI proof:

```powershell
cargo run -- run-session --teacher-backend mock --session-name "windows_acceptance_mock"
```

You can also perform this through the GUI.

Pass if:

- the run completes successfully
- a new session folder appears under `data/sessions/`
- the smoke run produces at least:
  - `session_config.json`
  - `curriculum_generated.jsonl`
  - `curriculum_validated.jsonl`
  - `teacher_calls.jsonl`
  - `interactions.jsonl`
  - `mcm_trace.jsonl`
  - `telemetry.jsonl`
  - `skill_events.jsonl`
  - `refusal_events.jsonl`
  - `analysis_report.json`
  - `analysis_report.md`
  - `session_summary.json`

Also confirm:

- the mock run is not a five-question toy
- the smoke profile still produces a structured curriculum and probes
- skill approvals are visible in the GUI

## Pass 7: Deterministic MCM Integrity

Use either the test suite result or direct artifact inspection.

Minimum pass evidence:

- `cargo test` passes the MCM deterministic-only tests
- `config/mcm_config.json` keeps `deterministic_only` set to `true`
- the run completes without any teacher-assisted Janet answering path

Strong manual evidence:

- `mcm_trace.jsonl` and `telemetry.jsonl` show explicit reasoning steps
- teacher calls are logged in `teacher_calls.jsonl` separately from Janet answers

## Pass 8: Local-LLM Teacher Acceptance

This pass is only required when you intend to use the local model path on that machine.

### Readiness Check

1. Run `cargo run -- inspect-runtime`.
2. Confirm runtime, server binary, and model file exist.
3. Confirm the endpoint is correct for your local runtime.

### CLI Or GUI Run

Use either:

```powershell
cargo run -- run-session --teacher-backend local-llm --session-name "windows_acceptance_local_llm"
```

or the GUI with backend switched to `local-llm`.

Pass if:

- curriculum generation succeeds
- a session folder is written
- `teacher_calls.jsonl` records `teacher_backend_id` as `local_llm`
- local-teacher artifacts and analysis outputs are written

Fail if:

- runtime launch is disabled and the endpoint is not reachable
- the model path is missing
- the server binary is missing
- teacher calls are not logged

## Artifact Review Checklist

For the newest accepted run, inspect:

- `data/sessions/<latest>/session_summary.json`
- `data/sessions/<latest>/analysis_report.md`
- `data/sessions/<latest>/telemetry.jsonl`
- `data/sessions/<latest>/teacher_calls.jsonl`
- `data/sessions/<latest>/mcm_trace.jsonl`
- `data/sessions/<latest>/skill_events.jsonl`
- `data/sessions/<latest>/refusal_events.jsonl`

You are looking for:

- visible skill governance
- separate teacher-call logging
- telemetry written per interaction
- conservative analysis language
- confirmed, boundary, and emergent analysis sections

## Acceptance Record Template

Copy this into a note, issue, or release checkpoint:

```text
Janet School Windows Acceptance
Date:
Machine:
Rust version:

[ ] cargo test passed
[ ] validate-config passed
[ ] inspect-runtime passed
[ ] sync-gui-state passed
[ ] serve-gui auto-opened or served at http://127.0.0.1:8787
[ ] FMI splash showed once and cleared correctly
[ ] mock smoke run completed
[ ] session artifacts written
[ ] deterministic-only posture confirmed
[ ] teacher calls logged separately
[ ] skill governance visible
[ ] local-llm readiness checked
[ ] local-llm run completed (if applicable)

Notes:
```

## Current v0.1 Interpretation

If Pass 1 through Pass 7 succeed on Windows, Janet School meets the practical standalone v0.1 acceptance bar for the mock-backed research rig.

If Pass 8 also succeeds, the local-LLM teacher path is accepted on that machine as well.
