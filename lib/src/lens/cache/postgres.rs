//! PostgreSQL backend implementation for the Lens cache

use crate::lens::article::{ArticleData, ArticleWithData};
use crate::lens::error::LensError;
use crate::lens::lensid::LensId;
use async_trait::async_trait;
use sqlx::PgPool;
use std::collections::HashMap;

use super::CacheBackend;

#[derive(sqlx::FromRow)]
struct ReferencesRow {
    pub lens_id: String,
    pub references_json: String,
}

impl ReferencesRow {
    fn extract(self) -> Result<(LensId, Vec<LensId>), LensError> {
        let lens_id = LensId::try_from(self.lens_id.as_str())?;
        let relationships: Vec<LensId> = serde_json::from_str(&self.references_json)
            .ok()
            .unwrap_or_default();

        Ok((lens_id, relationships))
    }
}

#[derive(sqlx::FromRow)]
struct CitationsRow {
    pub lens_id: String,
    pub citations_json: String,
}

impl CitationsRow {
    fn extract(self) -> Result<(LensId, Vec<LensId>), LensError> {
        let lens_id = LensId::try_from(self.lens_id.as_str())?;
        let citations: Vec<LensId> = serde_json::from_str(&self.citations_json)
            .ok()
            .unwrap_or_default();

        Ok((lens_id, citations))
    }
}

#[derive(sqlx::FromRow)]
struct ArticleRow {
    pub lens_id: String,
    pub article_json: String,
}

impl ArticleRow {
    fn extract(self) -> Result<ArticleWithData, LensError> {
        let lens_id = LensId::try_from(self.lens_id.as_str())?;
        let article_data: ArticleData = serde_json::from_str(&self.article_json)?;

        Ok(ArticleWithData {
            lens_id,
            article_data,
        })
    }
}

/// PostgreSQL-based cache backend
///
/// Uses two tables:
/// - `article_references`: stores immutable outgoing edges
/// - `article_citations`: stores mutable incoming edges with timestamps
///
/// Optimized for bulk operations with:
/// - Chunked multi-row inserts (more generous limits than SQLite)
/// - Single-transaction commits
/// - Native array operations with ANY() for efficient queries
/// - JSONB columns for better performance than TEXT
pub struct PostgresBackend {
    pool: PgPool,
}

