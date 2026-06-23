# Janet School Runbook

## Purpose

This runbook is the short practical companion to the full user manual. Use it when you already understand the basics and want concrete workflows.

If you need a formal pass/fail verification sheet instead of workflows, use [WINDOWS_ACCEPTANCE.md](WINDOWS_ACCEPTANCE.md).

Current posture:

- `mock` is the recommended first-run backend
- `local-llm` is ready when the local runtime and model path are valid
- the workspace is at a usable v0.1 research checkpoint

## Core Commands

```powershell
cargo run -- validate-config
cargo run -- inspect-runtime
cargo run -- serve-gui
cargo run -- serve-gui --no-browser
cargo run -- sync-gui-state
cargo run -- run-session --teacher-backend mock --session-name "Baseline"
```

## Workflow 1: First Baseline Run

1. Optionally run `cargo test` for a quick confidence pass.
2. Run `cargo run -- validate-config`.
3. Run `cargo run -- serve-gui`.
4. Janet School should auto-open `http://127.0.0.1:8787` in your browser when running standalone.
5. Let the FMI splash finish or skip it with click, `Space`, `Enter`, or `Escape`.
6. Leave backend on `mock`.
7. Leave all skills approved.
8. Enter a session name.
9. Click `Run Session`.
10. Review `Run`, `Telemetry`, and `Analysis`.

## Workflow 2: Memory-Only Run

1. Open the GUI.
2. Click `Deselect All` in the skill approvals panel.
3. Click `Confirm Skills`.
4. Run a new session.
5. Compare it against your baseline.

Expected use:

- refusal-pressure checks
- memory-path inspection
- isolation from deterministic skill execution

## Workflow 3: Single-Skill Isolation

1. Click `Deselect All`.
2. Re-enable one skill only.
3. Click `Confirm Skills`.
4. Run a new session.
5. Compare against baseline and memory-only runs.

Expected use:

- narrow deterministic skill testing
- debugging one reasoning path

## Workflow 4: Save A Compare Report

1. Open `Compare Runs`.
2. Pick two runs.
3. Adjust filters.
4. Choose export scope:
   `Visible items only` or `All filtered items`
5. Click `Save Markdown` or `Save JSON`.
6. Find the output in `compare_exports/`.

## Power-User GUI Host Mode

If you want Janet School to start the local GUI server without opening a browser automatically, use:

```powershell
cargo run -- serve-gui --no-browser
```

This is useful for:

- remote or scripted launches
- cases where you want to choose the browser manually
- hosted environments where another tool owns the visible shell

## Workflow 5: Runtime Readiness Check For Local LLM

1. Run `cargo run -- inspect-runtime`.
2. Open the GUI and review the `Setup` section.
3. Confirm:
   runtime path exists, server binary exists, model file exists, endpoint is correct
4. Only then switch the teacher backend to `local-llm`.

## Files Worth Looking At After A Run

- `data/sessions/<session>/session_summary.json`
- `data/sessions/<session>/analysis_report.md`
- `data/sessions/<session>/telemetry.jsonl`
- `data/sessions/<session>/mcm_trace.jsonl`
- `data/sessions/<session>/skill_events.jsonl`
- `data/sessions/<session>/refusal_events.jsonl`
- `compare_exports/<saved-compare-report>`

## If Something Looks Wrong

- No GUI:
  confirm `serve-gui` is running.
- No state:
  run `cargo run -- sync-gui-state`.
- Splash feels stuck:
  click once or press `Space`, `Enter`, or `Escape`.
- No local teacher readiness:
  fall back to `mock`.
- Skill selection mismatch:
  inspect `config/skill_approvals.json`.
- Compare save issue:
  inspect `compare_exports/` and restart the GUI host.

## UI Help

- Hover or focus interactive controls to read short tooltip help.
- Look for collapsed `More details`, `Trace details`, or similar disclosure rows when a panel looks intentionally compact.

## Recommended Session Naming

Use names that describe the condition.

Examples:

- `baseline_mock_all_skills`
- `memory_only_smoke`
- `single_skill_left_right`
- `compare_profile_visibility`

## Recommended Comparison Ladder

1. Baseline all-skills run
2. Memory-only run
3. Single-skill or small-skill-group run
4. Local teacher run if runtime is ready

That ladder makes later comparison reports much easier to interpret.

## Release Checkpoint

If you are handing the build to someone else or marking a checkpoint, use [../RELEASE_CHECKPOINT.md](../RELEASE_CHECKPOINT.md) alongside [WINDOWS_ACCEPTANCE.md](WINDOWS_ACCEPTANCE.md).
