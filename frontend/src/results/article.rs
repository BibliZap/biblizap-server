use serde::{Deserialize, Serialize};

use crate::results::Filters;

/// Struct representing an academic article with relevant metadata.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Article {
    pub first_author: Option<String>,
    pub year_published: Option<i32>,
    pub journal: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub doi: Option<String>,
    pub citations: Option<i32>,
    pub score: Option<i32>,
}

impl Article {
    /// Checks if any field in the article matches the given pattern (case-insensitive).
    pub fn matches_global(&self, pattern: &str) -> bool {
        let pattern_lowercase = pattern.to_lowercase();
        self.doi
            .as_ref()
            .map_or(false, |x| x.to_lowercase().contains(&pattern_lowercase))
            | self
                .title
                .as_ref()
                .map_or(false, |x| x.to_lowercase().contains(&pattern_lowercase))
            | self
                .journal
                .as_ref()
                .map_or(false, |x| x.to_lowercase().contains(&pattern_lowercase))
            | self
                .summary
                .as_ref()
                .map_or(false, |x| x.to_lowercase().contains(&pattern_lowercase))
            | self
                .first_author
                .as_ref()
                .map_or(false, |x| x.to_lowercase().contains(&pattern_lowercase))
            | self
                .year_published
                .map_or(false, |x| x.to_string().contains(&pattern_lowercase))
            | self
                .score
                .map_or(false, |x| x.to_string().contains(&pattern_lowercase))
            | self
                .citations
                .map_or(false, |x| x.to_string().contains(&pattern_lowercase))
    }

    /// Checks if the article matches all the provided filters (case-insensitive for strings).
    pub fn matches(&self, filters: &Filters) -> bool {
        self.doi.as_ref().map_or(false, |x| {
            x.to_lowercase().contains(&filters.doi.to_lowercase())
        }) & self.title.as_ref().map_or(false, |x| {
            x.to_lowercase().contains(&filters.title.to_lowercase())
        }) & self.journal.as_ref().map_or(false, |x| {
            x.to_lowercase().contains(&filters.journal.to_lowercase())
        }) & self.summary.as_ref().map_or(false, |x| {
            x.to_lowercase().contains(&filters.summary.to_lowercase())
        }) & self.first_author.as_ref().map_or(false, |x| {
            x.to_lowercase()
                .contains(&filters.first_author.to_lowercase())
        }) & self
            .year_published
            .map_or(false, |x| x.to_string().contains(&filters.year_published))
            & self
                .score
                .map_or(false, |x| x.to_string().contains(&filters.score))
            & self
                .citations
                .map_or(false, |x| x.to_string().contains(&filters.citations))
    }
}
