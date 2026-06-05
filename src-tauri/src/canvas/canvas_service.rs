use std::collections::HashSet;

/// Extracts and deduplicates tags from a comma-separated string.
pub fn extract_tags(tags_str: &str) -> Vec<String> {
    if tags_str.is_empty() {
        return vec![];
    }
    tags_str
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

/// Computes a deterministic cache key from sorted tags and source file IDs.
pub fn compute_cache_key(tags: &[String], source_file_ids: &[String]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    let mut sorted_tags: Vec<&str> = tags.iter().map(|s| s.as_str()).collect();
    sorted_tags.sort();
    let mut sorted_ids: Vec<&str> = source_file_ids.iter().map(|s| s.as_str()).collect();
    sorted_ids.sort();
    hasher.update(sorted_tags.join(","));
    hasher.update("|");
    hasher.update(sorted_ids.join(","));
    format!("{:x}", hasher.finalize())
}