#[async_trait]
impl CacheBackend for PostgresBackend {
    async fn get_references(
        &self,
        ids: &[LensId],
    ) -> Result<HashMap<LensId, Vec<LensId>>, LensError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }

        // PostgreSQL allows us to use ANY with an array - more efficient than JSON
        let ids_vec: Vec<String> = ids.iter().map(|id| id.as_ref().to_string()).collect();

        let rows: Vec<ReferencesRow> = sqlx::query_as(
            r#"
                SELECT lens_id, references_json
                FROM article_references
                WHERE lens_id = ANY($1)
            "#,
        )
        .bind(&ids_vec)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|x| x.extract()).collect()
    }

    async fn store_references(&self, batch: &[(LensId, Vec<LensId>)]) -> Result<(), LensError> {
        if batch.is_empty() {
            return Ok(());
        }

        // PostgreSQL has higher parameter limits, so we can use larger chunks
        // Postgres default max_prepared_transactions is 32767 parameters
        // With 3 params per row: 32767 / 3 = ~10922, use 5000 for safety
        const CHUNK_SIZE: usize = 5000;

        // Start transaction for all chunks
        let mut tx = self.pool.begin().await?;

        let rough_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        for chunk in batch.chunks(CHUNK_SIZE) {
            // Pre-serialize all JSON (handles errors before building query)
            let rows: Vec<(String, String, i64)> = chunk
                .iter()
                .map(|(id, refs)| {
                    let id_str = id.as_ref().to_string();
                    let refs_json = serde_json::to_string(refs)?;

                    Ok((id_str, refs_json, rough_timestamp))
                })
                .collect::<Result<Vec<_>, LensError>>()?;

            // Build multi-row INSERT
            let mut builder = sqlx::QueryBuilder::new(
                "INSERT INTO article_references (lens_id, references_json, fetched_at) ",
            );

            builder.push_values(rows, |mut b, (id_str, refs_json, timestamp)| {
                b.push_bind(id_str)
                    .push_bind(refs_json)
                    .push_bind(timestamp);
            });

            // References are immutable, so just ignore conflicts
            builder.push(" ON CONFLICT (lens_id) DO NOTHING");

            builder.build().execute(&mut *tx).await?;
        }

        // Commit once at the end
        tx.commit().await?;

        Ok(())
    }

    async fn get_citations(
        &self,
        ids: &[LensId],
    ) -> Result<HashMap<LensId, Vec<LensId>>, LensError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }

        // Calculate timestamp for 2 weeks ago (in Unix epoch seconds)
        let two_weeks_ago = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - (14 * 24 * 60 * 60); // 14 days in seconds

        // Use PostgreSQL's native array operations + timestamp filter
        let ids_vec: Vec<String> = ids.iter().map(|id| id.as_ref().to_string()).collect();

        let rows: Vec<CitationsRow> = sqlx::query_as(
            r#"
                SELECT lens_id, citations_json
                FROM article_citations
                WHERE lens_id = ANY($1)
                AND fetched_at >= $2
            "#,
        )
        .bind(&ids_vec)
        .bind(two_weeks_ago)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|x| x.extract()).collect()
    }

    async fn store_citations(&self, batch: &[(LensId, Vec<LensId>)]) -> Result<(), LensError> {
        if batch.is_empty() {
            return Ok(());
        }

        const CHUNK_SIZE: usize = 5000;

        // Start transaction for all chunks
        let mut tx = self.pool.begin().await?;

        let rough_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        for chunk in batch.chunks(CHUNK_SIZE) {
            // Pre-serialize all JSON (handles errors before building query)
            let rows: Vec<(String, String, i64)> = chunk
                .iter()
                .map(|(id, citations)| {
                    let id_str = id.as_ref().to_string();
                    let citations_json = serde_json::to_string(citations)?;

                    Ok((id_str, citations_json, rough_timestamp))
                })
                .collect::<Result<Vec<_>, LensError>>()?;

            // Build multi-row INSERT
            let mut builder = sqlx::QueryBuilder::new(
                "INSERT INTO article_citations (lens_id, citations_json, fetched_at) ",
            );

            builder.push_values(rows, |mut b, (id_str, citations_json, timestamp)| {
                b.push_bind(id_str)
                    .push_bind(citations_json)
                    .push_bind(timestamp);
            });

            // For citations, we want to update with fresh data on conflict
            builder.push(
                " ON CONFLICT (lens_id) DO UPDATE SET citations_json = EXCLUDED.citations_json, fetched_at = EXCLUDED.fetched_at",
            );

            builder.build().execute(&mut *tx).await?;
        }

        // Commit once at the end
        tx.commit().await?;

        Ok(())
    }

    async fn get_article_data(&self, ids: &[LensId]) -> Result<Vec<ArticleWithData>, LensError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        // PostgreSQL allows us to use ANY with an array - more efficient than JSON
        let ids_vec: Vec<String> = ids.iter().map(|id| id.as_ref().to_string()).collect();

        let rows: Vec<ArticleRow> = sqlx::query_as(
            r#"
                SELECT lens_id, article_json
                FROM article_data
                WHERE lens_id = ANY($1)
            "#,
        )
        .bind(&ids_vec)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|x| x.extract()).collect()
    }

    async fn store_article_data(&self, batch: &[ArticleWithData]) -> Result<(), LensError> {
        if batch.is_empty() {
            return Ok(());
        }

        // PostgreSQL has higher parameter limits, so we can use larger chunks
        // Postgres default max_prepared_transactions is 32767 parameters
        // With 3 params per row: 32767 / 3 = ~10922, use 5000 for safety
        const CHUNK_SIZE: usize = 5000;

        // Start transaction for all chunks
        let mut tx = self.pool.begin().await?;

        let rough_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        for chunk in batch.chunks(CHUNK_SIZE) {
            // Pre-serialize all JSON (handles errors before building query)
            let rows: Vec<(String, String, i64)> = chunk
                .iter()
                .map(|article_with_data| {
                    let id_str = article_with_data.lens_id.as_ref().to_string();
                    let article_json = serde_json::to_string(&article_with_data.article_data)?;

                    Ok((id_str, article_json, rough_timestamp))
                })
                .collect::<Result<Vec<_>, LensError>>()?;

            // Build multi-row INSERT
            let mut builder = sqlx::QueryBuilder::new(
                "INSERT INTO article_data (lens_id, article_json, fetched_at) ",
            );

            builder.push_values(rows, |mut b, (id_str, refs_json, timestamp)| {
                b.push_bind(id_str)
                    .push_bind(refs_json)
                    .push_bind(timestamp);
            });

            // References are immutable, so just ignore conflicts
            builder.push(" ON CONFLICT (lens_id) DO NOTHING");

            builder.build().execute(&mut *tx).await?;
        }

        // Commit once at the end
        tx.commit().await?;

        Ok(())
    }

    async fn get_id_mapping(
        &self,
        string_ids: &[String],
    ) -> Result<HashMap<String, LensId>, LensError> {
        if string_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let ids_vec: Vec<String> = string_ids.to_vec();

        let rows: Vec<(String, String)> = sqlx::query_as(
            r#"
                SELECT string_id, lens_id
                FROM id_mappings
                WHERE string_id = ANY($1)
            "#,
        )
        .bind(&ids_vec)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|(string_id, lens_id_str)| {
                let lens_id = LensId::try_from(lens_id_str.as_str())?;
                Ok((string_id, lens_id))
            })
            .collect()
    }

    async fn store_id_mapping(&self, batch: &[(String, LensId)]) -> Result<(), LensError> {
        if batch.is_empty() {
            return Ok(());
        }

        const CHUNK_SIZE: usize = 5000;

        let mut tx = self.pool.begin().await?;

        let rough_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        for chunk in batch.chunks(CHUNK_SIZE) {
            let rows: Vec<(String, String, i64)> = chunk
                .iter()
                .map(|(string_id, lens_id)| {
                    let lens_id_str = lens_id.as_ref().to_string();
                    (string_id.clone(), lens_id_str, rough_timestamp)
                })
                .collect();

            let mut builder = sqlx::QueryBuilder::new(
                "INSERT INTO id_mappings (string_id, lens_id, fetched_at) ",
            );

            builder.push_values(rows, |mut b, (string_id, lens_id_str, timestamp)| {
                b.push_bind(string_id)
                    .push_bind(lens_id_str)
                    .push_bind(timestamp);
            });

            builder.push(" ON CONFLICT (string_id) DO NOTHING");

            builder.build().execute(&mut *tx).await?;
        }

        tx.commit().await?;

        Ok(())
    }

    async fn mark_as_fetching(&self, id: &LensId) -> Result<bool, LensError> {
        let id_str = id.as_ref().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Stale threshold: 60 seconds
        let stale_threshold = now - 60;

        // Try to insert, or update if the existing record is stale (>60s old)
        let result = sqlx::query(
            r#"
            INSERT INTO pending_fetches (lens_id, started_at)
            VALUES ($1, $2)
            ON CONFLICT (lens_id) DO UPDATE
            SET started_at = EXCLUDED.started_at
            WHERE pending_fetches.started_at < $3
            "#,
        )
        .bind(&id_str)
        .bind(now)
        .bind(stale_threshold)
        .execute(&self.pool)
        .await?;

        // If rows_affected > 0, we successfully marked it (either inserted or updated stale)
        Ok(result.rows_affected() > 0)
    }

    async fn unmark_as_fetching(&self, id: &LensId) -> Result<(), LensError> {
        let id_str = id.as_ref().to_string();

        sqlx::query("DELETE FROM pending_fetches WHERE lens_id = $1")
            .bind(&id_str)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn is_being_fetched(&self, id: &LensId) -> Result<bool, LensError> {
        let id_str = id.as_ref().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let stale_threshold = now - 60;

        let row: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT started_at FROM pending_fetches
            WHERE lens_id = $1 AND started_at >= $2
            "#,
        )
        .bind(&id_str)
        .bind(stale_threshold)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.is_some())
    }

    async fn mark_as_fetching_batch(
        &self,
        ids: &[LensId],
    ) -> Result<Vec<(LensId, bool)>, LensError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let stale_threshold = now - 60;

        let ids_vec: Vec<String> = ids.iter().map(|id| id.as_ref().to_string()).collect();

        // Use PostgreSQL's UNNEST to insert multiple rows at once
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            INSERT INTO pending_fetches (lens_id, started_at)
            SELECT unnest($1::text[]), $2
            ON CONFLICT (lens_id) DO UPDATE
            SET started_at = EXCLUDED.started_at
            WHERE pending_fetches.started_at < $3
            RETURNING lens_id
            "#,
        )
        .bind(&ids_vec)
        .bind(now)
        .bind(stale_threshold)
        .fetch_all(&self.pool)
        .await?;

        // Create a set of successfully marked IDs
        let marked_ids: std::collections::HashSet<String> =
            rows.into_iter().map(|(id,)| id).collect();

        // Build result vec matching input order
        let results = ids
            .iter()
            .map(|id| {
                let id_str = id.as_ref().to_string();
                let success = marked_ids.contains(&id_str);
                (id.clone(), success)
            })
            .collect();

        Ok(results)
    }

    async fn unmark_as_fetching_batch(&self, ids: &[LensId]) -> Result<(), LensError> {
        if ids.is_empty() {
            return Ok(());
        }

        let ids_vec: Vec<String> = ids.iter().map(|id| id.as_ref().to_string()).collect();

        sqlx::query(
            r#"
            DELETE FROM pending_fetches
            WHERE lens_id = ANY($1)
            "#,
        )
        .bind(&ids_vec)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn clear_pending_fetches(&self) -> Result<(), LensError> {
        sqlx::query("DELETE FROM pending_fetches")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn clear(&self) -> Result<(), LensError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM article_references")
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM article_citations")
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM article_data")
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM id_mappings")
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM pending_fetches")
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }
}

