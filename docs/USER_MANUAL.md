# Janet School User Manual

## Who This Is For

This manual is for a person who has never used Janet School before and should not be expected to already know Rust, MCMs, LLM runtimes, local model hosting, or this project's research vocabulary.

## Plain-English Summary

Janet School is a local research tool.

It lets you:

- run a deterministic student called Janet
- generate or load curriculum items
- watch how Janet answers
- record what happened in detail
- compare different runs
- save reports for later review

It does not try to be a classroom product. It is better to think of it as a lab console than an app store app.

## Current Status

Janet School is currently at a usable v0.1 research checkpoint.

- the `mock` teacher path is working and is the recommended first-run path
- the `local-llm` teacher path is wired and ready when a valid local runtime and model are present
- the GUI, telemetry, analysis, compare/export surfaces, and acceptance docs are in place
- remaining work is mostly optional post-v0.1 compatibility or extension work

## Important Terms

- `Janet`
  The deterministic MCM student being tested.
- `MCM`
  In this project, the MCM is the rule-governed student core. It answers through explicit memory, approved deterministic skills, refusal policy, and logged reasoning traces. It is the subject under study.
- `Teacher backend`
  The system that helps generate curriculum or advisory material. In the current build this can be `mock` or `local-llm`.
- `Mock teacher`
  A deterministic, safe starting backend for testing the rig.
- `Local LLM teacher`
  A local model-backed teacher path that depends on a working runtime and model file.
- `Skill`
  A deterministic answer procedure Janet is allowed to use for a narrow kind of problem.
- `Skill approval`
  A governance setting that decides which deterministic skills Janet may use in future runs.
- `Memory-only run`
  A run where all skills are turned off, so Janet can only rely on explicit memory and refusal behavior.
- `Telemetry`
  The detailed logs and event records written during a run.
- `Boundary signal`
  Evidence that Janet may be close to a useful structure but is still constrained or inconsistent.
- `Emergent candidate`
  A possible reusable pattern that needs more testing and human review. It is not proof.

## Before You Start

You need the following:

- this project folder on your machine
- Rust installed if you want to run from source
- a terminal such as PowerShell
- a web browser

Optional but useful:

- a local model file in `models/`
- a local runtime binary in `runtime/`

If you are new, you do not need the local LLM path yet. Start with `mock`.

## Project Layout

- `config/`
  Main configuration files.
- `data/sessions/`
  One folder per run or session.
- `data/aggregated/`
  Cross-session or higher-level outputs.
- `compare_exports/`
  Compare reports saved from the GUI.
- `docs/`
  Documentation.
- `models/`
  Local model files.
- `runtime/`
  Local runtime binaries.
- `web/`
  Browser UI files used by the local GUI server.

## First-Time Setup

### Step 1: Open The Project In A Terminal

Change into the project root folder.

Example:

```powershell
cd C:\path\to\your\janet-school
```

### Step 2: Check That The Config Loads

Optional confidence check first:

```powershell
cargo test
```

This is the quickest way to confirm the current workspace still passes the Rust acceptance suite before you start operating it.

Then run:

```powershell
cargo run -- validate-config
```

What this does:

- confirms the config files can be loaded
- prints the resolved config

If this fails, fix the config files in `config/` before doing anything else.

### Step 3: Check Runtime Status

Run:

```powershell
cargo run -- inspect-runtime
```

What this does:

- shows runtime-related configuration
- helps you verify where the local runtime and endpoint are expected

This is mostly for the `local-llm` path. If you are using `mock`, it is okay if the endpoint is not ready.

## Starting The GUI

### Step 1: Start The Local GUI Host

Run:

```powershell
cargo run -- serve-gui
```

By default this serves the GUI at:

```text
http://127.0.0.1:8787
```

In standalone mode, Janet School should open that URL in your browser automatically.

If the browser does not open on its own, you can still open:

```text
http://127.0.0.1:8787
```

When Janet School is launched from ChattyCog as a hosted module, the GUI stays inside ChattyCog and does not open a separate browser tab.

Power-user option:

```powershell
cargo run -- serve-gui --no-browser
```

Use this when you want Janet School to serve the GUI without opening a browser automatically.

By default, a short FMI splash appears once at startup before the main Janet School interface is revealed.

