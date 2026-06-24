#[path = "../openalex/mod.rs"]
mod openalex;

use clap::{Parser, Subcommand};
use config as conf;
use serde::Deserialize;
use std::{env, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Deserialize, Default)]
struct FileConfig {
    openalex_dump_path: Option<PathBuf>,
}

#[derive(Debug, Error)]
enum Error {
    #[error("missing {0}; set it in biblizap.toml, an environment variable, or the CLI")]
    MissingConfig(&'static str),
    #[error("failed to connect to OpenAlex database: {0}")]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Import(#[from] openalex::OpenAlexImportError),
}

#[derive(Parser, Debug)]
#[command(version, about = "Build and manage the BibliZap OpenAlex database")]
struct Args {
    #[command(subcommand)]
    command: Command,

    /// Log level for the importer
    #[arg(short = 'L', long, default_value_t = log::LevelFilter::Info)]
    log_level: log::LevelFilter,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Import an OpenAlex gzipped JSON/JSONL dump into the BibliZap database
    Import {
        /// Path to an OpenAlex gzipped JSON/JSONL dump file or dump directory
        #[arg(long)]
        dump_path: Option<PathBuf>,

        /// PostgreSQL URL for the BibliZap OpenAlex database
        #[arg(long)]
        database_url: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();
    dotenvy::dotenv().ok();

    let mut logger_builder = env_logger::Builder::from_env(env_logger::Env::default());
    if env::var_os("RUST_LOG").is_none() {
        logger_builder.filter_level(args.log_level);
    }
    logger_builder.init();

    let file_cfg = load_file_config();

    match args.command {
        Command::Import {
            dump_path,
            database_url,
        } => run_import(dump_path, database_url, file_cfg).await?,
    }

    Ok(())
}

async fn run_import(
    dump_path: Option<PathBuf>,
    database_url: Option<String>,
    file_cfg: FileConfig,
) -> Result<(), Error> {
    let dump_path = resolve_dump_path(dump_path, file_cfg.openalex_dump_path)?;
    let database_url = resolve_database_url(database_url)?;

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;

    openalex::import_openalex_dump(&dump_path, &pool).await?;

    Ok(())
}

fn resolve_dump_path(
    cli_dump_path: Option<PathBuf>,
    config_dump_path: Option<PathBuf>,
) -> Result<PathBuf, Error> {
    cli_dump_path
        .or(config_dump_path)
        .or_else(|| {
            env::var("BIBLIZAP_OPENALEX_DUMP_PATH")
                .ok()
                .map(PathBuf::from)
        })
        .ok_or(Error::MissingConfig("openalex_dump_path"))
}

fn resolve_database_url(cli_database_url: Option<String>) -> Result<String, Error> {
    cli_database_url
        .or_else(|| env::var("DATABASE_URL").ok())
        .ok_or(Error::MissingConfig("DATABASE_URL"))
}

fn load_file_config() -> FileConfig {
    let user_config_dir = env::var("XDG_CONFIG_HOME").ok().unwrap_or_else(|| {
        let home = env::var("HOME").unwrap_or_default();
        format!("{}/.config", home)
    });

    let builder = conf::Config::builder()
        .add_source(conf::File::with_name("/etc/biblizap/biblizap.toml").required(false))
        .add_source(
            conf::File::with_name(&format!("{}/biblizap/biblizap.toml", user_config_dir))
                .required(false),
        )
        .add_source(conf::File::with_name("biblizap.toml").required(false))
        .add_source(conf::Environment::with_prefix("BIBLIZAP").separator("__"));

    let settings = builder.build().unwrap_or_else(|e| {
        log::warn!("failed to build config: {}", e);
        conf::Config::default()
    });

    settings.try_deserialize().unwrap_or_else(|e| {
        log::warn!("failed to parse config: {}", e);
        FileConfig::default()
    })
}