impl PostgresBackend {
    /// Create a new PostgreSQL backend from a pre-configured connection pool
    ///
    /// This method allows you to configure the connection pool size and other
    /// parameters before creating the backend. **Recommended for high-concurrency
    /// scenarios** where the default pool size (10 connections) may be insufficient.
    ///
    /// # Arguments
    /// * `pool` - Pre-configured sqlx PgPool
    ///
    /// # Example
    /// ```ignore
    /// use sqlx::postgres::PgPoolOptions;
    ///
    /// // Configure pool for high concurrency (e.g., 300+ concurrent users)
    /// let pool = PgPoolOptions::new()
    ///     .max_connections(50)  // Increase for high concurrency
    ///     .min_connections(10)  // Keep connections warm
    ///     .acquire_timeout(std::time::Duration::from_secs(5))
    ///     .connect("postgres://localhost/lens_cache")
    ///     .await?;
    ///
    /// let backend = PostgresBackend::from_pool(pool).await?;
    /// ```
    pub async fn from_pool(pool: PgPool) -> Result<Self, LensError> {
        let backend = Self { pool };
        backend.run_migrations().await?;
        backend.optimize_postgres().await?;

        Ok(backend)
    }

    /// Create a new PostgreSQL backend from a connection URL
    ///
    /// Uses default connection pool settings (max 10 connections). For high-concurrency
    /// scenarios (e.g., 16+ workers, 300+ concurrent users), consider using `from_pool()`
    /// with custom `PgPoolOptions` instead.
    ///
    /// # Arguments
    /// * `url` - PostgreSQL connection URL (e.g., "postgres://user:pass@localhost/dbname")
    ///
    /// # Example
    /// ```ignore
    /// let backend = PostgresBackend::from_url("postgres://localhost/lens_cache").await?;
    /// ```
    pub async fn from_url(url: &str) -> Result<Self, LensError> {
        let pool = PgPool::connect(url).await?;
        Self::from_pool(pool).await
    }

