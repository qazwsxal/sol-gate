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

type ModID = String;

#[derive(Deserialize, Serialize, Debug, Copy, Clone, PartialEq, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum Stability {
    Stable,
    RC,
    Nightly,
    Experimental,
    // Experimental not supported by FSN,
    // Need to differentiate to prevent standalones
    // From running arbritrary exes.
}

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
    Deserialize, Serialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Copy, Hash, sqlx::Type,
)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum EntryType {
    // Ordering of these enum variants is used for preferential sorting.
    Raw, // Source is a flat file, .png, .txt, .vp, .7z etc. This is *the file itself* not anything in it.
    VPEntry, // Source is an entry in a VP file. Parent must be set.
    SevenZipEntry, // Source is an entry in a 7z file, Parent must be set.
}

#[derive(
    Deserialize, Serialize, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, sqlx::Type,
)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum SourceLocation {
    // Ordering of these enum variants is used for preferential sorting.
    Local,
    Temp, // Temporary file.
    SolGate,
    FSN,
}

impl SourceLocation {
    pub fn is_local(&self) -> bool {
        *self == SourceLocation::Local || *self == SourceLocation::Temp
    }
}
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, sqlx::Type, sqlx::FromRow)]
pub struct Release {
    pub rel_id: i64,
    pub name: String,
    pub version: String,
    pub rel_type: RelType,
    pub date: NaiveDate,
    pub private: bool,
}

// Mini Release type for elimiating queries for existing releaes
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, sqlx::Type, sqlx::FromRow, Hash)]
pub struct Rel {
    pub name: String,
    pub version: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, sqlx::Type, sqlx::FromRow)]
pub struct Mod {
    pub name: String,    //-* Join from releases table on rel_id
    pub version: String, // |
    pub title: String,   // |
    pub date: NaiveDate, // |
    pub private: bool,   //-*
    pub parent: Option<ModID>,
    pub description: Option<String>,
    pub logo: Option<String>,
    pub tile: Option<String>,
    pub banner: Option<String>,
    pub notes: Option<String>,
    pub cmdline: String,
}

// Just the basics for dependency resolution.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, sqlx::Type, sqlx::FromRow)]
pub struct ModSmall {
    pub name: String,    //-* Join from releases table on rel_id
    pub version: String, //-*
    pub parent: Option<ModID>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, sqlx::Type, sqlx::FromRow)]
pub struct Build {
    pub name: String,    //-* Join from releases table on rel_id
    pub version: String, // |
    pub title: String,   // |
    pub date: NaiveDate, // |
    pub private: bool,   //-*
    pub stability: Stability,
    pub description: Option<String>,
    pub notes: Option<String>,
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, sqlx::Type, sqlx::FromRow)]
pub struct Package {
    pub rel_id: i64,
    pub p_id: i64,
    pub name: String,
    pub notes: String,
    pub status: DepType,
    pub environment: Option<String>,
    pub folder: String,
    pub is_vp: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, sqlx::Type, sqlx::FromRow)]
pub struct PackageDependency {
    pub key: i64,
    pub p_id: i64,
    pub modname: String,
    pub modver: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, sqlx::Type, sqlx::FromRow)]
pub struct DependencyDetail {
    pub id: i64,
    pub dep_id: i64,
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, sqlx::Type, sqlx::FromRow)]
pub struct Hash {
    pub id: i64,
    pub val: SHA256Checksum,
    pub size: Option<i64>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, sqlx::Type, sqlx::FromRow)]
pub struct File {
    pub p_id: i64,
    pub h_id: i64,
    pub filepath: String,
}

#[derive(
    Deserialize,
    Serialize,
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    sqlx::Type,
    sqlx::FromRow,
    Hash,
)]
pub struct Source {
    // struct member order is specified so that Ord can be derived automatically in a way we want
    pub location: SourceLocation,
    pub path: String,
    pub h_id: i64,
    pub size: i64,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, sqlx::Type, sqlx::FromRow)]
pub struct SourceWithID {
    pub id: i64,
    pub location: SourceLocation,
    pub path: String,
    pub h_id: i64,
    pub size: i64,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, sqlx::Type, sqlx::FromRow)]
pub struct Parent {
    pub child: i64,
    pub parent: i64,
    pub child_path: String,
    pub par_type: EntryType,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Hash, sqlx::Type, sqlx::FromRow)]
#[sqlx(transparent)]
pub struct SHA256Checksum(pub Vec<u8>);
