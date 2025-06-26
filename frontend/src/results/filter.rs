/// Struct holding the filter values for each column in the results table.
#[derive(Default, PartialEq, Debug)]
pub struct Filters {
    pub first_author: String,
    pub year_published: String,
    pub title: String,
    pub journal: String,
    pub summary: String,
    pub doi: String,
    pub citations: String,
    pub score: String,
}
