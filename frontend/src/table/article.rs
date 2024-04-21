use serde::{Deserialize, Serialize};

use crate::table::Filters;

fn unwrap_int(year: &Option<i32>) -> Option<String> {
    Some(format!("{}", (*year)?))
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Article {
    pub first_author: Option<String>,
    pub year_published: Option<i32>,
    pub journal: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub doi: Option<String>,
    pub citations: Option<i32>,
    pub score: Option<i32>
}

impl Article {
    pub fn matches_global(&self, pattern: &str) -> bool {
        self.doi.as_ref().map_or(false, |x| x.contains(pattern)) |
        self.title.as_ref().map_or(false, |x| x.contains(pattern)) |
        self.journal.as_ref().map_or(false, |x| x.contains(pattern)) |
        self.summary.as_ref().map_or(false, |x| x.contains(pattern)) |
        self.first_author.as_ref().map_or(false, |x| x.contains(pattern)) |
        self.year_published.map_or(false, |x| x.to_string().contains(pattern)) |
        self.score.map_or(false, |x| x.to_string().contains(pattern)) |
        self.citations.map_or(false, |x| x.to_string().contains(pattern))       
    }


    pub fn matches(&self, filters: &Filters) -> bool {
        self.doi.as_ref().map_or(false, |x| x.contains(&filters.doi)) &
        self.title.as_ref().map_or(false, |x| x.contains(&filters.title)) &
        self.journal.as_ref().map_or(false, |x| x.contains(&filters.journal)) &
        self.summary.as_ref().map_or(false, |x| x.contains(&filters.summary)) &
        self.first_author.as_ref().map_or(false, |x| x.contains(&filters.first_author)) &
        self.year_published.map_or(false, |x| x.to_string().contains(&filters.year_published)) &
        self.score.map_or(false, |x| x.to_string().contains(&filters.score)) &
        self.citations.map_or(false, |x| x.to_string().contains(&filters.citations))
    }
}