    /// Run database migrations (creates tables if they don't exist)
    async fn run_migrations(&self) -> Result<(), LensError> {
        // LensId tables (optimized with NoHasher)
        sqlx::query(
            r#"
            CREATE UNLOGGED TABLE IF NOT EXISTS article_references (
                lens_id TEXT PRIMARY KEY,
                references_json TEXT NOT NULL,
                fetched_at BIGINT NOT NULL DEFAULT EXTRACT(EPOCH FROM NOW())::BIGINT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE UNLOGGED TABLE IF NOT EXISTS article_citations (
                lens_id TEXT PRIMARY KEY,
                citations_json TEXT NOT NULL,
                fetched_at BIGINT NOT NULL DEFAULT EXTRACT(EPOCH FROM NOW())::BIGINT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create index on fetched_at for citations (useful for TTL queries)
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_citations_fetched ON article_citations(fetched_at)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE UNLOGGED TABLE IF NOT EXISTS article_data (
                lens_id TEXT PRIMARY KEY,
                article_json TEXT NOT NULL,
                fetched_at BIGINT NOT NULL DEFAULT EXTRACT(EPOCH FROM NOW())::BIGINT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // ID mappings table (PMID/DOI/etc → LensId)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS id_mappings (
                string_id TEXT PRIMARY KEY,
                lens_id TEXT NOT NULL,
                fetched_at BIGINT NOT NULL DEFAULT extract(epoch from now())
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Pending fetches table (for request deduplication)
        sqlx::query(
            r#"
            CREATE UNLOGGED TABLE IF NOT EXISTS pending_fetches (
                lens_id TEXT PRIMARY KEY,
                started_at BIGINT NOT NULL DEFAULT extract(epoch from now())
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Apply PostgreSQL-specific optimizations
    async fn optimize_postgres(&self) -> Result<(), LensError> {
        // Analyze tables to update statistics for the query planner
        // This is safe to run even if tables are empty
        let _ = sqlx::query("ANALYZE article_references")
            .execute(&self.pool)
            .await;

        let _ = sqlx::query("ANALYZE article_citations")
            .execute(&self.pool)
            .await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lens::cache::compute_misses;

    /// Helper to create an isolated PostgreSQL backend for testing
    ///
    /// Each test gets its own schema with a unique timestamp-based name.
    /// This ensures complete isolation between tests, even across test runs.
    /// Old test schemas remain in the database but don't affect new tests.
    /// Similar to SQLite's `:memory:` - each test is completely isolated.
    ///
    /// Note: Test schemas accumulate in the database. Clean them up manually if needed:
    /// ```sql
    /// SELECT 'DROP SCHEMA IF EXISTS ' || schema_name || ' CASCADE;'
    /// FROM information_schema.schemata
    /// WHERE schema_name LIKE 'test_%';
    /// ```
    async fn create_test_backend() -> Result<PostgresBackend, LensError> {
        let url = std::env::var("TEST_POSTGRES_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres@localhost/lens_test".to_string());

        // Create unique schema name using timestamp + random component
        // This ensures uniqueness even across test runs
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros();
        let random = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let schema_name = format!("test_{timestamp}_{random}");

        // First connect to create the schema
        let pool = PgPool::connect(&url).await.map_err(LensError::SqlxError)?;

        // Create isolated schema for this test
        sqlx::query(&format!("CREATE SCHEMA {schema_name}"))
            .execute(&pool)
            .await?;

        pool.close().await;

        // Reconnect with schema in the connection string
        // This ensures ALL connections from the pool use this schema
        let url_with_schema = format!("{url}?options=-c%20search_path%3D{schema_name}");
        let pool = PgPool::connect(&url_with_schema)
            .await
            .map_err(LensError::SqlxError)?;

        // Run migrations in the isolated schema
        let backend = PostgresBackend { pool };
        backend.run_migrations().await?;

        Ok(backend)
    }

    // Note: These tests require a running PostgreSQL instance
    // Set TEST_POSTGRES_DATABASE_URL environment variable to run them
    // Example: TEST_POSTGRES_DATABASE_URL=postgres://postgres:password@localhost/lens_test
    //
    // Each test runs in its own schema, so they can run in parallel without interference!

    #[tokio::test]
    async fn test_store_and_get_references() -> Result<(), LensError> {
        let backend = create_test_backend().await?;

        // Create test data
        let id1 = LensId::from(12345678901234);
        let id2 = LensId::from(98765432109876);
        let id3 = LensId::from(11111111111111);

        let refs1 = vec![LensId::from(1), LensId::from(2), LensId::from(3)];
        let refs2 = vec![LensId::from(4), LensId::from(5)];

        let batch = vec![(id1.clone(), refs1.clone()), (id2.clone(), refs2.clone())];

        // Store references
        backend.store_references(&batch).await?;

        // Retrieve references
        let result = backend.get_references(&[id1.clone(), id2.clone()]).await?;

        assert_eq!(result.len(), 2);
        assert_eq!(result.get(&id1).unwrap(), &refs1);
        assert_eq!(result.get(&id2).unwrap(), &refs2);

        // Query non-existent ID
        let result = backend.get_references(std::slice::from_ref(&id3)).await?;
        assert_eq!(result.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_store_references_immutable() -> Result<(), LensError> {
        let backend = create_test_backend().await?;

        let id1 = LensId::from(12345678901234);
        let refs1 = vec![LensId::from(1), LensId::from(2)];
        let refs2 = vec![LensId::from(3), LensId::from(4)];

        // Store initial references
        backend
            .store_references(&[(id1.clone(), refs1.clone())])
            .await?;

        // Try to store different references for the same ID (should be ignored)
        backend
            .store_references(&[(id1.clone(), refs2.clone())])
            .await?;

        // Should still return the original references
        let result = backend.get_references(std::slice::from_ref(&id1)).await?;
        assert_eq!(result.get(&id1).unwrap(), &refs1);

        Ok(())
    }

    #[tokio::test]
    async fn test_store_and_get_citations() -> Result<(), LensError> {
        let backend = create_test_backend().await?;

        let id1 = LensId::from(12345678901234);
        let id2 = LensId::from(98765432109876);
        let id3 = LensId::from(11111111111111);

        let cites1 = vec![LensId::from(10), LensId::from(20), LensId::from(30)];
        let cites2 = vec![LensId::from(40), LensId::from(50)];

        let batch = vec![(id1.clone(), cites1.clone()), (id2.clone(), cites2.clone())];

        // Store citations
        backend.store_citations(&batch).await?;

        // Retrieve citations
        let result = backend.get_citations(&[id1.clone(), id2.clone()]).await?;

        assert_eq!(result.len(), 2);
        assert_eq!(result.get(&id1).unwrap(), &cites1);
        assert_eq!(result.get(&id2).unwrap(), &cites2);

        // Query non-existent ID
        let result = backend.get_citations(std::slice::from_ref(&id3)).await?;
        assert_eq!(result.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_store_citations_updates_on_conflict() -> Result<(), LensError> {
        let backend = create_test_backend().await?;

        let id1 = LensId::from(12345678901234);
        let cites1 = vec![LensId::from(10), LensId::from(20)];
        let cites2 = vec![LensId::from(30), LensId::from(40), LensId::from(50)];

        // Store initial citations
        backend
            .store_citations(&[(id1.clone(), cites1.clone())])
            .await?;

        // Store updated citations (should replace)
        backend
            .store_citations(&[(id1.clone(), cites2.clone())])
            .await?;

        // Should return the updated citations
        let result = backend.get_citations(std::slice::from_ref(&id1)).await?;
        assert_eq!(result.get(&id1).unwrap(), &cites2);

        Ok(())
    }

    #[tokio::test]
    async fn test_empty_input_handling() -> Result<(), LensError> {
        let backend = create_test_backend().await?;

        // Test empty get_references
        let result = backend.get_references(&[]).await?;
        assert_eq!(result.len(), 0);

        // Test empty store_references
        backend.store_references(&[]).await?;

        // Test empty get_citations
        let result = backend.get_citations(&[]).await?;
        assert_eq!(result.len(), 0);

        // Test empty store_citations
        backend.store_citations(&[]).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_clear() -> Result<(), LensError> {
        let backend = create_test_backend().await?;

        let id1 = LensId::from(12345678901234);
        let id2 = LensId::from(98765432109876);

        let refs = vec![LensId::from(1), LensId::from(2)];
        let cites = vec![LensId::from(10), LensId::from(20)];

        // Store some data
        backend
            .store_references(&[(id1.clone(), refs.clone())])
            .await?;
        backend
            .store_citations(&[(id2.clone(), cites.clone())])
            .await?;

        // Verify data exists
        let refs_result = backend.get_references(std::slice::from_ref(&id1)).await?;
        let cites_result = backend.get_citations(std::slice::from_ref(&id2)).await?;
        assert_eq!(refs_result.len(), 1);
        assert_eq!(cites_result.len(), 1);

        // Clear all data
        backend.clear().await?;

        // Verify data is gone
        let refs_result = backend.get_references(std::slice::from_ref(&id1)).await?;
        let cites_result = backend.get_citations(std::slice::from_ref(&id2)).await?;
        assert_eq!(refs_result.len(), 0);
        assert_eq!(cites_result.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_bulk_insert_chunking() -> Result<(), LensError> {
        let backend = create_test_backend().await?;

        // Create a batch larger than SQLite's CHUNK_SIZE (333) but smaller than Postgres's (5000)
        let mut batch = Vec::new();
        for i in 0..1000 {
            let id = LensId::from(10000000000000 + i);
            let refs = vec![LensId::from(i), LensId::from(i + 1)];
            batch.push((id, refs));
        }

        // Store large batch
        backend.store_references(&batch).await?;

        // Verify all were stored
        let ids: Vec<LensId> = batch.iter().map(|(id, _)| id.clone()).collect();
        let result = backend.get_references(&ids).await?;
        assert_eq!(result.len(), 1000);

        Ok(())
    }

    #[tokio::test]
    async fn test_compute_misses() -> Result<(), LensError> {
        let backend = create_test_backend().await?;

        let id1 = LensId::from(12345678901234);
        let id2 = LensId::from(98765432109876);
        let id3 = LensId::from(11111111111111);
        let id4 = LensId::from(22222222222222);

        // Store only id1 and id2
        let refs = vec![LensId::from(1)];
        backend
            .store_references(&[(id1.clone(), refs.clone())])
            .await?;
        backend
            .store_references(&[(id2.clone(), refs.clone())])
            .await?;

        // Request id1, id2, id3, id4
        let requested = vec![id1.clone(), id2.clone(), id3.clone(), id4.clone()];
        let hits = backend.get_references(&requested).await?;

        // Compute misses
        let misses = compute_misses(&requested, &hits);

        assert_eq!(hits.len(), 2);
        assert_eq!(misses.len(), 2);
        assert!(misses.contains(&id3));
        assert!(misses.contains(&id4));
        assert!(!misses.contains(&id1));
        assert!(!misses.contains(&id2));

        Ok(())
    }

    #[tokio::test]
    async fn test_store_and_get_article_data() -> Result<(), LensError> {
        use crate::lens::article::{Author, ExternalIds, Source};

        let backend = create_test_backend().await?;

        // Create test articles
        let id1 = LensId::from(12345678901234);
        let id2 = LensId::from(98765432109876);
        let id3 = LensId::from(11111111111111);

        let article_data1 = ArticleData {
            title: Some("Test Article 1".to_string()),
            summary: Some("This is a test abstract".to_string()),
            scholarly_citations_count: Some(42),
            external_ids: Some(ExternalIds {
                pmid: vec!["12345".to_string()],
                doi: vec!["10.1234/test".to_string()],
            }),
            authors: Some(vec![Author {
                first_name: Some("John".to_string()),
                initials: Some("J".to_string()),
                last_name: Some("Doe".to_string()),
            }]),
            source: Some(Source {
                publisher: Some("Test Publisher".to_string()),
                title: Some("Test Journal".to_string()),
                kind: Some("journal".to_string()),
            }),
            year_published: Some(2023),
        };

        let article_data2 = ArticleData {
            title: Some("Test Article 2".to_string()),
            summary: None,
            scholarly_citations_count: Some(10),
            external_ids: None,
            authors: None,
            source: None,
            year_published: Some(2024),
        };

        let article1 = ArticleWithData {
            lens_id: id1.clone(),
            article_data: article_data1,
        };

        let article2 = ArticleWithData {
            lens_id: id2.clone(),
            article_data: article_data2,
        };

        let batch = vec![article1, article2];

        // Store articles
        backend.store_article_data(&batch).await?;

        // Retrieve articles
        let result = backend
            .get_article_data(&[id1.clone(), id2.clone()])
            .await?;

        assert_eq!(result.len(), 2);

        let retrieved1 = result.iter().find(|a| a.lens_id == id1).unwrap();
        assert_eq!(
            retrieved1.article_data.title,
            Some("Test Article 1".to_string())
        );
        assert_eq!(retrieved1.article_data.scholarly_citations_count, Some(42));
        assert_eq!(retrieved1.article_data.year_published, Some(2023));
        assert!(retrieved1.article_data.external_ids.is_some());

        let retrieved2 = result.iter().find(|a| a.lens_id == id2).unwrap();
        assert_eq!(
            retrieved2.article_data.title,
            Some("Test Article 2".to_string())
        );
        assert_eq!(retrieved2.article_data.year_published, Some(2024));

        // Query non-existent ID
        let result = backend.get_article_data(std::slice::from_ref(&id3)).await?;
        assert_eq!(result.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_store_and_get_id_mapping() -> Result<(), LensError> {
        let backend = create_test_backend().await?;

        let lens_id1 = LensId::from(12345678901234);
        let lens_id2 = LensId::from(98765432109876);
        let lens_id3 = LensId::from(11111111111111);

        // Store mappings from various string IDs to LensIds (raw IDs without type prefixes)
        let mappings = vec![
            ("12345".to_string(), lens_id1.clone()),        // PMID
            ("10.1234/test".to_string(), lens_id2.clone()), // DOI
            ("10.5678/foo".to_string(), lens_id2.clone()),  // Same LensId, different DOI
        ];

        backend.store_id_mapping(&mappings).await?;

        // Retrieve mappings
        let string_ids = vec![
            "12345".to_string(),
            "10.1234/test".to_string(),
            "10.5678/foo".to_string(),
            "99999".to_string(), // Not stored
        ];

        let result = backend.get_id_mapping(&string_ids).await?;

        assert_eq!(result.len(), 3);
        assert_eq!(result.get("12345"), Some(&lens_id1));
        assert_eq!(result.get("10.1234/test"), Some(&lens_id2));
        assert_eq!(result.get("10.5678/foo"), Some(&lens_id2));
        assert_eq!(result.get("99999"), None);

        // Test immutability - try to update an existing mapping
        let update_attempt = vec![("12345".to_string(), lens_id3.clone())];
        backend.store_id_mapping(&update_attempt).await?;

        // Verify it wasn't updated (ON CONFLICT DO NOTHING)
        let result_after = backend.get_id_mapping(&["12345".to_string()]).await?;
        assert_eq!(result_after.get("12345"), Some(&lens_id1)); // Still the original

        // Test empty input
        let empty_result = backend.get_id_mapping(&[]).await?;
        assert_eq!(empty_result.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_mark_and_unmark_fetching() -> Result<(), LensError> {
        let backend = create_test_backend().await?;

        let id1 = LensId::from(12345678901234);

        // Initially, ID should not be marked as fetching
        assert!(!backend.is_being_fetched(&id1).await?);

        // Mark as fetching
        let marked = backend.mark_as_fetching(&id1).await?;
        assert!(marked, "First mark should succeed");

        // Should now be marked as fetching
        assert!(backend.is_being_fetched(&id1).await?);

        // Try to mark again (should fail - already being fetched)
        let marked_again = backend.mark_as_fetching(&id1).await?;
        assert!(!marked_again, "Second mark should fail (already fetching)");

        // Unmark
        backend.unmark_as_fetching(&id1).await?;

        // Should no longer be marked
        assert!(!backend.is_being_fetched(&id1).await?);

        // Can mark again after unmarking
        let marked_third = backend.mark_as_fetching(&id1).await?;
        assert!(marked_third, "Mark after unmark should succeed");

        Ok(())
    }

    #[tokio::test]
    async fn test_stale_fetch_marks_are_overwritten() -> Result<(), LensError> {
        let backend = create_test_backend().await?;

        let id1 = LensId::from(12345678901234);

        // Insert a stale mark (61 seconds ago)
        let stale_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - 61;

        sqlx::query("INSERT INTO pending_fetches (lens_id, started_at) VALUES ($1, $2)")
            .bind(id1.as_ref().to_string())
            .bind(stale_timestamp)
            .execute(&backend.pool)
            .await?;

        // Stale mark should not be detected as "being fetched"
        assert!(
            !backend.is_being_fetched(&id1).await?,
            "Stale marks (>60s) should not be detected"
        );

        // Should be able to mark again (overwriting stale mark)
        let marked = backend.mark_as_fetching(&id1).await?;
        assert!(marked, "Should be able to overwrite stale mark");

        // Now it should be detected as being fetched (with fresh timestamp)
        assert!(backend.is_being_fetched(&id1).await?);

        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_fetch_coordination() -> Result<(), LensError> {
        let backend = std::sync::Arc::new(create_test_backend().await?);

        let id1 = LensId::from(12345678901234);
        let refs = vec![LensId::from(1), LensId::from(2), LensId::from(3)];

        // Simulate two concurrent requests for the same ID
        let backend1 = backend.clone();
        let backend2 = backend.clone();
        let id1_clone = id1.clone();
        let id1_clone2 = id1.clone();
        let refs_clone = refs.clone();
        let refs_clone2 = refs.clone();

        let handle1 = tokio::spawn(async move {
            // First caller tries to mark
            let marked = backend1.mark_as_fetching(&id1_clone).await.unwrap();
            if marked {
                // Simulate API call delay
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;

                // Store the data
                backend1
                    .store_references(&[(id1_clone.clone(), refs_clone.clone())])
                    .await
                    .unwrap();

                // Unmark
                backend1.unmark_as_fetching(&id1_clone).await.unwrap();

                "fetched"
            } else {
                "waited"
            }
        });

        let handle2 = tokio::spawn(async move {
            // Small delay to ensure handle1 marks first
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;

            // Second caller tries to mark (should fail)
            let marked = backend2.mark_as_fetching(&id1_clone2).await.unwrap();
            if marked {
                // Should not reach here in this test
                backend2
                    .store_references(&[(id1_clone2.clone(), refs_clone2.clone())])
                    .await
                    .unwrap();
                backend2.unmark_as_fetching(&id1_clone2).await.unwrap();
                "fetched"
            } else {
                // Wait for data to appear
                for _ in 0..20 {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    let cached = backend2
                        .get_references(&[id1_clone2.clone()])
                        .await
                        .unwrap();
                    if cached.contains_key(&id1_clone2) {
                        return "waited_found";
                    }
                }
                "waited_timeout"
            }
        });

        let result1 = handle1.await.unwrap();
        let result2 = handle2.await.unwrap();

        // One should fetch, one should wait and find
        assert!(
            (result1 == "fetched" && result2 == "waited_found")
                || (result1 == "waited_found" && result2 == "fetched"),
            "One caller should fetch, the other should wait and find data. Got: {} and {}",
            result1,
            result2
        );

        // Verify data is in cache
        let cached = backend.get_references(&[id1.clone()]).await?;
        assert_eq!(cached.get(&id1).unwrap(), &refs);

        // Verify no pending marks remain
        assert!(!backend.is_being_fetched(&id1).await?);

        Ok(())
    }

    #[tokio::test]
    async fn test_clear_pending_fetches() -> Result<(), LensError> {
        let backend = create_test_backend().await?;

        let id1 = LensId::from(12345678901234);
        let id2 = LensId::from(98765432109876);

        // Mark multiple IDs
        backend.mark_as_fetching(&id1).await?;
        backend.mark_as_fetching(&id2).await?;

        assert!(backend.is_being_fetched(&id1).await?);
        assert!(backend.is_being_fetched(&id2).await?);

        // Clear all pending fetches
        backend.clear_pending_fetches().await?;

        // Should all be cleared
        assert!(!backend.is_being_fetched(&id1).await?);
        assert!(!backend.is_being_fetched(&id2).await?);

        Ok(())
    }
}
