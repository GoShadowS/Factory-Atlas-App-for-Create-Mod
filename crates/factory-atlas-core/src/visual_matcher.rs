use crate::model::{MatchReason, VisualEntrySummary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualMatchStatus {
    Unique,
    Ambiguous,
    NotFound,
}

#[derive(Debug, Clone)]
pub struct VisualMatchResult {
    pub status: VisualMatchStatus,
    pub reason: MatchReason,
    pub best: Option<VisualEntrySummary>,
    pub candidates: Vec<VisualEntrySummary>,
}

pub fn normalize_visual_query(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn tokenize_query(input: &str) -> Vec<String> {
    input
        .split(|c: char| !c.is_alphanumeric())
        .map(|s| s.to_lowercase())
        .filter(|t| t.chars().count() >= 2)
        .collect()
}

/// Deterministic 5-tier lookup across the local visual reference index.
/// Strictly NEVER creates resources, machines, or recipes automatically.
pub fn match_visual_resources(
    query: &str,
    entries: &[VisualEntrySummary],
    max_candidates: usize,
) -> VisualMatchResult {
    let raw = query.trim();
    if raw.is_empty() || entries.is_empty() {
        return VisualMatchResult {
            status: VisualMatchStatus::NotFound,
            reason: MatchReason::None,
            best: None,
            candidates: Vec::new(),
        };
    }

    let lower = raw.to_lowercase();

    // 1. Exact namespaced ID (`create:cogwheel`)
    let exact_id: Vec<VisualEntrySummary> = entries
        .iter()
        .filter(|e| e.id.to_lowercase() == lower)
        .cloned()
        .collect();
    if exact_id.len() == 1 {
        return VisualMatchResult {
            status: VisualMatchStatus::Unique,
            reason: MatchReason::ExactId,
            best: exact_id.into_iter().next(),
            candidates: Vec::new(),
        };
    }
    if exact_id.len() > 1 {
        return VisualMatchResult {
            status: VisualMatchStatus::Ambiguous,
            reason: MatchReason::ExactId,
            best: None,
            candidates: exact_id.into_iter().take(max_candidates).collect(),
        };
    }

    // 2. Exact model, blockstate, or texture path
    let clean_path = lower
        .trim_start_matches('/')
        .trim_start_matches("local-originals/")
        .trim_start_matches("assets/")
        .trim_end_matches(".json")
        .trim_end_matches(".png");

    let exact_path: Vec<VisualEntrySummary> = entries
        .iter()
        .filter(|e| {
            [
                e.model_path.as_deref(),
                e.world_model_path.as_deref(),
                e.texture_path.as_deref(),
                e.blockstate_path.as_deref(),
                e.model_ref.as_deref(),
                e.world_model_ref.as_deref(),
            ]
            .into_iter()
            .flatten()
            .any(|p| {
                let lp = p.to_lowercase();
                let np = lp.trim_end_matches(".json").trim_end_matches(".png");
                clean_path.contains('/')
                    && (np == clean_path || np.ends_with(&format!("/{clean_path}")))
            })
        })
        .cloned()
        .collect();
    if exact_path.len() == 1 {
        return VisualMatchResult {
            status: VisualMatchStatus::Unique,
            reason: MatchReason::ExactPath,
            best: exact_path.into_iter().next(),
            candidates: Vec::new(),
        };
    }
    if exact_path.len() > 1 {
        return VisualMatchResult {
            status: VisualMatchStatus::Ambiguous,
            reason: MatchReason::ExactPath,
            best: None,
            candidates: exact_path.into_iter().take(max_candidates).collect(),
        };
    }

    // 3. Normalized ID / filename
    let norm = normalize_visual_query(raw);
    if norm.chars().count() >= 2 {
        let by_norm: Vec<VisualEntrySummary> = entries
            .iter()
            .filter(|e| e.normalized_key == norm || normalize_visual_query(&e.id) == norm)
            .cloned()
            .collect();
        if by_norm.len() == 1 {
            return VisualMatchResult {
                status: VisualMatchStatus::Unique,
                reason: MatchReason::Normalized,
                best: by_norm.into_iter().next(),
                candidates: Vec::new(),
            };
        }
        if by_norm.len() > 1 {
            return VisualMatchResult {
                status: VisualMatchStatus::Ambiguous,
                reason: MatchReason::Normalized,
                best: None,
                candidates: by_norm.into_iter().take(max_candidates).collect(),
            };
        }

        // 4. Exact localized RU or EN name
        let by_lang: Vec<VisualEntrySummary> = entries
            .iter()
            .filter(|e| {
                (!e.normalized_ru.is_empty() && e.normalized_ru == norm)
                    || (!e.normalized_en.is_empty() && e.normalized_en == norm)
            })
            .cloned()
            .collect();
        if by_lang.len() == 1 {
            return VisualMatchResult {
                status: VisualMatchStatus::Unique,
                reason: MatchReason::LangAlias,
                best: by_lang.into_iter().next(),
                candidates: Vec::new(),
            };
        }
        if by_lang.len() > 1 {
            return VisualMatchResult {
                status: VisualMatchStatus::Ambiguous,
                reason: MatchReason::LangAlias,
                best: None,
                candidates: by_lang.into_iter().take(max_candidates).collect(),
            };
        }
    }

    // 5. Token / partial match
    let tokens = tokenize_query(raw);
    if tokens.is_empty() {
        return VisualMatchResult {
            status: VisualMatchStatus::NotFound,
            reason: MatchReason::None,
            best: None,
            candidates: Vec::new(),
        };
    }

    let mut scored: Vec<(usize, &VisualEntrySummary)> = Vec::new();
    for e in entries {
        let hay_id = e.id.to_lowercase();
        let hay_en = e.name_en.to_lowercase();
        let hay_ru = e.name_ru.as_deref().unwrap_or("").to_lowercase();

        let mut hits = 0usize;
        let mut bonus = 0usize;
        for tok in &tokens {
            if hay_ru.contains(tok) || hay_en.contains(tok) || hay_id.contains(tok) {
                hits += 1;
                if hay_ru.starts_with(tok)
                    || hay_en.starts_with(tok)
                    || e.path_id.starts_with(tok)
                {
                    bonus += 2;
                }
            }
        }
        if hits == tokens.len() {
            scored.push((hits * 10 + bonus + usize::from(e.has_model_3d), e));
        }
    }

    if scored.is_empty() {
        return VisualMatchResult {
            status: VisualMatchStatus::NotFound,
            reason: MatchReason::None,
            best: None,
            candidates: Vec::new(),
        };
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.id.cmp(&b.1.id)));
    if scored.len() == 1 {
        return VisualMatchResult {
            status: VisualMatchStatus::Unique,
            reason: MatchReason::Fuzzy,
            best: Some(scored[0].1.clone()),
            candidates: Vec::new(),
        };
    }

    VisualMatchResult {
        status: VisualMatchStatus::Ambiguous,
        reason: MatchReason::Fuzzy,
        best: None,
        candidates: scored
            .into_iter()
            .take(max_candidates)
            .map(|(_, e)| e.clone())
            .collect(),
    }
}
