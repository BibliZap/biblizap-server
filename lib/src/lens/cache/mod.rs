//! Cache backend implementations for Lens.org article data
//!
//! This module provides a trait-based cache system with multiple backend implementations:
//! - SQLite (via `sqlite` module) - suitable for development and low-concurrency scenarios
//! - PostgreSQL (via `postgres` module) - recommended for production with high concurrency
//!
//! The cache stores two types of relationships:
//! - References (outgoing edges): immutable once fetched
//! - Citations (incoming edges): mutable, need periodic refresh
//!
//! ## Performance Considerations
//!
//! For high-concurrency scenarios (300+ users, 16+ async workers), use PostgreSQL with
//! a properly sized connection pool via `PostgresBackend::from_pool()`. See the README
//! for detailed configuration guidelines.

use crate::lens::article::ArticleWithData;

use super::error::LensError;
use super::lensid::LensId;
use async_trait::async_trait;
use std::collections::HashMap;

#[cfg(feature = "cache-sqlite")]
pub mod sqlite;

#[cfg(feature = "cache-postgres")]
pub mod postgres;

// Re-export the backend types for convenience
#[cfg(feature = "cache-sqlite")]
pub use sqlite::SqliteBackend;

#[cfg(feature = "cache-postgres")]
pub use postgres::PostgresBackend;

/// Computes which IDs were not found in the cache (misses)
///
/// # Arguments
/// * `requested` - The IDs that were requested
/// * `hits` - The IDs that were found in the cache
///
/// # Returns
/// A vector of IDs that were requested but not found in the cache
pub fn compute_misses<T>(requested: &[LensId], hits: &HashMap<LensId, T>) -> Vec<LensId> {
    requested
        .iter()
        .filter(|id| !hits.contains_key(id))
        .cloned()
        .collect()
}

/// Trait defining the cache backend interface
///
/// Implementations must be thread-safe (Send + Sync) as they may be used
/// across async tasks.
///
/// This trait is always available (no feature flag required), but implementations
/// (SqliteBackend, PostgresBackend) require their respective feature flags.
#[async_trait]
pub trait CacheBackend: Send + Sync {
    // References (immutable) - LensId optimized methods

    /// Retrieve references (outgoing edges) for the given article IDs (LensId)
    ///
    /// Returns only the IDs that were found in the cache.
    /// Use `compute_misses` to determine which IDs need to be fetched from the API.
    async fn get_references(
        &self,
        ids: &[LensId],
    ) -> Result<HashMap<LensId, Vec<LensId>>, LensError>;

    /// Store references (outgoing edges) for articles (LensId)
    ///
    /// References are immutable - if an ID already exists in the cache,
    /// the new data is ignored (ON CONFLICT DO NOTHING behavior).
    async fn store_references(&self, batch: &[(LensId, Vec<LensId>)]) -> Result<(), LensError>;

    // Citations (with TTL) - LensId optimized methods

    /// Retrieve citations (incoming edges) for the given article IDs (LensId)
    ///
    /// Returns citations as LensIds. Citations may be stale and should be
    /// refreshed periodically based on `fetched_at` timestamps.
    async fn get_citations(
        &self,
        ids: &[LensId],
    ) -> Result<HashMap<LensId, Vec<LensId>>, LensError>;

    /// Store citations (incoming edges) for articles (LensId)
    ///
    /// Citations are mutable - if an ID already exists, the data and timestamp
    /// are updated (ON CONFLICT DO UPDATE behavior).
    async fn store_citations(&self, batch: &[(LensId, Vec<LensId>)]) -> Result<(), LensError>;

    // Article data

    /// Retrieve article data for the given LensIds
    ///
    /// Returns a Vec of ArticleWithData structs for articles found in cache.
    async fn get_article_data(&self, ids: &[LensId]) -> Result<Vec<ArticleWithData>, LensError>;

