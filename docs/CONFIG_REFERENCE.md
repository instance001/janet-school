# Janet School Config Reference

## Purpose

This document explains the files inside `config/` in plain language.

Use it when:

- you want to know what a config file does
- `cargo run -- validate-config` fails
- you want to safely change a path or default
- you want to understand what the GUI is reading

This is an operator and maintainer reference. For first-time usage, read [USER_MANUAL.md](USER_MANUAL.md) first.

## Config Folder Overview

The project currently loads these files from `config/`:

- `app_config.json`
- `teacher_config.json`
- `mcm_config.json`
- `skill_manifest.json`
- `skill_approvals.json`

The CLI loads them together as one application config set. If one file is missing or malformed, config validation fails.

## Safe Workflow Before Editing Config

1. Change one file at a time.
2. Keep the JSON valid.
3. Run `cargo run -- validate-config`.
4. If the change affects runtime or GUI behavior, also run `cargo run -- inspect-runtime` or `cargo run -- sync-gui-state`.

## File: `app_config.json`

This file defines the app identity and several default workspace paths.

Current example:

```json
{
  "app_name": "Janet School",
  "version": "0.1.0",
  "environment": "local",
  "docs_dir": "docs",
  "data_dir": "data",
  "web_dir": "web",
  "show_splash": true,
  "splash_duration_ms": 3000,
  "session": {
    "default_run_mode": "smoke",
    "sessions_dir": "data/sessions",
    "aggregated_dir": "data/aggregated",
    "curriculum_size_hint": "smoke"
  }
}
```

### Top-Level Fields

- `app_name`
  Human-readable project name.
- `version`
  Configured app version string.
- `environment`
  Environment label shown in the GUI setup view. In this workspace it is `local`.
- `docs_dir`
  Path to the docs folder.
- `data_dir`
  Path to the main data folder.
- `web_dir`
  Path to the browser UI assets.
- `show_splash`
  Whether the FMI startup splash is shown before the main GUI appears.
- `splash_duration_ms`
  Default splash duration in milliseconds before the GUI reveals itself automatically.

### `session` Fields

- `default_run_mode`
  Default run posture for sessions.
  Allowed values:
  `smoke`, `full`, `analysis_only`
- `sessions_dir`
  Where per-session outputs are written.
- `aggregated_dir`
  Where cross-session or rolled-up outputs are written.
- `curriculum_size_hint`
  The intended curriculum scale.
  Allowed values:
  `tiny_fixture`, `smoke`, `full`

### Practical Meaning

- `smoke`
  Good default for quick proof-of-life runs.
- `full`
  Intended for larger or more complete runs.
- `analysis_only`
  Reserved for analysis-oriented operation rather than a normal session path.
- `tiny_fixture`
  Very small curriculum size. The GUI treats this as below normal smoke expectations.

### Important Notes

- If `default_run_mode` is `full` while `curriculum_size_hint` is still `smoke`, the GUI reports a warning because those settings pull in different directions.
- Changing `sessions_dir` or `aggregated_dir` changes where artifacts land, so do that carefully if you already have existing data.
- `show_splash: true` with `splash_duration_ms: 3000` is the current default startup behavior.
- The splash can still be skipped early from the GUI with click or keyboard input.

## File: `teacher_config.json`

This file controls the teacher backend and local runtime/model details.

Current example:

```json
{
  "backend": "mock",
  "runtime": {
    "enabled": true,
    "runtime_path": "runtime",
    "server_binary": "runtime/llama-server.exe",
    "endpoint": "http://127.0.0.1:8080/v1"
  },
  "local_model": {
    "model_path": "models/gpt-oss-20b-heretic-q4_k_m.gguf",
    "context_size": 8192,
    "gpu_layers": 999
  }
}
```

### `backend`

Which teacher path the config prefers by default.

Allowed values:

- `mock`
- `local_llm`

### `runtime` Fields

- `enabled`
  Whether runtime launching is allowed by config.
- `runtime_path`
  Path to the runtime folder.
- `server_binary`
  Path to the server executable.
- `endpoint`
  HTTP endpoint the local teacher path expects.

### `local_model` Fields

- `model_path`
  Path to the GGUF model file.
- `context_size`
  Requested context window size for the local model runtime.
- `gpu_layers`
  Requested GPU layer count for the runtime.

### Practical Meaning

- `mock`
  Best starting backend for testing the rig itself.
- `local_llm`
  Uses the local runtime and model path, so it depends on the runtime setup being valid.

### GUI Readiness Checks

The GUI setup snapshot checks:

- whether `runtime_path` exists
- whether `server_binary` exists as a file
- whether `model_path` exists as a file
- whether the configured `endpoint` appears reachable

If `backend` is `local_llm`, the endpoint is unavailable, and `runtime.enabled` is `false`, the GUI raises a warning because it cannot launch and it cannot connect.

### Safe Changes