It:

- lasts about 3 seconds
- can be skipped with click, `Space`, `Enter`, or `Escape`
- does not require any action to continue

You should see the Janet School Research Console Shell.

If you want a formal machine-level pass/fail checklist instead of the general operator flow, use [WINDOWS_ACCEPTANCE.md](WINDOWS_ACCEPTANCE.md).

## What You See In The GUI

The GUI now includes lightweight tooltip help on interactive controls.

If you hover a button, input, select box, checkbox, or disclosure label, a short in-tool help snippet should appear.

### Controls

This is where you:

- choose the teacher backend
- set a session name
- trigger major actions
- manage MCM skill approvals
- watch recent bridge jobs

### Setup

This shows:

- current environment
- configured teacher backend
- session and aggregate folders
- runtime and model detection
- endpoint path and readiness

### Curriculum

This shows:

- domain count
- concept count
- item count
- probe count
- sample items
- domain overview

### Run

This shows:

- latest run id
- completion time
- skill profile used
- item and probe totals
- correctness summary
- refusal and anomaly counts

### Live Host Job

This shows active host-side run or control activity such as:

- running
- paused
- stopping

### Telemetry

This shows a recent evidence trail, including:

- prompts
- Janet answers
- executed skill
- approved and blocked skills
- policy checks
- anomaly and refusal context

### Analysis

This shows:

- confirmed signals
- boundary signals
- emergent candidates
- caution notes
- recommended next probes

### Export

This shows:

- the session folder path
- links to current session artifacts

### Recent Sessions

This shows a card view of recent runs, including:

- skill profile
- item counts
- accuracy
- signals
- top artifact links

### Compare Runs

This lets you:

- choose two runs
- compare high-signal deltas
- filter overlapping items
- save compare reports into `compare_exports/`

### About

This shows:

- publisher and steward summary
- public GitHub identity
- license posture

It is intentionally collapsed by default so it does not distract from research work.

## Your First Safe Run

Use this exact order if you are new.

1. Start the GUI with `cargo run -- serve-gui`.
2. Let Janet School auto-open `http://127.0.0.1:8787`, or open it yourself if needed.
3. Leave teacher backend set to `mock`.
4. Enter a clear session name.
5. Leave all skills enabled for the first run.
6. Click `Run Session`.
7. Wait for the run to complete.
8. Review `Run`, `Telemetry`, `Analysis`, and `Recent Sessions`.

Why this is the safest first workflow:

- `mock` avoids runtime issues
- the all-skills profile gives you a baseline
- you can compare stricter skill profiles later

## Skill Approvals

Skill approvals are one of the most important controls in Janet School.

They determine which deterministic skills Janet is allowed to use in future runs.

### What The Buttons Mean

- `Select All`
  Approves all listed skills in the current UI selection.
- `Deselect All`
  Turns off all listed skills in the current UI selection.
- `Confirm Skills`
  Saves the current selection into `config/skill_approvals.json`.

### Why You Would Change Skill Approvals

- to run a memory-only session
- to isolate one skill
- to compare one skill group against another
- to test whether success depends on a specific deterministic path

### Examples

- All skills enabled:
  Good baseline run.
- One skill enabled:
  Good for narrow triangulation.
- Several related skills enabled:
  Good for cluster testing.
- No skills enabled:
  Memory-only and refusal-pressure run.

## Running Different Kinds Of Sessions

### Baseline Run

Use:

- `mock`
- all skills enabled

Best for:

- checking that the rig is working
- building a reference session

### Memory-Only Run

Use:

- `mock` or `local-llm`
- all skills deselected
- click `Confirm Skills`
- run a new session

Best for:

- seeing what Janet can do without deterministic skill execution
- surfacing refusal behavior
- separating memory from skill use

### Single-Skill Run

Use:

- deselect all skills
- enable only one skill
- confirm
- run a new session

Best for:

- testing a specific deterministic path

### Local LLM Teacher Run

Use only after you are comfortable with the mock path.

You need:

- a model file in `models/`
- a runtime binary in `runtime/`
- a reachable local endpoint

If runtime readiness is unclear, check `Setup` and run:

```powershell
cargo run -- inspect-runtime
```

## Understanding Outputs

