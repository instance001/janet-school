# Analysis Schema

## Purpose

The analyzer translates session evidence into conservative, reviewable findings. It
does not prove abstraction or intelligence. It flags patterns that may deserve
further human review and better-designed probes.

## Analysis Posture

- Evidence first
- Rules before speculative modeling
- Repetition matters more than one-off results
- Mismatch is informative
- Ambiguity should be surfaced, not smoothed away

## Output Families

The analyzer should produce:

- confirmed signals
- boundary signals
- emergent candidate signals
- unknown structure candidates
- repeated anomaly clusters
- category mismatch clusters
- cross-session pattern summaries
- caution notes
- recommended next probes

## Top-Level Report Shape

Suggested fields for `analysis_report.json`:

- `session_id`
- `run_id`
- `analysis_version`
- `generated_at`
- `confirmed_signals`
- `boundary_signals`
- `emergent_candidate_signals`
- `unknown_structure_candidates`
- `repeated_anomaly_clusters`
- `category_mismatch_clusters`
- `cross_session_summary`
- `caution_notes`
- `recommended_next_probes`

The Markdown report should mirror the same structure in a human-readable form.

## Signal Definitions

### Confirmed Signal

A pattern that fits expected tags and is well explained by known memory, approved
skills, or expected transfer behavior.

Suggested fields:

- `signal_id`
- `type`
- `summary`
- `supporting_event_ids`
- `explanation`
- `confidence`

Examples:

- exact recall
- approved skill on intended item
- expected transfer inside a tagged category

### Boundary Signal

A pattern suggesting Janet is near a known structure boundary or being constrained
by skill/policy edges.

Suggested fields:

- `signal_id`
- `type`
- `summary`
- `supporting_event_ids`
- `boundary_kind`
- `why_flagged`
- `recommended_probe`
- `confidence`

Examples:

- approval-blocked matches
- adjacent-skill near misses
- refusals close to known structures
- partial compositional attempts
- inconsistent refusal behavior across similar items

### Emergent Candidate Signal

A repeated or clustered pattern that current tags do not explain well and that may
suggest a missing or too-coarse structure in the taxonomy or curriculum labels.

Suggested fields:

- `signal_id`
- `summary`
- `supporting_event_ids`
- `why_flagged`
- `current_tags_failed_to_explain`
- `pattern_repeated`
- `confidence`
- `requires_human_review`
- `recommended_probe`

Examples:

- repeated success where intended tags fit poorly
- stable success across items with different intended relations
- repeated skill path divergence with correct answers
- cross-item clustering not predicted by the current taxonomy

### Unknown Structure Candidate

A stricter form of caution where the system observed a pattern but does not yet have
enough explanatory coverage to place it confidently.

Suggested fields:

- `candidate_id`
- `summary`
- `supporting_event_ids`
- `observed_commonality`
- `missing_explanation`
- `recommended_probe`
- `requires_human_review`

## Cluster Definitions

### Repeated Anomaly Cluster

Groups similar anomalies across multiple items.

Suggested fields:

- `cluster_id`
- `anomaly_type`
- `item_ids`
- `supporting_event_ids`
- `count`
- `common_features`
- `interpretation`

### Category Mismatch Cluster

Groups items where observed behavior repeatedly fails to align with intended labels.

Suggested fields:

- `cluster_id`
- `item_ids`
- `supporting_event_ids`
- `intended_category_pattern`
- `observed_behavior_pattern`
- `possible_labeling_gap`
- `recommended_probe`

## Confidence Language

The analyzer should prefer a small controlled vocabulary:

- `low`
- `medium`
- `high`

High confidence means confidence in the flagging rationale, not confidence that a
deep abstraction has been proven.

## Hard Analysis Rules

- One success is never enough to claim abstraction.
- Emergent signals must remain candidates.
- Every emergent signal must cite supporting event IDs.
- Every emergent signal must include a recommended next probe.
- Human review is required for strong interpretive claims.
- Contradictory evidence should be preserved in caution notes.

## Recommended Next Probes

Probe recommendations should be concrete and testable. They may suggest:

- near-transfer retests
- far-transfer variations
- cross-representation variants
- composition decompositions
- ambiguity-control items
- boundary stress items

## Cross-Session Summary

Once multiple sessions exist, the analyzer should be able to aggregate:

- recurring confirmed structures
- recurring boundary pressure points
- recurring unexplained clusters
- drift signals over time

This remains descriptive, not proof-oriented.