Safe common edits include:

- switching `backend` between `mock` and `local_llm`
- updating `model_path` when you move or replace the GGUF
- updating `endpoint` if your runtime serves on a different port

After changes, run:

```powershell
cargo run -- validate-config
cargo run -- inspect-runtime
```

## File: `mcm_config.json`

This file defines the identity and policy posture of the Janet student core.

Current example:

```json
{
  "class_label": "MCM",
  "deterministic_only": true,
  "refusal_mode": "strong",
  "memory_store": "explicit_structured",
  "policy_version": "0.1.0"
}
```

### Fields

- `class_label`
  Human-readable label for the student core.
- `deterministic_only`
  Whether the student is constrained to deterministic behavior only.
- `refusal_mode`
  Label for refusal posture.
- `memory_store`
  Label for the memory system type.
- `policy_version`
  Version marker for the current MCM policy.

### Practical Meaning

This file is about governance and identity more than file paths.

For the current build:

- `deterministic_only: true` matches the research goal
- `refusal_mode: "strong"` reflects a conservative refusal posture
- `memory_store: "explicit_structured"` signals that memory is intended to be explicit and inspectable

### Caution

Treat this file as a research-governance surface, not a cosmetic label file. Changing it may affect how future operators interpret the legitimacy of a run.

## File: `skill_manifest.json`

This file lists the deterministic skills Janet knows about.

Current example categories include:

- exact match lookup
- ordered relation compare
- same or different compare
- first or last selector
- more or less compare
- equal compare
- left or right selector
- option match selector
- inside or outside selector

### Fields

- `manifest_version`
  Version marker for the skill catalog.
- `skills`
  Array of skill entries.

Each skill entry contains:

- `skill_id`
  Stable identifier used by config and telemetry.
- `description`
  Plain-language statement of what the skill does.
- `deterministic`
  Whether the skill is deterministic.

### Important Distinction

This file does not mean the skills are approved for use.

It only defines which skills exist in the current build.

Approval is handled separately in `skill_approvals.json`.

### When To Edit It

Edit this file when:

- adding a new deterministic skill to the build
- renaming a skill identifier across the system
- updating the description for clarity

Do not edit this file just to turn a skill on or off for a run. Use the approvals file or the GUI approval controls for that.

## File: `skill_approvals.json`

This file controls which known skills are allowed during runs.

Current example:

```json
{
  "approvals_version": "0.1.0",
  "approved_skill_ids": [
    "equal_compare",
    "exact_match_lookup",
    "first_last_selector",
    "inside_outside_selector",
    "left_right_selector",
    "more_less_compare",
    "option_match_selector",
    "ordered_relation_compare",
    "same_different_compare"
  ],
  "blocked_skill_ids": []
}
```

### Fields

- `approvals_version`
  Version marker for the active approvals policy.
- `approved_skill_ids`
  Skills Janet may execute.
- `blocked_skill_ids`
  Skills explicitly blocked.

### Practical Meaning

This is the live control surface for:

- all-skills runs
- memory-only runs
- single-skill runs
- small-skill-cluster runs

### Important Behavior

- The GUI `Select All`, `Deselect All`, and `Confirm Skills` flow writes this file.
- If all skills are deselected and confirmed, you have a memory-only run condition.
- This file is separate from the manifest on purpose:
  one file says what exists, the other says what is allowed.

### Safe Editing Advice

For normal operation, use the GUI instead of hand-editing this file.

If you do hand-edit it:

1. Keep only valid skill IDs from `skill_manifest.json`.
2. Validate config after saving.
3. Re-sync GUI state if needed with `cargo run -- sync-gui-state`.

## Common Commands For Config Work

```powershell
cargo run -- validate-config
cargo run -- inspect-runtime
cargo run -- sync-gui-state
```

## Common Failure Cases

### Invalid JSON

Symptoms:

- `validate-config` fails immediately

Fix:

- check commas, quotes, brackets, and trailing syntax mistakes

### Missing Runtime Or Model Path

Symptoms:

- GUI setup warnings
- local LLM path not ready

Fix:

- verify `runtime_path`, `server_binary`, and `local_model.model_path`

### Skill Selection Looks Wrong

Symptoms:

- the GUI does not reflect the run profile you expected

Fix:

- inspect `skill_approvals.json`
- confirm your last selection was saved
- run `cargo run -- sync-gui-state`

## Which File Should I Change?

- Change `app_config.json` for app paths and session defaults.
- Change `teacher_config.json` for backend, runtime, endpoint, and model settings.
- Change `mcm_config.json` for MCM identity and policy posture.
- Change `skill_manifest.json` for the catalog of available skills.
- Change `skill_approvals.json` for which skills are allowed in runs.

## Final Advice

If you are unsure whether a change is operational or research-sensitive, assume it is research-sensitive until proven otherwise. Validate first, run a small session second, and only then use the new config for comparison work you care about.
