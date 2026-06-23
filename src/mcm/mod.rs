use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::{McmConfig, SkillApprovals, SkillManifest};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McmTraceEvent {
    pub trace_id: String,
    pub session_id: String,
    pub item_id: String,
    pub input_normalization: String,
    pub surface_features: Vec<String>,
    pub memory_reads: Vec<String>,
    pub candidate_skills: Vec<String>,
    pub approved_skills: Vec<String>,
    pub blocked_skills: Vec<String>,
    pub executed_skill: Option<String>,
    pub partial_attempt: Option<String>,
    pub refusal_reason: Option<String>,
    pub reasoning_steps: Vec<String>,
    pub policy_checks: Vec<String>,
    pub uncertainty_state: String,
    pub final_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct McmAnswer {
    pub answer: Option<String>,
    pub trace: McmTraceEvent,
}

#[derive(Debug, Clone)]
pub struct StudentEngine {
    config: McmConfig,
    manifest: SkillManifest,
    approvals: SkillApprovals,
    memory: ExplicitMemoryStore,
}

impl StudentEngine {
    pub fn new(
        config: McmConfig,
        manifest: SkillManifest,
        approvals: SkillApprovals,
        memory: ExplicitMemoryStore,
    ) -> Self {
        Self {
            config,
            manifest,
            approvals,
            memory,
        }
    }

    pub fn answer(&self, session_id: &str, item_id: &str, prompt: &str) -> McmAnswer {
        let normalized = normalize_prompt(prompt);
        let features = classify_surface_features(&normalized);
        let mut memory_reads = Vec::new();
        let mut reasoning_steps = vec![
            "received_question".to_string(),
            "normalized_input".to_string(),
            "classified_surface_features".to_string(),
        ];

        let mut candidate_skills = self.match_candidate_skills(&features);
        if self.memory.contains_exact(&normalized) {
            candidate_skills.insert(0, "exact_match_lookup".to_string());
        }
        candidate_skills.dedup();

        let approved_skills: Vec<String> = candidate_skills
            .iter()
            .filter(|skill| self.approvals.approved_skill_ids.contains(skill))
            .cloned()
            .collect();
        let blocked_skills: Vec<String> = candidate_skills
            .iter()
            .filter(|skill| self.approvals.blocked_skill_ids.contains(skill))
            .cloned()
            .collect();

        if self.memory.contains_exact(&normalized) {
            memory_reads.push(normalized.clone());
        }

        let mut answer = None;
        let mut executed_skill = None;
        let mut refusal_reason = None;
        let mut partial_attempt = None;
        let uncertainty_state;
        let final_mode;

        if approved_skills.iter().any(|s| s == "exact_match_lookup") {
            executed_skill = Some("exact_match_lookup".to_string());
            answer = self.memory.lookup_exact(&normalized);
            reasoning_steps.push("answered_from_explicit_memory".to_string());
            uncertainty_state = "low".to_string();
            final_mode = "answered_from_memory".to_string();
        } else if approved_skills
            .iter()
            .any(|s| s == "ordered_relation_compare")
        {
            executed_skill = Some("ordered_relation_compare".to_string());
            answer = ordered_relation_compare(prompt);
            if answer.is_some() {
                reasoning_steps.push("executed_ordered_relation_compare".to_string());
                uncertainty_state = "medium".to_string();
                final_mode = "answered_with_skill".to_string();
            } else {
                partial_attempt = Some("identified ordering language but could not resolve a deterministic answer".to_string());
                refusal_reason = Some("insufficient deterministic structure".to_string());
                reasoning_steps.push("order_skill_failed_to_resolve".to_string());
                uncertainty_state = "high".to_string();
                final_mode = "refused".to_string();
            }
        } else if approved_skills
            .iter()
            .any(|s| s == "same_different_compare")
        {
            executed_skill = Some("same_different_compare".to_string());
            answer = same_different_compare(prompt);
            if answer.is_some() {
                reasoning_steps.push("executed_same_different_compare".to_string());
                uncertainty_state = "medium".to_string();
                final_mode = "answered_with_skill".to_string();
            } else {
                partial_attempt =
                    Some("identified comparison language but could not resolve sameness deterministically".to_string());
                refusal_reason = Some("insufficient deterministic comparison structure".to_string());
                reasoning_steps.push("comparison_skill_failed_to_resolve".to_string());
                uncertainty_state = "high".to_string();
                final_mode = "refused".to_string();
            }
        } else if approved_skills.iter().any(|s| s == "first_last_selector") {
            executed_skill = Some("first_last_selector".to_string());
            answer = first_last_selector(prompt);
            if answer.is_some() {
                reasoning_steps.push("executed_first_last_selector".to_string());
                uncertainty_state = "medium".to_string();
                final_mode = "answered_with_skill".to_string();
            } else {
                partial_attempt =
                    Some("identified list-order language but could not resolve first/last deterministically".to_string());
                refusal_reason = Some("insufficient deterministic list-order structure".to_string());
                reasoning_steps.push("list_order_skill_failed_to_resolve".to_string());
                uncertainty_state = "high".to_string();
                final_mode = "refused".to_string();
            }
        } else if approved_skills.iter().any(|s| s == "more_less_compare") {
            executed_skill = Some("more_less_compare".to_string());
            answer = more_less_compare(prompt);
            if answer.is_some() {
                reasoning_steps.push("executed_more_less_compare".to_string());
                uncertainty_state = "medium".to_string();
                final_mode = "answered_with_skill".to_string();
            } else {
                partial_attempt =
                    Some("identified quantity language but could not resolve comparison deterministically".to_string());
                refusal_reason = Some("insufficient deterministic quantity structure".to_string());
                reasoning_steps.push("quantity_skill_failed_to_resolve".to_string());
                uncertainty_state = "high".to_string();
                final_mode = "refused".to_string();
            }
        } else if approved_skills.iter().any(|s| s == "equal_compare") {
            executed_skill = Some("equal_compare".to_string());
            answer = equal_compare(prompt);
            if answer.is_some() {
                reasoning_steps.push("executed_equal_compare".to_string());
                uncertainty_state = "medium".to_string();
                final_mode = "answered_with_skill".to_string();
            } else {
                partial_attempt =
                    Some("identified equality language but could not resolve it deterministically".to_string());
                refusal_reason = Some("insufficient deterministic equality structure".to_string());
                reasoning_steps.push("equal_compare_skill_failed_to_resolve".to_string());
                uncertainty_state = "high".to_string();
                final_mode = "refused".to_string();
            }
        } else if approved_skills.iter().any(|s| s == "left_right_selector") {
            executed_skill = Some("left_right_selector".to_string());
            answer = left_right_selector(prompt);
            if answer.is_some() {
                reasoning_steps.push("executed_left_right_selector".to_string());
                uncertainty_state = "medium".to_string();
                final_mode = "answered_with_skill".to_string();
            } else {
                partial_attempt =
                    Some("identified spatial row language but could not resolve neighbor deterministically".to_string());
                refusal_reason = Some("insufficient deterministic spatial structure".to_string());
                reasoning_steps.push("spatial_skill_failed_to_resolve".to_string());
                uncertainty_state = "high".to_string();
                final_mode = "refused".to_string();
            }
        } else if approved_skills.iter().any(|s| s == "option_match_selector") {
            executed_skill = Some("option_match_selector".to_string());
            answer = option_match_selector(prompt);
            if answer.is_some() {
                reasoning_steps.push("executed_option_match_selector".to_string());
                uncertainty_state = "medium".to_string();
                final_mode = "answered_with_skill".to_string();
            } else {
                partial_attempt = Some(
                    "identified match-selection language but could not resolve the target deterministically"
                        .to_string(),
                );
                refusal_reason =
                    Some("insufficient deterministic match-selection structure".to_string());
                reasoning_steps.push("option_match_skill_failed_to_resolve".to_string());
                uncertainty_state = "high".to_string();
                final_mode = "refused".to_string();
            }
        } else if approved_skills.iter().any(|s| s == "inside_outside_selector") {
            executed_skill = Some("inside_outside_selector".to_string());
            answer = inside_outside_selector(prompt);
            if answer.is_some() {
                reasoning_steps.push("executed_inside_outside_selector".to_string());
                uncertainty_state = "medium".to_string();
                final_mode = "answered_with_skill".to_string();
            } else {
                partial_attempt = Some(
                    "identified inside-outside language but could not resolve the target deterministically"
                        .to_string(),
                );
                refusal_reason =
                    Some("insufficient deterministic inside-outside structure".to_string());
                reasoning_steps.push("inside_outside_skill_failed_to_resolve".to_string());
                uncertainty_state = "high".to_string();
                final_mode = "refused".to_string();
            }
        } else {
            refusal_reason = Some("no approved deterministic skill path".to_string());
            reasoning_steps.push("refused_without_skill_path".to_string());
            uncertainty_state = "high".to_string();
            final_mode = "refused".to_string();
        }

        let policy_checks = vec![
            format!("deterministic_only={}", self.config.deterministic_only),
            format!(
                "approved_candidate_count={}",
                approved_skills.len()
            ),
            format!("manifest_skill_count={}", self.manifest.skills.len()),
        ];

        let trace = McmTraceEvent {
            trace_id: format!("trace-{session_id}-{item_id}"),
            session_id: session_id.to_string(),
            item_id: item_id.to_string(),
            input_normalization: normalized,
            surface_features: features,
            memory_reads,
            candidate_skills,
            approved_skills,
            blocked_skills,
            executed_skill,
            partial_attempt,
            refusal_reason,
            reasoning_steps,
            policy_checks,
            uncertainty_state,
            final_mode,
        };

        McmAnswer { answer, trace }
    }

    fn match_candidate_skills(&self, features: &[String]) -> Vec<String> {
        let mut skills = Vec::new();

        if features.iter().any(|feature| feature == "ordering_language") {
            skills.push("ordered_relation_compare".to_string());
        }
        if features.iter().any(|feature| feature == "comparison_language") {
            skills.push("same_different_compare".to_string());
        }
        if features.iter().any(|feature| feature == "list_order_language") {
            skills.push("first_last_selector".to_string());
        }
        if features.iter().any(|feature| feature == "quantity_language") {
            skills.push("more_less_compare".to_string());
        }
        if features.iter().any(|feature| feature == "equality_language") {
            skills.push("equal_compare".to_string());
        }
        if features.iter().any(|feature| feature == "spatial_row_language") {
            skills.push("left_right_selector".to_string());
        }
        if features
            .iter()
            .any(|feature| feature == "match_selection_language")
        {
            skills.push("option_match_selector".to_string());
        }
        if features
            .iter()
            .any(|feature| feature == "inside_outside_language")
        {
            skills.push("inside_outside_selector".to_string());
        }

        skills
    }
}

#[derive(Debug, Clone, Default)]
pub struct ExplicitMemoryStore {
    exact_answers: HashMap<String, String>,
}

impl ExplicitMemoryStore {
    pub fn with_exact_entries(entries: impl IntoIterator<Item = (String, String)>) -> Self {
        Self {
            exact_answers: entries.into_iter().collect(),
        }
    }

    pub fn contains_exact(&self, key: &str) -> bool {
        self.exact_answers.contains_key(key)
    }

    pub fn lookup_exact(&self, key: &str) -> Option<String> {
        self.exact_answers.get(key).cloned()
    }
}

fn normalize_prompt(prompt: &str) -> String {
    prompt.trim().to_ascii_lowercase()
}

fn classify_surface_features(normalized: &str) -> Vec<String> {
    let mut features = Vec::new();
    if normalized.contains("before") || normalized.contains("after") {
        features.push("ordering_language".to_string());
    }
    if normalized.contains("same") || normalized.contains("different") {
        features.push("comparison_language".to_string());
    }
    if normalized.contains("which is first")
        || normalized.contains("which is last")
        || normalized.contains("list:")
    {
        features.push("list_order_language".to_string());
    }
    if normalized.contains("more")
        || normalized.contains("less")
        || normalized.contains("greater")
        || normalized.contains("fewer")
    {
        features.push("quantity_language".to_string());
    }
    if normalized.contains("equal")
        || normalized.contains("same number")
        || normalized.contains("same amount")
    {
        features.push("equality_language".to_string());
    }
    if normalized.contains("left of")
        || normalized.contains("right of")
        || normalized.contains("row:")
    {
        features.push("spatial_row_language".to_string());
    }
    if normalized.contains("matches sample")
        || normalized.contains("matches target")
        || normalized.contains("options:")
    {
        features.push("match_selection_language".to_string());
    }
    if normalized.contains("inside the box")
        || normalized.contains("outside the box")
        || normalized.contains("inside:")
        || normalized.contains("outside:")
    {
        features.push("inside_outside_language".to_string());
    }
    if features.is_empty() {
        features.push("unknown_surface".to_string());
    }
    features
}

fn ordered_relation_compare(prompt: &str) -> Option<String> {
    let normalized = normalize_prompt(prompt);
    let tokens: Vec<&str> = normalized
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect();

    if tokens.len() < 4 || !tokens.contains(&"before") {
        return None;
    }

    let before_index = tokens.iter().position(|token| *token == "before")?;
    if before_index == 0 || before_index + 1 >= tokens.len() {
        return None;
    }

    let mut answer_index = before_index - 1;
    if matches!(tokens[answer_index], "comes" | "is") && answer_index > 0 {
        answer_index -= 1;
    }

    Some(tokens[answer_index].to_string())
}

fn same_different_compare(prompt: &str) -> Option<String> {
    let normalized = normalize_prompt(prompt);
    let tokens: Vec<&str> = normalized
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect();

    let is_index = tokens.iter().position(|token| *token == "is")?;
    let and_index = tokens.iter().position(|token| *token == "and")?;
    if and_index <= is_index || and_index + 1 >= tokens.len() {
        return None;
    }

    let left = tokens.get(is_index + 1)?;
    let right = tokens.get(and_index + 1)?;
    Some(if left == right { "same" } else { "different" }.to_string())
}

fn first_last_selector(prompt: &str) -> Option<String> {
    let normalized = normalize_prompt(prompt);
    let list_index = normalized.find("list:")?;
    let question_part = &normalized[..list_index];
    let list_part = &normalized[list_index + 5..];
    let wants_first = question_part.contains("which is first");
    let wants_last = question_part.contains("which is last");

    if !wants_first && !wants_last {
        return None;
    }

    let items: Vec<String> = list_part
        .split([',', '.'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect();

    if items.is_empty() {
        return None;
    }

    if wants_first {
        items.first().cloned()
    } else {
        items.last().cloned()
    }
}

fn more_less_compare(prompt: &str) -> Option<String> {
    let normalized = normalize_prompt(prompt);
    let tokens: Vec<&str> = normalized
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect();

    if tokens.len() < 4 {
        return None;
    }

    let comparator_index =
        tokens
            .iter()
            .position(|token| matches!(*token, "more" | "greater" | "fewer" | "less"))?;
    if comparator_index + 3 >= tokens.len() {
        return None;
    }

    let left_token = tokens[comparator_index + 1];
    let bridge_token = tokens[comparator_index + 2];
    let right_token = tokens[comparator_index + 3];
    if !matches!(bridge_token, "or" | "than") {
        return None;
    }

    let left = parse_number_word(left_token)?;
    let right = parse_number_word(right_token)?;

    let wants_more = matches!(tokens[comparator_index], "more" | "greater");
    Some(if wants_more {
        if left >= right {
            left_token.to_string()
        } else {
            right_token.to_string()
        }
    } else if left <= right {
        left_token.to_string()
    } else {
        right_token.to_string()
    })
}

fn equal_compare(prompt: &str) -> Option<String> {
    let normalized = normalize_prompt(prompt);
    let tokens: Vec<&str> = normalized
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect();

    let equal_index = tokens.iter().position(|token| *token == "equal")?;
    if equal_index + 3 >= tokens.len() {
        return None;
    }

    let left_token = tokens[equal_index + 1];
    let bridge_token = tokens[equal_index + 2];
    let right_token = tokens[equal_index + 3];
    if bridge_token != "and" {
        return None;
    }

    let left = parse_number_word(left_token)?;
    let right = parse_number_word(right_token)?;
    Some(if left == right { "equal" } else { "not_equal" }.to_string())
}

fn parse_number_word(token: &str) -> Option<u32> {
    match token {
        "0" | "zero" => Some(0),
        "1" | "one" => Some(1),
        "2" | "two" => Some(2),
        "3" | "three" => Some(3),
        "4" | "four" => Some(4),
        "5" | "five" => Some(5),
        "6" | "six" => Some(6),
        "7" | "seven" => Some(7),
        "8" | "eight" => Some(8),
        "9" | "nine" => Some(9),
        "10" | "ten" => Some(10),
        _ => None,
    }
}

fn left_right_selector(prompt: &str) -> Option<String> {
    let normalized = normalize_prompt(prompt);
    let row_index = normalized.find("row:")?;
    let question_part = normalized[..row_index].trim();
    let row_part = normalized[row_index + 4..].trim();
    let items: Vec<String> = row_part
        .split([',', '.'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect();
    if items.len() < 2 {
        return None;
    }

    let wants_left = question_part.contains("left of");
    let wants_right = question_part.contains("right of");
    if !wants_left && !wants_right {
        return None;
    }

    let target = if wants_left {
        question_part.split("left of").nth(1)?.trim()
    } else {
        question_part.split("right of").nth(1)?.trim()
    };
    let target = target
        .split("in the")
        .next()
        .unwrap_or(target)
        .trim_end_matches('?')
        .trim();
    let target_index = items.iter().position(|item| item == target)?;

    if wants_left {
        target_index.checked_sub(1).and_then(|idx| items.get(idx).cloned())
    } else {
        items.get(target_index + 1).cloned()
    }
}

fn option_match_selector(prompt: &str) -> Option<String> {
    let normalized = normalize_prompt(prompt);
    let sample = normalized
        .split("matches sample")
        .nth(1)
        .or_else(|| normalized.split("matches target").nth(1))?;
    let sample = sample
        .split('?')
        .next()
        .unwrap_or(sample)
        .split("options:")
        .next()
        .unwrap_or(sample)
        .trim()
        .trim_end_matches(':')
        .trim();
    let options = normalized.split("options:").nth(1)?;
    let options: Vec<String> = options
        .split([',', '.'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect();
    options.into_iter().find(|option| option == sample)
}

fn inside_outside_selector(prompt: &str) -> Option<String> {
    let normalized = normalize_prompt(prompt);
    let wants_inside = normalized.contains("inside the box");
    let wants_outside = normalized.contains("outside the box");
    if !wants_inside && !wants_outside {
        return None;
    }

    let inside = normalized
        .split("inside:")
        .nth(1)?
        .split(',')
        .next()?
        .trim()
        .trim_end_matches('.');
    let outside = normalized
        .split("outside:")
        .nth(1)?
        .split([',', '.'])
        .next()?
        .trim();

    if wants_inside {
        Some(inside.to_string())
    } else {
        Some(outside.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{ExplicitMemoryStore, StudentEngine};
    use crate::config::{McmConfig, SkillApprovals, SkillManifest, SkillManifestEntry};

    fn engine(approved_skills: Vec<&str>) -> StudentEngine {
        StudentEngine::new(
            McmConfig {
                class_label: "MCM".to_string(),
                deterministic_only: true,
                refusal_mode: "strong".to_string(),
                memory_store: "explicit".to_string(),
                policy_version: "0.1.0".to_string(),
            },
            SkillManifest {
                manifest_version: "0.1.0".to_string(),
                skills: vec![
                    SkillManifestEntry {
                        skill_id: "exact_match_lookup".to_string(),
                        description: "lookup".to_string(),
                        deterministic: true,
                    },
                    SkillManifestEntry {
                        skill_id: "ordered_relation_compare".to_string(),
                        description: "ordering".to_string(),
                        deterministic: true,
                    },
                    SkillManifestEntry {
                        skill_id: "same_different_compare".to_string(),
                        description: "comparison".to_string(),
                        deterministic: true,
                    },
                    SkillManifestEntry {
                        skill_id: "first_last_selector".to_string(),
                        description: "list order".to_string(),
                        deterministic: true,
                    },
                    SkillManifestEntry {
                        skill_id: "more_less_compare".to_string(),
                        description: "quantity comparison".to_string(),
                        deterministic: true,
                    },
                    SkillManifestEntry {
                        skill_id: "equal_compare".to_string(),
                        description: "quantity equality".to_string(),
                        deterministic: true,
                    },
                    SkillManifestEntry {
                        skill_id: "left_right_selector".to_string(),
                        description: "spatial row neighbor".to_string(),
                        deterministic: true,
                    },
                    SkillManifestEntry {
                        skill_id: "option_match_selector".to_string(),
                        description: "match selection".to_string(),
                        deterministic: true,
                    },
                    SkillManifestEntry {
                        skill_id: "inside_outside_selector".to_string(),
                        description: "inside outside".to_string(),
                        deterministic: true,
                    },
                ],
            },
            SkillApprovals {
                approvals_version: "0.1.0".to_string(),
                approved_skill_ids: approved_skills.into_iter().map(str::to_string).collect(),
                blocked_skill_ids: vec!["forbidden_skill".to_string()],
            },
            ExplicitMemoryStore::with_exact_entries([(
                "what color is the stop sign?".to_string(),
                "red".to_string(),
            )]),
        )
    }

    #[test]
    fn identical_inputs_produce_identical_outputs() {
        let engine = engine(vec!["exact_match_lookup"]);
        let first = engine.answer("s1", "i1", "What color is the stop sign?");
        let second = engine.answer("s1", "i1", "What color is the stop sign?");

        assert_eq!(first.answer, second.answer);
        assert_eq!(first.trace.final_mode, second.trace.final_mode);
        assert_eq!(first.trace.policy_checks, second.trace.policy_checks);
    }

    #[test]
    fn blocked_or_missing_skill_path_refuses() {
        let engine = engine(vec![]);
        let response = engine.answer("s1", "i2", "A comes before B. What comes first?");

        assert!(response.answer.is_none());
        assert_eq!(response.trace.final_mode, "refused");
        assert_eq!(
            response.trace.refusal_reason.as_deref(),
            Some("no approved deterministic skill path")
        );
    }

    #[test]
    fn ordering_skill_answers_without_any_teacher_dependency() {
        let engine = engine(vec!["ordered_relation_compare"]);
        let response = engine.answer("s1", "i3", "A comes before B. What comes first?");

        assert_eq!(response.answer.as_deref(), Some("a"));
        assert_eq!(response.trace.executed_skill.as_deref(), Some("ordered_relation_compare"));
    }

    #[test]
    fn same_different_skill_answers_deterministically() {
        let engine = engine(vec!["same_different_compare"]);
        let response = engine.answer("s1", "i4", "Is red and blue the same or different?");

        assert_eq!(response.answer.as_deref(), Some("different"));
        assert_eq!(response.trace.executed_skill.as_deref(), Some("same_different_compare"));
    }

    #[test]
    fn first_last_skill_answers_deterministically() {
        let engine = engine(vec!["first_last_selector"]);
        let response = engine.answer(
            "s1",
            "i5",
            "Which is first in the list: cat, dog, bird.",
        );

        assert_eq!(response.answer.as_deref(), Some("cat"));
        assert_eq!(response.trace.executed_skill.as_deref(), Some("first_last_selector"));
    }

    #[test]
    fn quantity_skill_answers_deterministically() {
        let engine = engine(vec!["more_less_compare"]);
        let response = engine.answer("s1", "i6", "Which is more 3 or 5?");

        assert_eq!(response.answer.as_deref(), Some("5"));
        assert_eq!(response.trace.executed_skill.as_deref(), Some("more_less_compare"));
    }

    #[test]
    fn equal_compare_skill_answers_deterministically() {
        let engine = engine(vec!["equal_compare"]);
        let response = engine.answer("s1", "i6b", "Is equal 4 and 4?");

        assert_eq!(response.answer.as_deref(), Some("equal"));
        assert_eq!(response.trace.executed_skill.as_deref(), Some("equal_compare"));
    }

    #[test]
    fn left_right_skill_answers_deterministically() {
        let engine = engine(vec!["left_right_selector"]);
        let response = engine.answer(
            "s1",
            "i7",
            "What is left of banana in the row: apple, banana, cherry.",
        );

        assert_eq!(response.answer.as_deref(), Some("apple"));
        assert_eq!(response.trace.executed_skill.as_deref(), Some("left_right_selector"));
    }

    #[test]
    fn option_match_skill_answers_deterministically() {
        let engine = engine(vec!["option_match_selector"]);
        let response = engine.answer(
            "s1",
            "i8",
            "Which option matches sample red? options: red, blue, green.",
        );

        assert_eq!(response.answer.as_deref(), Some("red"));
        assert_eq!(response.trace.executed_skill.as_deref(), Some("option_match_selector"));
    }

    #[test]
    fn inside_outside_skill_answers_deterministically() {
        let engine = engine(vec!["inside_outside_selector"]);
        let response = engine.answer(
            "s1",
            "i9",
            "Which item is inside the box? inside: coin, outside: key.",
        );

        assert_eq!(response.answer.as_deref(), Some("coin"));
        assert_eq!(response.trace.executed_skill.as_deref(), Some("inside_outside_selector"));
    }
}
