# Telemetry Schema

## Purpose

Telemetry is a first-class product output of Janet School. Every interaction,
teacher call, memory event, policy check, and analysis artifact should be preserved
in explicit, inspectable records.

## Storage Principles

- Event streams use append-only JSONL.
- Session summary artifacts may use JSON and Markdown.
- Raw evidence should be preserved even when later analysis disagrees with earlier
  expectations.
- Telemetry should be rich enough to reconstruct the path of a run.

## Session Folder Outputs

Required output files per session:

- `session_config.json`
- `curriculum_generated.jsonl`
- `curriculum_validated.jsonl`
- `teacher_calls.jsonl`
- `interactions.jsonl`
- `mcm_trace.jsonl`
- `telemetry.jsonl`
- `memory_events.jsonl`
- `skill_events.jsonl`
- `refusal_events.jsonl`
- `transfer_probes.jsonl`
- `anomaly_events.jsonl`
- `analysis_report.json`
- `analysis_report.md`
- `session_summary.json`

## Event Families

### Interaction Event

Captures one curriculum item attempt.

Suggested fields:

- `event_id`
- `session_id`
- `run_id`
- `timestamp`
- `curriculum_version`
- `item_id`
- `domain_id`
- `concept_id`
- `item_type`
- `teacher_backend_id`
- `teacher_prompt`
- `generated_question`
- `intended_structure`
- `expected_answer`
- `janet_answer`
- `correctness_judgment`
- `teacher_feedback`
- `structure_fit`
- `anomaly_flags`
- `latency_ms`
- `raw_event_trace_hash`
- `code_version_hash`

### MCM Trace Event

Captures the deterministic decision path for one item.

Suggested fields:

- `trace_id`
- `session_id`
- `item_id`
- `input_normalization`
- `surface_features`
- `memory_reads`
- `candidate_skills`
- `approved_skills`
- `blocked_skills`
- `executed_skill`
- `partial_attempt`
- `refusal_reason`
- `reasoning_steps`
- `policy_checks`
- `uncertainty_state`
- `final_mode`

### Teacher Call Event

Captures each teacher backend request and response.

Suggested fields:

- `teacher_call_id`
- `session_id`
- `timestamp`
- `teacher_backend_id`
- `operation`
- `request_payload`
- `response_payload`
- `token_counts`
- `latency_ms`
- `model_config`
- `runtime_config`
- `success`
- `error`

### Memory Event

Captures explicit MCM memory reads and writes.

Suggested fields:

- `memory_event_id`
- `session_id`
- `timestamp`
- `operation`
- `memory_scope`
- `key`
- `value_before`
- `value_after`
- `reason`
- `source_item_id`
- `approved_by_policy`

### Skill Event

Captures skill discovery, approval, and execution evidence.

Suggested fields:

- `skill_event_id`
- `session_id`
- `timestamp`
- `item_id`
- `candidate_skills`
- `approved_skills`
- `blocked_skills`
- `executed_skill`
- `approval_ledger_version`
- `reason`

### Refusal Event

Captures any abstention or refusal path.

Suggested fields:

- `refusal_event_id`
- `session_id`
- `timestamp`
- `item_id`
- `reason`
- `uncertainty_state`
- `candidate_next_steps`
- `policy_trace`

### Anomaly Event

Captures flagged anomalies and boundary pressure indicators.

Suggested fields:

- `anomaly_event_id`
- `session_id`
- `timestamp`
- `item_id`
- `structure_fit`
- `anomaly_flags`
- `observed_structure`
- `structure_fit_explanation`
- `supporting_trace_ids`

## Enumerations

### `structure_fit`

- `matched`
- `partial_match`
- `mismatch`
- `unknown`

### `anomaly_flags`

- `unexpected_success`
- `unexpected_failure`
- `unexpected_refusal`
- `unexpected_skill_path`
- `unlabeled_transfer`
- `structure_mismatch`
- `boundary_pressure`
- `category_mismatch`
- `possible_emergent_structure`
- `repeated_near_miss`
- `teacher_taxonomy_insufficient`

### `final_mode`

- `answered_from_memory`
- `answered_with_skill`
- `partial_attempt`
- `refused`

## Logging Rules

- Teacher calls must be logged separately from MCM traces.
- Probe feedback must not write into teaching memory paths unless an explicit future
  design says otherwise.
- Policy checks must be visible in the trace.
- Missing optional values should still preserve the event rather than dropping it.
- Hashes should be included where possible for trace integrity.

## Session Summary Shape

Suggested top-level fields:

- `session_id`
- `run_id`
- `started_at`
- `completed_at`
- `run_mode`
- `teacher_backend_id`
- `curriculum_stats`
- `interaction_stats`
- `memory_stats`
- `skill_stats`
- `refusal_stats`
- `anomaly_stats`
- `analysis_artifact_paths`
- `notes`

## Practical Note

Telemetry should be designed so that later tools can reconstruct:

- what item was asked
- what Janet saw
- what Janet used
- what Janet refused
- what the teacher proposed
- what the analyzer concluded

If any of those questions cannot be answered from session artifacts, the schema is
too weak.
