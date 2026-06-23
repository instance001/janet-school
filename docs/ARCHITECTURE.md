# Janet School Architecture

## Overview

Janet School is a standalone Rust application with a thin WebView GUI and a
headless-capable backend. The backend owns the research logic, data contracts,
telemetry, and analysis pipeline. The GUI is an operator surface over that backend.

The system is intentionally divided into four cooperating areas:

- configuration and session control
- deterministic MCM student execution
- teacher proposal generation
- telemetry and analysis

## Architectural Principles

- The MCM core is deterministic and inspectable.
- The teacher backend is external to the MCM decision path.
- All persisted evidence is explicit and append-only where event-like.
- The session runner is the orchestrator; the GUI is not the orchestrator.
- Analysis is rule-based first and conservative in wording.
- The application must run headlessly for testing and automation.

## High-Level Components

### App Shell

Owns startup, config loading, command routing, and GUI bootstrapping.

Suggested modules:

- `main`
- `app`
- `config`

### Session Engine

Coordinates end-to-end runs:

1. load config
2. create session folder
3. generate or load curriculum
4. validate curriculum
5. execute teaching items
6. execute probe items
7. persist telemetry
8. run analyzer
9. emit reports

Suggested modules:

- `session`
- `storage`
- `util`

### Curriculum System

Handles curriculum generation, normalization, validation, indexing, and traversal.
Curriculum annotations remain hypotheses supplied by humans or the teacher.

Suggested modules:

- `curriculum`

### Teacher Layer

Provides proposal-generation services only. Teacher outputs are always treated as
generated artifacts to be logged, validated, and interpreted by the rest of the
system.

Interface shape:

```rust
trait TeacherBackend {
    fn generate_curriculum(&self, request: CurriculumRequest) -> Result<CurriculumDraft>;
    fn generate_question(&self, context: QuestionContext) -> Result<CurriculumItem>;
    fn evaluate_answer(&self, context: EvaluationContext) -> Result<TeacherEvaluation>;
    fn suggest_next_step(&self, state: SessionStateView) -> Result<TeacherAdvice>;
    fn summarize_session(&self, data: SessionDataView) -> Result<TeacherSessionNotes>;
}
```

Planned backends:

- `MockTeacherBackend`
- `LocalLlmTeacherBackend`

The local GGUF model and Vulkan-enabled runtime are future inputs to
`LocalLlmTeacherBackend`, not part of the MCM core.

### MCM Core

The deterministic student pipeline should remain inspectable and stable:

1. receive question
2. normalize input
3. classify surface features
4. query explicit memory
5. identify candidate skills
6. check skill approvals and policy
7. execute approved deterministic path, compose a partial deterministic answer, or refuse
8. emit answer
9. emit full trace

Suggested modules:

- `mcm`
- `skills`
- `memory`

### Telemetry Layer

Captures all event streams and write-once artifacts. This layer should make it hard
to skip logging by accident.

Suggested modules:

- `telemetry`
- `storage`

### Analysis Layer

Consumes session artifacts and produces rule-based summaries and candidate findings.
The analyzer must not upgrade evidence into claims of proof.

Suggested modules:

- `analysis`

### GUI Layer

Provides pages for setup, curriculum inspection, run control, live telemetry,
analysis review, and exports. The GUI should subscribe to backend state rather than
reimplement any core logic.

Suggested modules:

- `gui`
- `web/`

## Data Flow

### Run Preparation

1. Operator selects configs and run mode.
2. App creates a session folder and manifest.
3. Teacher backend generates or curriculum loader reads item data.
4. Curriculum validator normalizes and validates the curriculum.

### Item Execution

1. Session engine selects the next curriculum item.
2. Item is passed to the MCM core.
3. MCM produces answer plus deterministic trace.
4. Teacher may evaluate the answer and provide advisory feedback.
5. Session engine writes all events and updates session state.

### Analysis

1. Analyzer reads completed artifacts.
2. Analyzer classifies events into confirmed, boundary, emergent, and unknown buckets.
3. Analyzer clusters repeated anomalies and mismatch patterns.
4. Analyzer emits JSON and Markdown reports.

## Isolation Boundaries

### Teacher Isolation

- Teacher outputs are advisory proposals.
- Teacher cannot write memory directly.
- Teacher cannot decide final MCM outputs.
- Every teacher call is logged separately.

### Memory Isolation

- Explicit memory is the only operational memory for the MCM.
- Probe feedback must not contaminate teaching memory.
- All memory writes are logged and attributable.

### Policy Isolation

- Policy checks occur before final output decisions.
- No backend may bypass policy enforcement.
- Policy results must appear in the deterministic trace.

### Analysis Isolation

- Analysis runs after evidence capture, not instead of it.
- Analyzer classifications are review artifacts, not ground truth.

## Runtime Notes

The workspace currently contains a local runtime bundle and a GGUF model artifact.
These should be represented in configuration as optional teacher-backend resources.
The application must still be usable in mock-teacher mode without those assets.

## Suggested Initial Project Layout

```text
janet-school-rs/
  Cargo.toml
  src/
    main.rs
    app.rs
    config/
    curriculum/
    teacher/
    mcm/
    skills/
    memory/
    telemetry/
    analysis/
    session/
    gui/
    storage/
    util/
  web/
    index.html
    app.js
    styles.css
  config/
    app_config.json
    teacher_config.json
    mcm_config.json
    skill_manifest.json
    skill_approvals.json
  data/
    sessions/
    aggregated/
  docs/
```

## Phase Boundaries

### Phase 0

Documentation only. Lock terminology, architecture, schemas, and UI intent.

### Phase 1

Skeleton only. No teacher calls and no hidden shortcuts.

### Later Phases

Add the deterministic MCM, then teacher backends, then session runner, then
analysis, then GUI integration, while preserving the boundaries described above.
