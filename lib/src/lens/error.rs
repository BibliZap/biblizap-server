use reqwest::header::ToStrError;
use thiserror::Error;

use super::lensid::LensIdError;

#[derive(Error, Debug)]
pub enum LensError {
    #[error("All provided IDs are invalid")]
    NoValidIdsInInputList,
    #[error("Lens API is unresponsive : {0}")]
    Request(#[from] reqwest::Error),
    #[error("Failed to extract rate limit information")]
    RateLimitExtraction(#[from] RateLimitExtractionError),
    #[error("{0}")]
    LensApi(LensApiErrorInfo),
    #[error("Failed to parse JSON response from Lens API : {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("No articles found for the given query, please check if your input IDs exist")]
    NoArticlesFound,
    #[error(transparent)]
    LensIDError(#[from] LensIdError),
    #[cfg(any(feature = "cache-sqlite", feature = "cache-postgres"))]
    #[error("Database error: {0}")]
    SqlxError(#[from] sqlx::Error),
}

#[derive(Error, Debug)]
pub enum RateLimitExtractionError {
    #[error(transparent)]
    RequestToStr(#[from] ToStrError),
    #[error(transparent)]
    ParseError(#[from] std::num::ParseIntError),
}

pub struct LensApiErrorInfo {
    pub status_code: u16,
    pub message: String,
}

// Shown to users
impl std::fmt::Display for LensApiErrorInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Lens API replied with status {}, please ask your administrator if his API key is valid", self.status_code)
    }
}

// Shown in server logs
impl std::fmt::Debug for LensApiErrorInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Lens Api replied with status {}: {}",
            self.status_code, self.message
        )
    }
}
