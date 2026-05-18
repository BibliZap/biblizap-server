use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub enum SearchFor {
    References,
    Citations,
    Both,
}
