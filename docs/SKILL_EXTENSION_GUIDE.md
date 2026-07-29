# Janet School Skill Extension Guide

## Purpose

This guide is for maintainers who want to add a new deterministic MCM skill to Janet School.

Right now, skills are not implemented as separate plugin files. The current build keeps the core skill logic inside the MCM engine, then layers manifest registration, approval control, session logging, and analysis on top of that.

That means adding a skill is a cross-cutting change even when the behavior itself is small.

## What Counts As A Skill In This Build

In Janet School, a skill is:

- deterministic
- narrow in scope
- inspectable in logs
- approvable or blockable
- testable without teacher dependency

A skill is not:

- a general free-form reasoning mode
- an LLM answer path
- a hidden heuristic with no trace surface

## Current Skill Architecture

The current extension surface spans a few places:

- [src/mcm/mod.rs](../src/mcm/mod.rs)
  Skill matching, execution, trace generation, and unit tests.
- [config/skill_manifest.json](../config/skill_manifest.json)
  Declares which skills exist.
- [config/skill_approvals.json](../config/skill_approvals.json)
  Declares which skills are allowed for runs.
- [src/session/mod.rs](../src/session/mod.rs)
  Converts MCM traces into session artifacts and skill events.
- [src/skills/mod.rs](../src/skills/mod.rs)
  Defines the `SkillEvent` output shape.

The important thing to understand is this:

- the manifest says a skill exists
- the approvals file says a skill may run
- the MCM engine decides whether it matches and executes
- the session layer records what happened

## The Usual Add-A-Skill Workflow

1. Define the new skill's scope and refusal boundary.
2. Add surface-feature detection or reuse an existing feature.
3. Add candidate-skill matching in the MCM engine.
4. Add the deterministic execution function.
5. Add execution and refusal trace steps.
6. Register the skill in the manifest.
7. Decide whether it should be approved by default.
8. Add or update tests.
9. Run validation and test passes.
10. Do a smoke session and inspect the artifacts.

## Step 1: Define The Skill Before You Code

Write down:

- what exact prompt pattern the skill is allowed to solve
- what it should return on success
- when it must refuse
- what nearby prompts it must not pretend to solve

If the boundary is fuzzy, the skill is not ready yet.

Good deterministic skills have a crisp envelope. They are boring in a good way.

## Step 2: Add Or Reuse Surface Feature Detection

The MCM engine classifies prompt surface features before matching candidate skills.

That happens in [src/mcm/mod.rs](../src/mcm/mod.rs).

When adding a new skill, check whether the prompt family already maps cleanly to an existing feature label. If not, add a new one.

Your goal is not just to make the skill work. Your goal is to make it legible why the skill was considered.

## Step 3: Add Candidate-Skill Matching

Inside `StudentEngine::match_candidate_skills`, map the relevant feature to the new `skill_id`.

This is the point where the engine says:

- "this prompt shape looks like this skill might apply"

It is only a candidate stage. Approval and execution come later.

## Step 4: Add The Deterministic Execution Function

The current skills are implemented as helper functions in [src/mcm/mod.rs](../src/mcm/mod.rs).

Follow the existing pattern:

- input is the prompt text
- output is `Option<String>`
- `Some(...)` means a deterministic answer was found
- `None` means the skill recognized the pattern but could not resolve a safe answer

That `None` path matters. It is what lets the engine produce a meaningful partial attempt and a principled refusal instead of bluffing.

## Step 5: Wire The Skill Into Execution Order

Inside `StudentEngine::answer`, add a new branch for the skill using the same structure as the existing ones.

Each branch should:

- set `executed_skill`
- attempt the deterministic answer
- append a success reasoning step if it resolves
- append a failure reasoning step if it does not
- set `partial_attempt` when the pattern was recognized but not resolved
- set `refusal_reason` when it cannot answer safely
- set `uncertainty_state`
- set `final_mode`

Keep the language specific. The trace should make sense to a future reviewer who did not write the code.

## Step 6: Register The Skill In The Manifest

Add a new entry to [config/skill_manifest.json](../config/skill_manifest.json).

Every entry needs:

- `skill_id`
- `description`
- `deterministic`

Example shape:

