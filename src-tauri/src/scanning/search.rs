use crate::{models::FileEntry, state::FileIndex};

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

pub fn search_file_entries(query: &str, limit: usize, files: &[FileEntry]) -> Vec<FileEntry> {
    let query = query.trim().to_lowercase();
    let limit = limit.clamp(1, 30);
    if query.is_empty() {
        return files.iter().take(limit).cloned().collect();
    }

    let tokens: Vec<&str> = query.split_whitespace().collect();

    let mut scored: Vec<(i32, &FileEntry)> = files
        .iter()
        .filter_map(|file| {
            let name = &file.search_name;
            let contains_all = tokens.iter().all(|token| file.search_text.contains(token));
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

pub fn search_files(query: &str, limit: usize, index: &FileIndex) -> Vec<FileEntry> {
    if query.trim().is_empty() {
        return Vec::new();
    }

    let files = match index.files.lock() {
        Ok(files) => files,
        Err(_) => return Vec::new(),
    };
    search_file_entries(query, limit, &files)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, path: &str) -> FileEntry {
        FileEntry {
            id: format!("file:{path}"),
            name: name.to_string(),
            path: path.to_string(),
            extension: "txt".to_string(),
            parent: "D:\\Projects".to_string(),
            is_directory: false,
            search_name: name.to_lowercase(),
            search_text: format!("{} {}", name.to_lowercase(), path.to_lowercase()),
        }
    }

    #[test]
    fn path_search_can_list_files_without_query_text() {
        let files = vec![
            file("alpha.txt", "D:\\Projects\\alpha.txt"),
            file("beta.txt", "D:\\Projects\\beta.txt"),
        ];

        let results = search_file_entries("", 8, &files);

        assert_eq!(
            results
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha.txt", "beta.txt"]
        );
    }

    #[test]
    fn path_search_ranks_matching_files_without_requiring_a_full_path() {
        let files = vec![
            file("notes.txt", "D:\\Projects\\notes.txt"),
            file("report.txt", "D:\\Projects\\report.txt"),
        ];

        let results = search_file_entries("repo", 8, &files);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "report.txt");
    }
}
