//! Fuzzy matching used by the file index.

use crate::{models::FileEntry, state::FileIndex};

/// Scores `name` against a query using initials and subsequence matching.
fn fuzzy_name_score(query: &str, name: &str) -> i32 {
    let compact_query: String = query
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let query_length = compact_query.chars().count();
    if query_length < 2 {
        return 0;
    }
    let allow_subsequence = query_length >= 3;

    let compact_name: String = name
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let initials: String = name
        .split(|character: char| !character.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.chars().next())
        .collect();

    if initials == compact_query {
        return 105;
    }
    if initials.starts_with(&compact_query) {
        return 90;
    }
    if initials.contains(&compact_query) {
        return 70;
    }

    if !allow_subsequence {
        return 0;
    }

    let query_chars: Vec<char> = compact_query.chars().collect();
    let mut query_index = 0;
    for character in compact_name.chars() {
        if character == query_chars[query_index] {
            query_index += 1;
            if query_index == query_chars.len() {
                let gaps = compact_name.chars().count() - query_chars.len();
                return (80 - gaps as i32).max(35);
            }
        }
    }

    0
}

/// Returns the best `limit` matches for `query`, best first.
pub fn search_files(query: &str, limit: usize, index: &FileIndex) -> Vec<FileEntry> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }

    let limit = limit.clamp(1, 30);
    let tokens: Vec<&str> = query.split_whitespace().collect();
    let files = match index.files.lock() {
        Ok(files) => files,
        Err(_) => return Vec::new(),
    };

    let mut scored: Vec<(i32, &FileEntry)> = files
        .iter()
        .filter_map(|file| {
            let name = &file.search_name;
            let contains_all = tokens
                .iter()
                .all(|token| file.search_text.contains(token));
            let fuzzy_score = if contains_all {
                0
            } else {
                fuzzy_name_score(&query, name)
            };

            if !contains_all && fuzzy_score == 0 {
                return None;
            }

            let score = if name == &query {
                140
            } else if name.starts_with(&query) {
                110
            } else if name.contains(&query) {
                80
            } else if contains_all {
                45
            } else {
                fuzzy_score
            };

            Some((score - (name.len() as i32 / 8).min(20), file))
        })
        .collect();

    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.name.len().cmp(&b.1.name.len()))
            .then_with(|| a.1.search_name.cmp(&b.1.search_name))
    });

    scored
        .into_iter()
        .take(limit)
        .map(|(_, file)| file.clone())
        .collect()
}
