use anyhow::{anyhow, Result};
use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
struct DomainCatalogEntry {
    domain_id: &'static str,
    domain_name: &'static str,
    concept_ids: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CurriculumBundle {
    pub curriculum_id: String,
    pub session_id: String,
    pub version: String,
    pub generated_at: String,
    pub generated_by: String,
    pub teacher_backend_id: String,
    pub run_mode: String,
    pub domains: Vec<CurriculumDomain>,
    pub items: Vec<CurriculumItem>,
    pub generation_notes: Vec<String>,
    pub schema_version: String,
}

impl CurriculumBundle {
    pub fn validate(&self) -> Result<CurriculumValidationReport> {
        if self.domains.is_empty() {
            return Err(anyhow!("curriculum contains no domains"));
        }
        if self.items.is_empty() {
            return Err(anyhow!("curriculum contains no items"));
        }

        for item in &self.items {
            if item.item_id.trim().is_empty() {
                return Err(anyhow!("curriculum item has an empty item_id"));
            }
            if item.prompt.trim().is_empty() {
                return Err(anyhow!("curriculum item {} has an empty prompt", item.item_id));
            }
        }

        Ok(CurriculumValidationReport {
            curriculum_id: self.curriculum_id.clone(),
            item_count: self.items.len(),
            domain_count: self.domains.len(),
            probe_count: self
                .items
                .iter()
                .filter(|item| item.item_type.starts_with("probe_"))
                .count(),
            valid: true,
            warnings: Vec::new(),
            validated_at: Utc::now().to_rfc3339(),
        })
    }

