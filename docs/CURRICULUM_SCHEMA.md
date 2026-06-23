# Curriculum Schema

## Purpose

The curriculum schema defines how Janet School represents generated or loaded
curriculum items for teaching and probing. These fields describe experimental
hypotheses and orchestration needs. They do not determine what Janet actually did.

## Design Rules

- Support large generated curricula, not only toy fixtures.
- Support teaching items and multiple probe types.
- Support many domains and concepts.
- Preserve experimental hypotheses without turning them into truth labels.
- Make curriculum validation strict enough for reproducibility.

## Top-Level Objects

### CurriculumBundle

Represents a generated or loaded curriculum for one session.

Suggested fields:

- `curriculum_id`
- `session_id`
- `version`
- `generated_at`
- `generated_by`
- `teacher_backend_id`
- `run_mode`
- `domains`
- `items`
- `generation_notes`
- `schema_version`

### CurriculumDomain

Represents a curriculum domain such as sequencing or spatial reasoning.

Suggested fields:

- `domain_id`
- `name`
- `description`
- `concepts`
- `notes`

### CurriculumConcept

Represents a concept within a domain.

Suggested fields:

- `concept_id`
- `domain_id`
- `name`
- `description`
- `difficulty_band`
- `notes`

### CurriculumItem

Represents one teaching item or probe item.

Required core fields:

- `item_id`
- `domain_id`
- `concept_id`
- `item_type`
- `prompt`
- `difficulty`
- `surface_domain`
- `novelty_class`
- `probe_role`
- `representation_type`
- `created_by`
- `created_at`
- `curriculum_version`
- `schema_version`

Recommended answer fields:

- `expected_answer`
- `acceptable_answers`
- `answer_format`

Recommended hypothesis fields:

- `teaching_context`
- `intended_relations`
- `expected_skills`
- `boundary_kind`
- `composition_parts`
- `notes`

## Enumerations

### `item_type`

- `teaching`
- `probe_near_transfer`
- `probe_far_transfer`
- `probe_cross_representation`
- `probe_composition`
- `probe_boundary`
- `probe_open_structure`

### `novelty_class`

- `familiar`
- `slightly_varied`
- `novel_surface`
- `novel_structure`
- `mixed_novelty`
- `unknown`

### `probe_role`

- `none`
- `verification`
- `transfer`
- `boundary`
- `open_structure`

### `representation_type`

- `text`
- `symbolic`
- `ordered_tokens`
- `categorical`
- `spatial_text`
- `multi_part_text`

### `boundary_kind`

- `none`
- `adjacent_skill`
- `near_miss`
- `rule_exception`
- `cross_category`
- `underspecified`
- `open_ended`

## Validation Rules

### Identity Rules

- `item_id` must be unique within the curriculum.
- `domain_id` must reference a declared domain.
- `concept_id` must reference a concept in the matching domain.

### Structural Rules

- All probe items must declare a non-`none` `probe_role`.
- Open-structure probes should use `probe_open_structure` or `open_structure`.
- Composition probes should include `composition_parts` when applicable.
- `acceptable_answers` may be empty for certain open-structure probes, but this
  must be explicit and justified.

### Provenance Rules

- `created_by` must record whether the item came from a teacher backend, fixture,
  human-authored source, or import path.
- `curriculum_version` and `schema_version` are required for reproducibility.

## Minimum Full-Curriculum Expectations For v0.1

For full runs, the curriculum generator should be capable of producing at least:

- 12 domains
- 5 concepts per domain
- 5 teaching items per concept
- 3 probe items per concept
- 1 boundary or open-structure probe per concept

This is a capability requirement for the system, not a requirement that every test
run execute the full set.

## Starter Domain Scaffold

Suggested initial domains:

- attention and discrimination
- matching and sameness/difference
- sorting and categorization
- sequencing and order
- quantity and comparison
- basic numeracy
- language comprehension
- relation words
- spatial reasoning
- temporal reasoning
- cause and effect
- functional problem solving
- social/pragmatic scenario reasoning
- rule following and exception handling
- pattern recognition
- abstraction and transfer probes

## Example Item Shape

```json
{
  "item_id": "seq-ord-00017",
  "domain_id": "sequencing_order",
  "concept_id": "before_after",
  "item_type": "probe_boundary",
  "prompt": "If Mia finishes lunch before art and art is before reading, what comes first?",
  "expected_answer": "lunch",
  "acceptable_answers": ["lunch", "Mia finishes lunch first"],
  "answer_format": "short_text",
  "teaching_context": "ordering relations using before/after language",
  "difficulty": 2,
  "surface_domain": "language",
  "intended_relations": ["ordering", "transitive_relation"],
  "expected_skills": ["compare_order_terms", "compose_two_relations"],
  "novelty_class": "slightly_varied",
  "probe_role": "boundary",
  "boundary_kind": "adjacent_skill",
  "representation_type": "text",
  "composition_parts": ["relation_a", "relation_b"],
  "notes": "Boundary probe for compositional ordering.",
  "created_by": "teacher:mock",
  "created_at": "2026-06-22T00:00:00Z",
  "curriculum_version": "0.1.0",
  "schema_version": "0.1.0"
}
```

## Interpretation Rule

Curriculum metadata expresses the experimenter's and teacher's intended framing. If
telemetry later suggests a different structure fit, the telemetry wins as evidence.
