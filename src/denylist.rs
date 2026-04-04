use std::collections::BTreeSet;

use actix_web::{HttpResponse, Responder, web};

use crate::AppConfig;

pub async fn upload_denylist(req_body: String, config: web::Data<AppConfig>) -> impl Responder {
    let denylist = DenyList::from_flat_string(&req_body);
    let hash = denylist.save_to_database(&config.database_pool).await;
    match hash {
        Ok(hash) => HttpResponse::Ok().body(hex::encode(hash)),
        Err(e) => HttpResponse::InternalServerError().body(format!("Failed to save DenyList: {e}")),
    }
}

pub async fn download_denylist(hash_hex: String, config: web::Data<AppConfig>) -> impl Responder {
    let hash_bytes = match hex::decode(&hash_hex) {
        Ok(bytes) if bytes.len() == 32 => {
            let mut hash_array = [0u8; 32];
            hash_array.copy_from_slice(&bytes);
            hash_array
        }
        _ => return HttpResponse::BadRequest().body("Invalid hash format"),
    };

    match DenyList::load_from_database(&config.database_pool, &hash_bytes).await {
        Ok(denylist) => HttpResponse::Ok().body(denylist.to_flat_string()),
        Err(DenyListError::DatabaseError(sqlx::Error::RowNotFound)) => {
            HttpResponse::NotFound().body("Denylist not found")
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Failed to load DenyList: {e}")),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DenyListError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Compression error: {0}")]
    CompressionError(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Doi(String);

impl Doi {
    fn new(doi: &str) -> Option<Self> {
        let normalized = doi.trim().to_lowercase();
        crate::common::is_valid_doi(&normalized).then(|| Doi(normalized))
    }
}

#[derive(Debug, Clone)]
struct DenyList {
    pub dois: BTreeSet<Doi>,
}

impl DenyList {
    pub fn to_flat_string(&self) -> String {
        self.dois
            .iter()
            .map(|doi| doi.0.clone())
            .collect::<Vec<String>>()
            .join("\n")
    }

    pub fn from_flat_string(flat: &str) -> Self {
        let dois = flat
            .lines()
            .filter_map(|line| Doi::new(line))
            .collect::<BTreeSet<Doi>>();
        DenyList { dois }
    }

    pub fn sha256(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        for doi in &self.dois {
            hasher.update(doi.0.as_bytes());
            hasher.update(b"\n");
        }
        hasher.finalize().into()
    }

    pub fn compress(&self) -> Result<Vec<u8>, DenyListError> {
        use zstd::stream::encode_all;
        let flat_string = self.to_flat_string();
        Ok(encode_all(flat_string.as_bytes(), 0)?)
    }
}

impl From<BTreeSet<Doi>> for DenyList {
    fn from(dois: BTreeSet<Doi>) -> Self {
        Self { dois }
    }
}

impl From<DenyList> for BTreeSet<Doi> {
    fn from(denylist: DenyList) -> Self {
        denylist.dois
    }
}

impl From<Vec<String>> for DenyList {
    fn from(doi_strings: Vec<String>) -> Self {
        let hashset: BTreeSet<Doi> = doi_strings
            .into_iter()
            .filter_map(|s| Doi::new(&s))
            .collect();
        Self { dois: hashset }
    }
}

impl From<DenyList> for Vec<String> {
    fn from(denylist: DenyList) -> Self {
        denylist.dois.into_iter().map(|doi| doi.0).collect()
    }
}

impl DenyList {
    pub async fn save_to_database(&self, pool: &sqlx::PgPool) -> Result<[u8; 32], DenyListError> {
        let hash = self.sha256();
        let compressed_blob = self.compress()?;
        let _ = sqlx::query!(
            r#"
            INSERT INTO bbz_denylists (hash, data)
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
    ) -> Result<Self, DenyListError> {
        let record = sqlx::query!(
            r#"
            SELECT data
            FROM bbz_denylists
            WHERE hash = $1
            "#,
            hash
        )
        .fetch_one(pool)
        .await?;

        let compressed_blob = record.data;
        let decompressed_data = zstd::stream::decode_all(&compressed_blob[..])?;
        let flat_string = String::from_utf8(decompressed_data).map_err(|e| {
            DenyListError::CompressionError(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })?;
        Ok(DenyList::from_flat_string(&flat_string))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make(dois: &[&str]) -> DenyList {
        DenyList::from(dois.iter().map(|s| s.to_string()).collect::<Vec<_>>())
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
    fn invalid_dois_dropped() {
        let with_invalid = make(&["10.1234/abc", "not-a-doi", "https://doi.org/10.1234/abc"]);
        let clean = make(&["10.1234/abc"]);
        assert_eq!(with_invalid.sha256(), clean.sha256());
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let original = make(&["10.1016/j.cell.2020", "10.1038/s41586-021"]);
        let roundtripped = DenyList::from_flat_string(&original.to_flat_string());
        assert_eq!(original.sha256(), roundtripped.sha256());
    }

    #[sqlx::test]
    fn database_roundtrip(pool: sqlx::PgPool) -> Result<(), DenyListError> {
        let original = make(&["10.1016/j.cell.2020", "10.1038/s41586-021"]);
        let hash = original.save_to_database(&pool).await?;
        let loaded = DenyList::load_from_database(&pool, &hash).await?;
        assert_eq!(original.sha256(), loaded.sha256());
        Ok(())
    }
}
