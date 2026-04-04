use std::time::{SystemTime, UNIX_EPOCH};

/// Validates if a string is a valid DOI.
/// DOIs start with "10." followed by at least 4 digits, a "/", and a suffix.
pub fn is_valid_doi(s: &str) -> bool {
    s.starts_with("10.") 
        && s.len() > 7  // Minimum: "10.1234/x"
        && s.contains('/') 
        && s.chars().skip(3).take_while(|c| c.is_ascii_digit()).count() >= 4
}

/// Validates if a string is a valid PMID.
/// PMIDs are purely numeric identifiers.
pub fn is_valid_pmid(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// Validates if a string is either a valid DOI or PMID.
pub fn is_valid_id(s: &str) -> bool {
    is_valid_doi(s) || is_valid_pmid(s)
}

pub fn epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}