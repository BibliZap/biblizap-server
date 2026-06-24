use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum OpenAlexImportError {
    #[error("OpenAlex importer is not implemented yet")]
    NotImplemented,
}

pub async fn import_openalex_dump(
    dump_path: &Path,
    _pool: &sqlx::PgPool,
) -> Result<(), OpenAlexImportError> {
    log::info!(
        "OpenAlex import stub reached for dump path: {}",
        dump_path.display()
    );
    Err(OpenAlexImportError::NotImplemented)
}
