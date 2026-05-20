use std::collections::BTreeSet;

use actix_web::{HttpResponse, Responder, web};

use crate::AppConfig;

pub async fn upload_corpus(req_body: String, config: web::Data<AppConfig>) -> impl Responder {
    let corpus = Corpus::from_flat_string(&req_body);
    let hash = corpus.save_to_database(&config.database_pool).await;
    match hash {
        Ok(hash) => HttpResponse::Ok().body(hex::encode(hash)),
        Err(e) => HttpResponse::InternalServerError().body(format!("Failed to save corpus: {e}")),
    }
}

pub async fn download_corpus(
    path: web::Path<String>,
    config: web::Data<AppConfig>,
) -> impl Responder {
    let hash_bytes = match hex::decode(path.as_str()) {
        Ok(bytes) if bytes.len() == 32 => {
            let mut hash_array = [0u8; 32];
            hash_array.copy_from_slice(&bytes);
            hash_array
        }
        _ => return HttpResponse::BadRequest().body("Invalid hash format"),
    };

    match Corpus::load_from_database(&config.database_pool, &hash_bytes).await {
        Ok(corpus) => HttpResponse::Ok().body(corpus.to_flat_string()),
        Err(CorpusError::DatabaseError(sqlx::Error::RowNotFound)) => {
            HttpResponse::NotFound().body("Corpus not found")
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Failed to load corpus: {e}")),
    }
}

pub async fn enrich_corpus(
    path: web::Path<String>,
    config: web::Data<AppConfig>,
) -> impl Responder {
    let hash_bytes = match hex::decode(path.as_str()) {
        Ok(bytes) if bytes.len() == 32 => {
            let mut hash_array = [0u8; 32];
            hash_array.copy_from_slice(&bytes);
            hash_array
        }
        _ => return HttpResponse::BadRequest().body("Invalid hash format"),
    };

    let corpus = match Corpus::load_from_database(&config.database_pool, &hash_bytes).await {
        Ok(c) => c,
        Err(CorpusError::DatabaseError(sqlx::Error::RowNotFound)) => {
            return HttpResponse::NotFound().body("Corpus not found");
        }
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!("Failed to load corpus: {e}"));
        }
    };

    let id_strings: Vec<String> = corpus.into();
    let doi_strs: Vec<&str> = id_strings.iter().map(|s| s.as_str()).collect();

    match biblizap_rs::enrich_by_raw_ids(
        &doi_strs,
        &config.lens_api_key,
        None,
        Some(&config.cache_backend),
    )
    .await
    {
        Ok(articles) => HttpResponse::Ok().json(articles),
        Err(e) => HttpResponse::InternalServerError().body(format!("Failed to enrich corpus: {e}")),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CorpusError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Compression error: {0}")]
    CompressionError(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Identifier(String);

impl Identifier {
    fn new(s: &str) -> Option<Self> {
        let normalized = s.trim().to_lowercase();
        crate::common::is_valid_id(&normalized).then(|| Identifier(normalized))
    }
}

#[derive(Debug, Clone)]
struct Corpus {
    ids: BTreeSet<Identifier>,
}

impl Corpus {
    pub fn to_flat_string(&self) -> String {
        self.ids
            .iter()
            .map(|id| id.0.clone())
            .collect::<Vec<String>>()
            .join("\n")
    }

    pub fn from_flat_string(flat: &str) -> Self {
        let ids = flat
            .lines()
            .filter_map(|line| Identifier::new(line))
            .collect::<BTreeSet<Identifier>>();
        Corpus { ids }
    }

    pub fn sha256(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        for id in &self.ids {
            hasher.update(id.0.as_bytes());
            hasher.update(b"\n");
        }
        hasher.finalize().into()
    }

    pub fn compress(&self) -> Result<Vec<u8>, CorpusError> {
        use zstd::stream::encode_all;
        let flat_string = self.to_flat_string();
        Ok(encode_all(flat_string.as_bytes(), 0)?)
    }
}

impl From<BTreeSet<Identifier>> for Corpus {
    fn from(ids: BTreeSet<Identifier>) -> Self {
        Self { ids }
    }
}

impl From<Corpus> for BTreeSet<Identifier> {
    fn from(corpus: Corpus) -> Self {
        corpus.ids
    }
}

impl From<Vec<String>> for Corpus {
    fn from(id_strings: Vec<String>) -> Self {
        let ids: BTreeSet<Identifier> = id_strings
            .into_iter()
            .filter_map(|s| Identifier::new(&s))
            .collect();
        Self { ids }
    }
}

impl From<Corpus> for Vec<String> {
    fn from(corpus: Corpus) -> Self {
        corpus.ids.into_iter().map(|id| id.0).collect()
    }
}

impl Corpus {
    pub async fn save_to_database(&self, pool: &sqlx::PgPool) -> Result<[u8; 32], CorpusError> {
        let hash = self.sha256();
        let compressed_blob = self.compress()?;
        let _ = sqlx::query!(
            r#"
            INSERT INTO bbz_corpora (hash, data)
            VALUES ($1, $2)
            ON CONFLICT (hash) DO NOTHING
            "#,
            &hash,
            &compressed_blob
        )
        .execute(pool)
        .await?;

        Ok(hash)
    }

    pub async fn load_from_database(
        pool: &sqlx::PgPool,
        hash: &[u8; 32],
    ) -> Result<Self, CorpusError> {
        let record = sqlx::query!(
            r#"
            SELECT data
            FROM bbz_corpora
            WHERE hash = $1
            "#,
            hash
        )
        .fetch_one(pool)
        .await?;

        let compressed_blob = record.data;
        let decompressed_data = zstd::stream::decode_all(&compressed_blob[..])?;
        let flat_string = String::from_utf8(decompressed_data).map_err(|e| {
            CorpusError::CompressionError(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })?;
        Ok(Corpus::from_flat_string(&flat_string))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make(dois: &[&str]) -> Corpus {
        Corpus::from(dois.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn order_independence() {
        let a = make(&[
            "10.1016/j.cell.2020",
            "10.1038/s41586-021",
            "10.1093/nar/gkaa",
        ]);
        let b = make(&[
            "10.1093/nar/gkaa",
            "10.1016/j.cell.2020",
            "10.1038/s41586-021",
        ]);
        assert_eq!(a.sha256(), b.sha256());
    }

    #[test]
    fn different_content_different_hash() {
        let a = make(&["10.1016/j.cell.2020"]);
        let b = make(&["10.1038/s41586-021"]);
        assert_ne!(a.sha256(), b.sha256());
    }

    #[test]
    fn deduplication() {
        let with_dups = make(&["10.1234/abc", "10.1234/abc", "10.5678/xyz"]);
        let without_dups = make(&["10.1234/abc", "10.5678/xyz"]);
        assert_eq!(with_dups.sha256(), without_dups.sha256());
    }

    #[test]
    fn normalization_casing() {
        let lower = make(&["10.1234/abc"]);
        let upper = make(&["10.1234/ABC"]);
        assert_eq!(lower.sha256(), upper.sha256());
    }

    #[test]
    fn normalization_whitespace() {
        let clean = make(&["10.1234/abc"]);
        let padded = make(&["  10.1234/abc  "]);
        assert_eq!(clean.sha256(), padded.sha256());
    }

    #[test]
    fn invalid_ids_dropped() {
        let with_invalid = make(&["10.1234/abc", "not-a-doi", "https://doi.org/10.1234/abc"]);
        let clean = make(&["10.1234/abc"]);
        assert_eq!(with_invalid.sha256(), clean.sha256());
    }

    #[test]
    fn pmids_accepted() {
        let with_pmid = make(&["10.1234/abc", "29406940"]);
        let doi_only = make(&["10.1234/abc"]);
        assert_ne!(with_pmid.sha256(), doi_only.sha256());
        // PMID alone is also valid
        let pmid_only = make(&["29406940"]);
        assert_ne!(pmid_only.sha256(), doi_only.sha256());
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let original = make(&["10.1016/j.cell.2020", "10.1038/s41586-021"]);
        let roundtripped = Corpus::from_flat_string(&original.to_flat_string());
        assert_eq!(original.sha256(), roundtripped.sha256());
    }

    #[sqlx::test]
    fn database_roundtrip(pool: sqlx::PgPool) -> Result<(), CorpusError> {
        let original = make(&["10.1016/j.cell.2020", "10.1038/s41586-021"]);
        let hash = original.save_to_database(&pool).await?;
        let loaded = Corpus::load_from_database(&pool, &hash).await?;
        assert_eq!(original.sha256(), loaded.sha256());
        Ok(())
    }
}