    /// Store article data for LensIds
    ///
    /// Accepts a slice of ArticleWithData structs to store in cache.
    async fn store_article_data(&self, batch: &[ArticleWithData]) -> Result<(), LensError>;

    // ID Mappings (PMID/DOI/etc → LensId)

    /// Retrieve LensId mappings for string identifiers (PMID, DOI, etc.)
    ///
    /// Given a list of raw string IDs (e.g., "12345", "10.1234/foo"),
    /// returns a HashMap of those IDs mapped to their corresponding LensIds.
    ///
    /// Note: IDs are stored as raw strings without type prefixes. Users provide
    /// "12345" not "PMID:12345", and "10.1234/foo" not "DOI:10.1234/foo".
    ///
    /// This allows resolving user-provided IDs (PMIDs, DOIs) to LensIds,
    /// which can then be used to query the main LensId-keyed cache tables.
    ///
    /// # Arguments
    /// * `string_ids` - Slice of raw string identifiers to look up (without type prefixes)
    ///
    /// # Returns
    /// A HashMap mapping found string IDs to their LensIds. IDs not found in
    /// the cache are simply not included in the result.
    async fn get_id_mapping(
        &self,
        string_ids: &[String],
    ) -> Result<HashMap<String, LensId>, LensError>;

    /// Store ID mappings from string identifiers to LensIds
    ///
    /// Stores mappings from user-provided raw IDs (e.g., "12345", "10.1234/foo")
    /// to their canonical LensIds. This enables fast resolution of string IDs to
    /// LensIds without needing to query the Lens API.
    ///
    /// Note: Store IDs as raw strings without type prefixes. For example,
    /// store "12345" not "PMID:12345", and "10.1234/foo" not "DOI:10.1234/foo".
    ///
    /// Mappings are immutable - if a mapping already exists, it is not updated
    /// (ON CONFLICT DO NOTHING behavior).
    ///
    /// # Arguments
    /// * `batch` - Slice of (raw_string_id, lens_id) tuples to store
    async fn store_id_mapping(&self, batch: &[(String, LensId)]) -> Result<(), LensError>;

    // Pending fetch coordination (prevents thundering herd)

    /// Mark an ID as being fetched (in-flight API request)
    ///
    /// Returns true if the mark was successful (caller should proceed with fetch).
    /// Returns false if another caller is already fetching this ID.
    ///
    /// Orphaned marks (>60s old) are automatically ignored/overwritten.
    async fn mark_as_fetching(&self, id: &LensId) -> Result<bool, LensError>;

    /// Mark multiple IDs as being fetched (batch operation)
    ///
    /// Returns a Vec of (LensId, success) tuples indicating which marks succeeded.
    /// - true: caller should fetch this ID
    /// - false: another caller is already fetching this ID
    ///
    /// Orphaned marks (>60s old) are automatically ignored/overwritten.
    async fn mark_as_fetching_batch(
        &self,
        ids: &[LensId],
    ) -> Result<Vec<(LensId, bool)>, LensError>;

    /// Unmark an ID after fetch completes (success or failure)
    ///
    /// Should be called in a finally block to ensure cleanup even on errors.
    async fn unmark_as_fetching(&self, id: &LensId) -> Result<(), LensError>;

    /// Unmark multiple IDs after fetches complete (batch operation)
    ///
    /// Should be called in a finally block to ensure cleanup even on errors.
    async fn unmark_as_fetching_batch(&self, ids: &[LensId]) -> Result<(), LensError>;

    /// Check if an ID is currently being fetched by another caller
    ///
    /// Returns false if the pending mark is stale (>60s old).
    async fn is_being_fetched(&self, id: &LensId) -> Result<bool, LensError>;

    /// Clear all pending fetch marks (for cleanup on startup/crashes)
    async fn clear_pending_fetches(&self) -> Result<(), LensError>;

    /// Clear all cached data (both references and citations)
    async fn clear(&self) -> Result<(), LensError>;
}
