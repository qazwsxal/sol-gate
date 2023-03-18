use chrono::NaiveDate;
use std::{fmt::Debug, path::PathBuf};

use serde::{Deserialize, Serialize};
use sqlx::{
    self,
    migrate::{MigrateError, Migrator},
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    SqlitePool,
};

pub mod queries;

static MIG: Migrator = sqlx::migrate!("./migrations/");

pub(crate) const BIND_LIMIT: usize = 32766; //SQLITE_LIMIT_VARIABLE_NUMBER default value.

pub async fn init(sqlitepath: PathBuf) -> Result<SqlitePool, MigrateError> {
    let c_opts = SqliteConnectOptions::new()
        .filename(sqlitepath)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .create_if_missing(true);
    // Specifiy higher max connections, we're using Wal, so writes don't lock reads.
    let pool = SqlitePoolOptions::new()
        .max_connections(64)
        .connect_lazy_with(c_opts);
    MIG.run(&pool).await?;
    Ok(pool)
}

// All of the types for different tables in the database

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Copy, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum RelType {
    Build,
    Mod,
    TC,
    Tool, // Not supported by FSN
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Copy, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum LinkType {
    Screenshot,
    Attachment,
    ReleaseThread,
    Video,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Copy, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum DepType {
    Required,
    Recommended, // Optional, but selected by default.
    Optional,
}

#[derive(
    Deserialize, Serialize, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, sqlx::Type,
)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum SourceLocation {
    // Ordering of these enum variants is used for preferential sorting.
    Temp, // Temporary file. These are more likely to be stored on an SSD, so we should prefer reading from them.
    Local,
    Unmanaged, // Local Directories we shouldn't control, i.e. FS2 retail, knossos etc.
    SolGate,
    FSN,
}

impl SourceLocation {
    pub fn is_local(&self) -> bool {
        *self == SourceLocation::Local
            || *self == SourceLocation::Temp
            || *self == SourceLocation::Unmanaged
    }
}

// Mini Release type for elimiating queries for existing releaes
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, sqlx::Type, sqlx::FromRow, Hash)]
pub struct Rel {
    pub name: String,
    pub version: String,
}

// Just the basics for dependency resolution.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, sqlx::Type, sqlx::FromRow)]
pub struct ModSmall {
    pub name: String,    //-* Join from releases table on rel_id
    pub version: String, //-*
    pub parent: Option<ModID>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, sqlx::Type, sqlx::FromRow)]
pub struct ModLink {
    pub name: String,    //-* Join from releases table on rel_id
    pub version: String, //-*
    pub link_type: LinkType,
    link: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Copy, sqlx::Type, sqlx::FromRow)]
pub struct ModFlags {
    pub key: i64,
    pub rel_id: i64,
    pub dep_id: i64,
}
