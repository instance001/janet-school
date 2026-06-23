# GUI Plan

## Purpose

The GUI is an operator console for running and inspecting Janet School sessions. It
must make the research process legible without becoming the source of core logic.

## Design Principles

- Thin client over backend state
- Research console, not chatbot shell
- Clear evidence surfaces over decorative polish
- Provisional wording throughout
- Strong visibility into deterministic and stochastic boundaries

## Interaction Model

The backend owns session state, event generation, and analysis. The GUI reads state,
subscribes to updates, sends explicit commands, and presents artifacts.

The GUI should never:

- decide Janet outputs
- execute teacher logic directly
- hide anomalies for cleanliness
- present anthropomorphic identity cues

## Primary Pages

### Setup Page

Purpose:
Configure a session before execution.

Controls:

- teacher backend selector
- optional local model path
- optional localhost endpoint
- curriculum generation size
- session name
- output folder
- run mode selector

Displayed metadata:

- backend description
- whether runtime/model resources are detected
- warning when full-run size is below target expectations

### Curriculum Page

Purpose:
Inspect the generated or loaded curriculum before running.

Views:

- domain list
- concept list
- item counts
- probe counts
- item detail inspector

Warnings:

- missing required fields
- too-small curriculum
- low probe coverage

### Run Page

Purpose:
Operate the session runner.

Controls:

- start
- pause
- stop

Displayed state:

- current domain
- current concept
- current item
- current prompt
- Janet answer
- teacher feedback
- progress counters

### Live Telemetry Page

Purpose:
Expose what happened during execution in near real time.

Views:

- memory reads and writes
- candidate skills
- approved skills
- blocked skills
- executed skill
- policy checks
- refusal events
- structure fit
- anomaly flags

This page is one of the main research surfaces and should be treated as core UI.

### Analysis Page

Purpose:
Review post-run findings without overclaiming.

Sections:

- confirmed signals
- boundary signals
- emergent candidate signals
- unknown structure candidates
- repeated anomalies
- recommended next probes
- caution notes

### Export Page

Purpose:
Provide artifact access and later handoff support.

Actions:

- open session folder
- export Markdown report
- export JSON bundle
- show generated artifact paths

ChattyCog handoff support may be added later but is out of scope for v0.1.

## UX Guardrails

Avoid:

- avatars
- emotional styling
- assistant-like chat framing
- personality language
- celebratory abstraction claims

Prefer:

- explicit labels such as `teacher backend`, `MCM trace`, and `analysis candidate`
- structured tables and inspectors
- visible timestamps and provenance
- warnings when data is incomplete or interpretive

## Frontend Architecture

Suggested structure:

- static web assets served locally
- simple command bridge between WebView and Rust backend
- event subscription or polling for session status
- artifact readers for JSON/JSONL/Markdown outputs

## v0.1 Visual Tone

The interface should feel like a lab console:

- clean
- inspectable
- restrained
- trace-first

It should not feel like a teaching product or conversational AI frontend.

## Phase Sequence

### Phase 0

Define page responsibilities and boundary rules only.

### Phase 1

Implement a basic shell and navigation with stub data.

### Later Phases

Connect setup to config, run control to session engine, live telemetry to event
streams, and analysis to generated reports while keeping all core logic in Rust.
