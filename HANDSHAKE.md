# Janet School - Handshake

## Module identity

- **module_id**: `janet_school`
- **display_name**: `Janet School`

## What this module is for

Janet School is a standalone research rig for studying a deterministic MCM student inside a controlled curriculum, telemetry, and analysis scaffold. When hosted by ChattyCog, it keeps owning its own backend and UI while reporting a concise research-state summary back through the bridge.

## Inputs this module expects

- Session name and run condition
- Teacher backend choice (`mock` first, `local-llm` when runtime/model are ready)
- Skill approval profile for the current run
- Optional runtime/model path adjustments inside Janet School's own setup surface
- Optional exported compare or session artifacts for later cross-module discussion

## Outputs this module produces

- A completed or partial Janet School session with telemetry and analysis artifacts
- Conservative run summaries and compare exports
- A bridge rundown that lets ChattyCog understand latest run state, refusal/anomaly counts, and next likely operator action

## Operating rules / preferences

- Tone: research-oriented
- Risk level: medium
- Default tags to use in logs: janet_school, research, mcm, telemetry

## Suspend rundown template

> **Status:** Janet School is at `<current run state or ready state>`.
> **What changed:** `<latest run outcome, skill profile, or setup change>`.
> **Open questions:** `<runtime readiness, comparison target, or anomaly/refusal follow-up>`.
> **Next action:** `<run next condition, inspect artifacts, or compare sessions>`.
> **Artifacts:** `<session artifact paths or compare export paths when relevant>`.

## Portable bridge note

This hosted module runs Janet School from its own module folder and uses the ChattyCog bridge only for discovery, hosted UI status, and cross-module context. Janet School remains responsible for its own runtime, code, and data layout.