```json
{
  "skill_id": "new_skill_id",
  "description": "Deterministic description of the prompt family this skill handles.",
  "deterministic": true
}
```

If the manifest is missing the skill, the config will not truthfully represent the build even if the engine code exists.

## Step 7: Decide Approval Posture

Decide whether the skill should be approved by default in [config/skill_approvals.json](../config/skill_approvals.json).

Questions to ask:

- Is the skill already well-tested?
- Is the boundary crisp enough for baseline runs?
- Would default approval muddy earlier comparisons?

If the answer is not clear, do not approve it by default yet. Let it be opt-in through approvals.

## Step 8: Update Test Fixtures

There are two important fixture surfaces:

- the unit test engine setup in [src/mcm/mod.rs](../src/mcm/mod.rs)
- the session test config helper in [src/session/mod.rs](../src/session/mod.rs)

Both currently include explicit lists of known skill IDs.

If you add a new skill and forget those lists, tests may silently miss the new path or future test helpers may drift from the real build.

## Step 9: Add MCM Unit Tests

At minimum, add tests for:

- deterministic success on a clean example
- refusal when the skill is not approved
- refusal or partial-attempt behavior on an unresolved edge case if applicable
- stability across repeated identical inputs if the new path adds any complexity

Follow the existing tests in [src/mcm/mod.rs](../src/mcm/mod.rs) as the model.

The minimum confidence bar is:

- same input
- same output
- same trace posture

## Step 10: Check Session-Level Effects

The session layer automatically records the MCM trace into:

- `mcm_trace.jsonl`
- `skill_events.jsonl`
- `refusal_events.jsonl` when applicable
- `telemetry.jsonl`
- `analysis_report.json`
- `analysis_report.md`

Your new skill should show up clearly through:

- `candidate_skills`
- `approved_skills`
- `blocked_skills`
- `executed_skill`
- `reason`
- refusal traces if the skill was recognized but not safely resolved

You usually do not need to add a brand-new session event type for a normal deterministic skill. The existing trace structure is already designed to carry it.

## What Good Telemetry Looks Like

After a successful run using the new skill, you should be able to answer:

- why the skill became a candidate
- whether it was approved
- whether it executed
- what answer it produced
- whether the result matched the expected skill on the curriculum item

If the logs do not make that obvious, the skill is not integrated cleanly enough yet.

## Commands To Run After The Change

```powershell
cargo run -- validate-config
cargo test
cargo run -- serve-gui
```

If the skill affects local runtime assumptions, also run:

```powershell
cargo run -- inspect-runtime
```

## Recommended Manual Smoke Test

1. Keep the teacher backend on `mock`.
2. Approve only the new skill if possible.
3. Run a session with a clear name.
4. Inspect `Run`, `Telemetry`, and `Recent Sessions` in the GUI.
5. Open the new session folder under `data/sessions/`.
6. Confirm the trace and skill events mention the new `skill_id`.
7. Compare the run against a baseline or memory-only run if the result matters.

## Common Failure Modes

### The Skill Exists In Code But Never Runs

Usually means one of these:

- surface features never match
- `match_candidate_skills` does not map to the new ID
- the skill is not approved

### The Skill Runs But Never Appears In Config

Usually means:

- `skill_manifest.json` was not updated

### The Skill Is Approved But Still Refuses

Usually means:

- the deterministic parser did not actually resolve the prompt
- the prompt family is broader than the helper function can safely handle

This is not always a bug. Sometimes it is the correct boundary signal.

### Tests Pass But Session Review Is Muddy

Usually means:

- reasoning steps are too vague
- refusal reasons are too generic
- the curriculum expected skill mapping was not updated where needed

## Naming Advice

Prefer names that describe the deterministic operation, not an abstract aspiration.

Good examples from the current build:

- `left_right_selector`
- `same_different_compare`
- `option_match_selector`

Those names tell a reviewer what kind of operation happened.

## A Good Change Checklist

- skill boundary is explicit
- deterministic helper added
- candidate matching added
- execution branch added
- manifest updated
- approvals decision made
- test fixtures updated
- MCM tests added
- config validates
- tests pass
- smoke run inspected

## Final Advice

If a new skill makes Janet look stronger but makes the trace harder to interpret, that is not a good trade. In this project, legibility and clean refusal boundaries matter just as much as capability.