Each session writes a folder under `data/sessions/`.

Important files include:

- `session_config.json`
- `curriculum_generated.jsonl`
- `teacher_calls.jsonl`
- `interactions.jsonl`
- `mcm_trace.jsonl`
- `telemetry.jsonl`
- `memory_events.jsonl`
- `skill_events.jsonl`
- `refusal_events.jsonl`
- `anomaly_events.jsonl`
- `analysis_report.json`
- `analysis_report.md`
- `session_summary.json`

### What These Files Are For

- Curriculum files:
  What was generated for the run.
- Teacher call files:
  What the teacher backend was asked to do.
- Interaction and trace files:
  What Janet received, did, and answered.
- Memory and skill files:
  How deterministic behavior was governed.
- Refusal and anomaly files:
  Where things became interesting or problematic.
- Analysis report:
  A conservative post-run interpretation.

## Comparing Runs

The compare panel is for asking questions like:

- what changed when I removed a skill?
- what changed when I ran memory-only?
- what changed between two different backends?

### Basic Compare Workflow

1. Open `Compare Runs`.
2. Pick a primary run.
3. Pick a secondary run.
4. Review `High-Signal Deltas`.
5. Use filters to narrow overlap items.
6. Save the comparison into `compare_exports/`.

### Filter Meaning

- `Changed only`
  Show only items where the two runs differ in meaningful outcome fields.
- `Refusals only`
  Show only items involving refusal behavior.
- `Anomalies only`
  Show only items with anomaly flags.
- `Domain`
  Limit comparison to one domain.
- `Item type`
  Limit comparison to a type such as teaching or probe items.

### Export Scope Meaning

- `Visible items only`
  Saves only the current visible compare slice.
- `All filtered items`
  Saves the full filtered compare set, even if the UI only displays a subset.

### Where Compare Exports Go

Saved compare reports land in:

```text
compare_exports/
```

This makes the workflow portable across machines and removable drives.

## Pause, Resume, And Stop

When a run is active, the GUI can request:

- pause
- resume
- stop

### What Stop Means

Stop is intended to preserve partial work rather than throwing it away. Partial artifacts may still be written for review.

## Troubleshooting

### The GUI Does Not Load

Check:

- that `cargo run -- serve-gui` is still running
- that `http://127.0.0.1:8787` is open, either from Janet School's auto-launch or manually
- that another app is not already using port `8787`

### The GUI Loads But Looks Empty

Try:

- refreshing the page
- running `cargo run -- sync-gui-state`
- checking whether a previous session exists in `data/sessions/`

### The Splash Stays Up Too Long

Try:

- clicking once
- pressing `Space`, `Enter`, or `Escape`
- refreshing the page if the GUI state failed to load underneath it

The splash should not be used as a long loading gate.

### The Local LLM Path Is Not Ready

Check:

- `config/teacher_config.json`
- whether `runtime/llama-server.exe` exists
- whether the model file path is correct
- whether the configured endpoint is reachable

Start with `mock` if you are blocked.

### Skill Changes Did Not Stick

Make sure you clicked `Confirm Skills`.

The saved approvals live in:

```text
config/skill_approvals.json
```

### Compare Export Did Not Save

Check:

- that the GUI host is running the latest build
- that `compare_exports/` exists
- that the browser bridge is connected

The host should save compare exports into the workspace rather than a machine-specific downloads folder.

## Good Operating Habits

- Name sessions clearly.
- Keep one baseline run with all skills enabled.
- Save compare reports after interesting deltas.
- Use memory-only and restricted-skill runs deliberately.
- Read analysis conservatively.
- Treat anomalies as valuable evidence, not noise to clean up.

## Release And Handoff

If you are preparing a build checkpoint for someone else, use:

- [../RELEASE_CHECKPOINT.md](../RELEASE_CHECKPOINT.md)
  Short release or handoff checklist
- [WINDOWS_ACCEPTANCE.md](WINDOWS_ACCEPTANCE.md)
  Full Windows acceptance run sheet

## How To Think About Results

A good Janet School operator does not ask:

- "Did we prove abstraction?"

A better question is:

- "What happened, how stable was it, what changed across conditions, and what should we probe next?"

That mindset is the whole point of the rig.
