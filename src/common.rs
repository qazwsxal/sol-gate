use crate::db::{DepType, RelType, SourceLocation};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

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

#[derive(
    Deserialize, Serialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Copy, Hash, sqlx::Type,
)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum Archive {
    // Ordering of these enum variants is used for preferential sorting.
    VP,       // Source is an entry in a VP file.
    SevenZip, // Source is an entry in a 7z file.
}

#[derive(
    Deserialize, Serialize, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, sqlx::Type,
)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum SourceFormat {
    Raw,
    SevenZip,
    VP,
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, sqlx::Type, sqlx::FromRow)]
pub struct Mod {
    pub name: String,    //-* Join from releases table on rel_id
    pub version: String, // |
    pub title: String,   // |
    pub date: NaiveDate, // |
    pub private: bool,   //-*
    pub parent: Option<String>,

    pub description: Option<String>,
    pub logo: Option<String>,
    pub tile: Option<String>,
    pub banner: Option<String>,
    pub notes: Option<String>,
    pub cmdline: String,
    pub installed: bool,
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
pub struct Package {
    pub p_id: i64,
    pub rel_id: i64,
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
    pub format: SourceFormat,
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
pub struct ArchiveEntry {
    pub file_id: i64,
    pub file_path: String,
    pub archive_id: i64,
    pub archive_type: Archive,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq, Hash, sqlx::Type, sqlx::FromRow)]
#[sqlx(transparent)]
pub struct SHA256Checksum(pub Vec<u8>);