    pub fn summary(&self) -> CurriculumSummary {
        CurriculumSummary {
            curriculum_id: self.curriculum_id.clone(),
            item_count: self.items.len(),
            domain_count: self.domains.len(),
            concept_count: self.domains.iter().map(|d| d.concepts.len()).sum(),
            probe_count: self
                .items
                .iter()
                .filter(|item| item.item_type.starts_with("probe_"))
                .count(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CurriculumDomain {
    pub domain_id: String,
    pub name: String,
    pub description: String,
    pub concepts: Vec<CurriculumConcept>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CurriculumConcept {
    pub concept_id: String,
    pub domain_id: String,
    pub name: String,
    pub description: String,
    pub difficulty_band: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CurriculumItem {
    pub item_id: String,
    pub domain_id: String,
    pub concept_id: String,
    pub item_type: String,
    pub prompt: String,
    pub expected_answer: Option<String>,
    pub acceptable_answers: Vec<String>,
    pub answer_format: Option<String>,
    pub teaching_context: Option<String>,
    pub difficulty: u8,
    pub surface_domain: String,
    pub intended_relations: Vec<String>,
    pub expected_skills: Vec<String>,
    pub novelty_class: String,
    pub probe_role: String,
    pub boundary_kind: String,
    pub representation_type: String,
    pub composition_parts: Vec<String>,
    pub notes: Option<String>,
    pub created_by: String,
    pub created_at: String,
    pub curriculum_version: String,
    pub schema_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CurriculumRequest {
    pub session_id: String,
    pub run_mode: String,
    pub curriculum_size: String,
    pub target_domain_count: usize,
    pub concepts_per_domain: usize,
    pub teaching_items_per_concept: usize,
    pub probe_items_per_concept: usize,
    pub include_boundary_probe_per_concept: bool,
}

impl CurriculumRequest {
    pub fn from_size_hint(session_id: String, run_mode: String, size_hint: &str) -> Self {
        match size_hint {
            "full" => Self {
                session_id,
                run_mode,
                curriculum_size: "full".to_string(),
                target_domain_count: 12,
                concepts_per_domain: 5,
                teaching_items_per_concept: 5,
                probe_items_per_concept: 3,
                include_boundary_probe_per_concept: true,
            },
            "smoke" => Self {
                session_id,
                run_mode,
                curriculum_size: "smoke".to_string(),
                target_domain_count: 5,
                concepts_per_domain: 2,
                teaching_items_per_concept: 2,
                probe_items_per_concept: 1,
                include_boundary_probe_per_concept: true,
            },
            _ => Self {
                session_id,
                run_mode,
                curriculum_size: "tiny_fixture".to_string(),
                target_domain_count: 1,
                concepts_per_domain: 1,
                teaching_items_per_concept: 1,
                probe_items_per_concept: 1,
                include_boundary_probe_per_concept: false,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CurriculumValidationReport {
    pub curriculum_id: String,
    pub item_count: usize,
    pub domain_count: usize,
    pub probe_count: usize,
    pub valid: bool,
    pub warnings: Vec<String>,
    pub validated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CurriculumSummary {
    pub curriculum_id: String,
    pub item_count: usize,
    pub domain_count: usize,
    pub concept_count: usize,
    pub probe_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CurriculumOutline {
    pub rationale: String,
    pub domains: Vec<OutlineDomain>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OutlineDomain {
    pub domain_id: String,
    pub domain_name: String,
    pub concepts: Vec<OutlineConcept>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct OutlineConcept {
    pub concept_id: String,
    pub concept_name: String,
}

pub fn build_mock_curriculum(request: &CurriculumRequest, created_by: &str) -> CurriculumBundle {
    let selected = default_domain_catalog()
        .into_iter()
        .take(request.target_domain_count)
        .collect::<Vec<_>>();
    build_curriculum_from_catalog(
        request,
        &selected,
        created_by,
        vec![
            "Mock curriculum generated deterministically.".to_string(),
            format!("Curriculum size profile: {}", request.curriculum_size),
        ],
    )
}

pub fn build_curriculum_from_outline(
    request: &CurriculumRequest,
    outline: &CurriculumOutline,
    created_by: &str,
) -> CurriculumBundle {
    let catalog = default_domain_catalog();
    let mut selected = Vec::new();

    for outlined_domain in &outline.domains {
        if let Some(entry) = catalog
            .iter()
            .find(|entry| entry.domain_id == outlined_domain.domain_id)
        {
            selected.push(entry.clone());
        }
    }

    for entry in catalog {
        if selected.len() >= request.target_domain_count {
            break;
        }
        if !selected.iter().any(|chosen| chosen.domain_id == entry.domain_id) {
            selected.push(entry);
        }
    }

    build_curriculum_from_catalog(
        request,
        &selected,
        created_by,
        vec![
            "Local LLM outline expanded deterministically.".to_string(),
            outline.rationale.clone(),
        ],
    )
}

pub fn outline_catalog_reference() -> Vec<OutlineDomain> {
    default_domain_catalog()
        .into_iter()
        .map(|entry| OutlineDomain {
            domain_id: entry.domain_id.to_string(),
            domain_name: entry.domain_name.to_string(),
            concepts: entry
                .concept_ids
                .iter()
                .map(|concept_id| OutlineConcept {
                    concept_id: format!("{}_{}", entry.domain_id, concept_id),
                    concept_name: concept_id.replace('_', " "),
                })
                .collect(),
        })
        .collect()
}

fn build_curriculum_from_catalog(
    request: &CurriculumRequest,
    selected: &[DomainCatalogEntry],
    created_by: &str,
    generation_notes: Vec<String>,
) -> CurriculumBundle {
    let mut domains = Vec::new();
    let mut items = Vec::new();
    let now = Utc::now().to_rfc3339();

    for (domain_index, entry) in selected.iter().take(request.target_domain_count).enumerate() {
        let concept_specs = entry
            .concept_ids
            .iter()
            .take(request.concepts_per_domain)
            .enumerate()
            .map(|(concept_index, concept_key)| CurriculumConcept {
                concept_id: format!("{}_{}", entry.domain_id, concept_key),
                domain_id: entry.domain_id.to_string(),
                name: concept_key.replace('_', " "),
                description: format!("Concept {} for {}", concept_index + 1, entry.domain_name),
                difficulty_band: if concept_index == 0 {
                    "intro".to_string()
                } else {
                    "core".to_string()
                },
                notes: vec!["Teacher-selected concept".to_string()],
            })
            .collect::<Vec<_>>();

        for concept in &concept_specs {
            for teaching_index in 0..request.teaching_items_per_concept {
                items.push(mock_item(
                    domain_index,
                    concept,
                    "teaching",
                    teaching_index,
                    created_by,
                    &now,
                    "none",
                ));
            }

            for probe_index in 0..request.probe_items_per_concept {
                let skill_backed = is_skill_backed_concept(&concept.concept_id);
                let probe_item_type = if skill_backed {
                    "probe_near_transfer"
                } else if request.include_boundary_probe_per_concept && probe_index == 0 {
                    "probe_boundary"
                } else {
                    "probe_near_transfer"
                };
                let probe_boundary_kind = if skill_backed {
                    "skill_transfer"
                } else if request.include_boundary_probe_per_concept && probe_index == 0 {
                    "adjacent_skill"
                } else {
                    "none"
                };
                items.push(mock_item(
                    domain_index,
                    concept,
                    probe_item_type,
                    probe_index,
                    created_by,
                    &now,
                    probe_boundary_kind,
                ));
            }
        }

        domains.push(CurriculumDomain {
            domain_id: entry.domain_id.to_string(),
            name: entry.domain_name.to_string(),
            description: format!("Teacher scaffold for {}", entry.domain_name),
            concepts: concept_specs,
            notes: vec![format!("Generated by {}", created_by)],
        });
    }

    CurriculumBundle {
        curriculum_id: Uuid::new_v4().to_string(),
        session_id: request.session_id.clone(),
        version: "0.1.0".to_string(),
        generated_at: now,
        generated_by: created_by.to_string(),
        teacher_backend_id: created_by.to_string(),
        run_mode: request.run_mode.clone(),
        domains,
        items,
        generation_notes,
        schema_version: "0.1.0".to_string(),
    }
}

fn default_domain_catalog() -> Vec<DomainCatalogEntry> {
    vec![
        DomainCatalogEntry {
            domain_id: "attention_discrimination",
            domain_name: "Attention and Discrimination",
            concept_ids: &[
                "visual_notice",
                "signal_pickout",
                "difference_detect",
                "target_tracking",
                "salience_filter",
            ],
        },
        DomainCatalogEntry {
            domain_id: "matching_difference",
            domain_name: "Matching and Difference",
            concept_ids: &[
                "same_vs_different",
                "exact_match",
                "near_match",
                "feature_overlap",
                "contrast_pair",
            ],
        },
        DomainCatalogEntry {
            domain_id: "sequencing_order",
            domain_name: "Sequencing and Order",
            concept_ids: &[
                "before_after",
                "first_last",
                "ordered_steps",
                "middle_position",
                "sequence_repair",
            ],
        },
        DomainCatalogEntry {
            domain_id: "quantity_comparison",
            domain_name: "Quantity and Comparison",
            concept_ids: &[
                "more_less",
                "equal_compare",
                "relative_quantity",
                "largest_smallest",
                "quantity_gap",
            ],
        },
        DomainCatalogEntry {
            domain_id: "spatial_reasoning",
            domain_name: "Spatial Reasoning",
            concept_ids: &[
                "left_right",
                "inside_outside",
                "position_relation",
                "above_below",
                "near_far",
            ],
        },
        DomainCatalogEntry {
            domain_id: "language_comprehension",
            domain_name: "Language Comprehension",
            concept_ids: &[
                "simple_instruction",
                "relation_words",
                "short_passage",
                "reference_resolution",
                "command_following",
            ],
        },
        DomainCatalogEntry {
            domain_id: "temporal_reasoning",
            domain_name: "Temporal Reasoning",
            concept_ids: &[
                "before_later",
                "sequence_time",
                "temporal_marker",
                "duration_order",
                "event_spacing",
            ],
        },
        DomainCatalogEntry {
            domain_id: "cause_effect",
            domain_name: "Cause and Effect",
            concept_ids: &[
                "simple_cause",
                "effect_choice",
                "outcome_link",
                "cause_chain",
                "intervention_effect",
            ],
        },
        DomainCatalogEntry {
            domain_id: "rule_exception",
            domain_name: "Rule Following and Exception Handling",
            concept_ids: &[
                "simple_rule",
                "rule_break",
                "exception_case",
                "conditional_rule",
                "priority_override",
            ],
        },
        DomainCatalogEntry {
            domain_id: "pattern_recognition",
            domain_name: "Pattern Recognition",
            concept_ids: &[
                "repeat_pattern",
                "alternate_pattern",
                "missing_piece",
                "growth_pattern",
                "pattern_switch",
            ],
        },
        DomainCatalogEntry {
            domain_id: "functional_problem_solving",
            domain_name: "Functional Problem Solving",
            concept_ids: &[
                "tool_choice",
                "goal_match",
                "repair_step",
                "constraint_handling",
                "multi_step_fix",
            ],
        },
        DomainCatalogEntry {
            domain_id: "abstraction_transfer",
            domain_name: "Abstraction and Transfer Probes",
            concept_ids: &[
                "cross_surface",
                "mixed_structure",
                "transfer_probe",
                "cross_representation",
                "open_structure_probe",
            ],
        },
    ]
}

fn mock_item(
    _domain_index: usize,
    concept: &CurriculumConcept,
    item_type: &str,
    item_index: usize,
    created_by: &str,
    created_at: &str,
    boundary_kind: &str,
) -> CurriculumItem {
    let domain_label = concept.domain_id.replace('_', " ");
    let concept_label = concept.name.clone();
    let item_number = item_index + 1;
    let concept_token = concept.concept_id.rsplit('_').next().unwrap_or("concept");
    let is_transfer_probe = item_type == "probe_near_transfer";
    let (prompt, expected_answer, acceptable_answers, expected_skills) =
        if concept.concept_id.contains("before_after") {
            let subject = if item_type == "teaching" { "alpha" } else { "gamma" };
            let object = if item_type == "teaching" { "beta" } else { "delta" };
            (
                format!(
                    "If {subject} comes before {object} in {domain_label}, what comes first?"
                ),
                subject.to_string(),
                vec![subject.to_string()],
                vec!["ordered_relation_compare".to_string()],
            )
        } else if concept.concept_id.contains("same_vs_different") {
            let left = if item_type == "teaching" { "red" } else { "square" };
            let right = if item_type == "teaching" { "red" } else { "triangle" };
            let answer = if left == right { "same" } else { "different" };
            (
                format!("Is {left} and {right} the same or different?"),
                answer.to_string(),
                vec![answer.to_string()],
                vec!["same_different_compare".to_string()],
            )
        } else if concept.concept_id.contains("visual_notice")
            || concept.concept_id.contains("signal_pickout")
            || concept.concept_id.contains("exact_match")
        {
            let sample = if item_type == "teaching" { "red" } else { "triangle" };
            let options = if item_type == "teaching" {
                ("red", "blue", "green")
            } else {
                ("circle", "triangle", "square")
            };
            (
                format!(
                    "Which option matches sample {sample}? options: {}, {}, {}.",
                    options.0, options.1, options.2
                ),
                sample.to_string(),
                vec![sample.to_string()],
                vec!["option_match_selector".to_string()],
            )
        } else if concept.concept_id.contains("first_last") {
            let list = if item_type == "teaching" {
                "cat, dog, bird"
            } else {
                "oak, pine, maple"
            };
            let asks_first = item_number % 2 == 1;
            let answer = if asks_first {
                list.split(',').next().unwrap_or("").trim()
            } else {
                list.split(',').next_back().unwrap_or("").trim()
            };
            (
                format!(
                    "Which is {} in the list: {list}.",
                    if asks_first { "first" } else { "last" }
                ),
                answer.to_string(),
                vec![answer.to_string()],
                vec!["first_last_selector".to_string()],
            )
        } else if concept.concept_id.contains("more_less") {
            let (left, right) = if item_type == "teaching" { ("3", "5") } else { ("2", "7") };
            let left_num = left.parse::<u32>().unwrap_or(0);
            let right_num = right.parse::<u32>().unwrap_or(0);
            let answer = if left_num >= right_num {
                left
            } else {
                right
            };
            (
                format!("Which is more {left} or {right}?"),
                answer.to_string(),
                vec![answer.to_string()],
                vec!["more_less_compare".to_string()],
            )
        } else if concept.concept_id.contains("equal_compare") {
            let (left, right) = if item_type == "teaching" { ("4", "4") } else { ("6", "8") };
            let answer = if left == right { "equal" } else { "not_equal" };
            (
                format!("Is equal {left} and {right}?"),
                answer.to_string(),
                vec![answer.to_string()],
                vec!["equal_compare".to_string()],
            )
        } else if concept.concept_id.contains("left_right") {
            let row = if item_type == "teaching" {
                ("apple", "banana", "cherry")
            } else {
                ("pencil", "eraser", "book")
            };
            let asks_left = item_number % 2 == 1;
            let target = row.1;
            let answer = if asks_left { row.0 } else { row.2 };
            (
                format!(
                    "What is {} of {} in the row: {}, {}, {}.",
                    if asks_left { "left" } else { "right" },
                    target,
                    row.0,
                    row.1,
                    row.2
                ),
                answer.to_string(),
                vec![answer.to_string()],
                vec!["left_right_selector".to_string()],
            )
        } else if concept.concept_id.contains("inside_outside") {
            let inside = if item_type == "teaching" { "coin" } else { "shell" };
            let outside = if item_type == "teaching" { "key" } else { "stone" };
            let asks_inside = item_number % 2 == 1;
            let answer = if asks_inside { inside } else { outside };
            (
                format!(
                    "Which item is {} the box? inside: {}, outside: {}.",
                    if asks_inside { "inside" } else { "outside" },
                    inside,
                    outside
                ),
                answer.to_string(),
                vec![answer.to_string()],
                vec!["inside_outside_selector".to_string()],
            )
        } else if item_type == "teaching" {
            (
                format!(
                    "Approved memory item {} for {} concept {}.",
                    item_number,
                    domain_label,
                    concept_label
                ),
                format!("{concept_token}_answer_{item_number}"),
                vec![format!("{concept_token}_answer_{item_number}")],
                vec!["exact_match_lookup".to_string()],
            )
        } else {
            (
                format!(
                    "Probe item {} for {} concept {} requires a deterministic response.",
                    item_number,
                    domain_label,
                    concept_label
                ),
                format!("{concept_token}_probe_{item_number}"),
                vec![format!("{concept_token}_probe_{item_number}")],
                vec!["exact_match_lookup".to_string()],
            )
        };

    CurriculumItem {
        item_id: format!(
            "{}-{}-{}-{:03}",
            concept.domain_id, concept.concept_id, item_type, item_number
        ),
        domain_id: concept.domain_id.clone(),
        concept_id: concept.concept_id.clone(),
        item_type: item_type.to_string(),
        prompt,
        expected_answer: Some(expected_answer),
        acceptable_answers,
        answer_format: Some("short_text".to_string()),
        teaching_context: Some(format!("Scaffold for {}", concept_label)),
        difficulty: if item_type == "teaching" { 1 } else { 2 },
        surface_domain: concept.domain_id.clone(),
        intended_relations: vec![concept.concept_id.clone()],
        expected_skills,
        novelty_class: if item_type == "teaching" {
            "familiar".to_string()
        } else if is_transfer_probe {
            "novel_surface".to_string()
        } else {
            "slightly_varied".to_string()
        },
        probe_role: match item_type {
            "probe_boundary" => "boundary".to_string(),
            "probe_near_transfer" => "transfer".to_string(),
            _ => "none".to_string(),
        },
        boundary_kind: boundary_kind.to_string(),
        representation_type: "text".to_string(),
        composition_parts: Vec::new(),
        notes: Some("Mock teacher curriculum item".to_string()),
        created_by: created_by.to_string(),
        created_at: created_at.to_string(),
        curriculum_version: "0.1.0".to_string(),
        schema_version: "0.1.0".to_string(),
    }
}

fn is_skill_backed_concept(concept_id: &str) -> bool {
    [
        "before_after",
        "same_vs_different",
        "visual_notice",
        "signal_pickout",
        "exact_match",
        "first_last",
        "more_less",
        "equal_compare",
        "left_right",
        "inside_outside",
    ]
    .iter()
    .any(|needle| concept_id.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_curriculum_validates_and_meets_expected_counts() {
        let request = CurriculumRequest::from_size_hint(
            "session-smoke".to_string(),
            "smoke".to_string(),
            "smoke",
        );

        let curriculum = build_mock_curriculum(&request, "mock");
        let validation = curriculum.validate().expect("smoke curriculum should validate");
        let summary = curriculum.summary();

        assert!(validation.valid);
        assert_eq!(summary.domain_count, 5);
        assert_eq!(summary.concept_count, 10);
        assert_eq!(summary.item_count, 30);
        assert_eq!(summary.probe_count, 10);
    }

    #[test]
    fn full_curriculum_expands_beyond_toy_size() {
        let request = CurriculumRequest::from_size_hint(
            "session-full".to_string(),
            "full".to_string(),
            "full",
        );

        let curriculum = build_mock_curriculum(&request, "mock");
        let summary = curriculum.summary();

        assert_eq!(summary.domain_count, 12);
        assert_eq!(summary.concept_count, 60);
        assert_eq!(summary.item_count, 480);
        assert_eq!(summary.probe_count, 180);
    }
}
