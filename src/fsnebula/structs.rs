use hex;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

use crate::{common::SHA256Checksum, db};

#[derive(Deserialize, Serialize, Debug, Default, PartialEq)]
pub struct Repo {
    pub mods: Vec<FSNMod>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct FSNMod {
    pub id: String,
    pub title: String,
    pub version: String,
    pub private: bool,
    pub stability: Option<db::Stability>,
    pub parent: Option<String>,
    pub description: String,
    pub logo: Option<String>,
    pub tile: Option<String>,
    pub banner: Option<String>,
    pub screenshots: Vec<String>,
    pub attachments: Vec<String>,
    pub release_thread: Option<String>,
    pub videos: Vec<String>,
    pub notes: String,
    pub first_release: chrono::NaiveDate,
    pub last_update: chrono::NaiveDate,
    pub cmdline: String,
    pub mod_flag: Vec<String>,
    #[serde(rename = "type")]
    pub mod_type: FSNRelType,
    pub packages: Vec<FSNPackage>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct FSNPackage {
    pub name: String,
    pub notes: String,
    pub status: String,
    pub dependencies: Vec<FSNDependency>,
    pub environment: Option<String>,
    pub folder: Option<String>,
    pub is_vp: bool,
    pub executables: Vec<FSNExecutable>,
    pub files: Vec<FSNZipFile>,
    pub filelist: Vec<FSNModFile>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FSNRelType {
    Engine,
    Mod,
    TC,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct FSNDependency {
    pub id: String,
    pub version: Option<String>,
    pub packages: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct FSNExecutable {
    pub file: String,
    pub label: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct FSNZipFile {
    pub filename: String,
    pub dest: String,
    pub checksum: SHA256Checksum,
    pub filesize: i64,
    pub urls: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct FSNModFile {
    pub filename: String,
    pub archive: String,
    pub orig_name: String,
    pub checksum: SHA256Checksum,
}

// Need a custom Deserializer as Checksum is a list and not a map, i.e:
// fsnebula: ['sha256', '<HASH>']
impl<'de> Deserialize<'de> for SHA256Checksum {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ChecksumVisitor)
    }
}

pub(crate) struct ChecksumVisitor;

impl<'de> Visitor<'de> for ChecksumVisitor {
    type Value = SHA256Checksum;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("expected list of 2 strings")
    }
    // fsnebula form
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let hash_type: &str = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let hash_val: &str = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        match hash_type {
            "sha256" => Ok(SHA256Checksum(hex::decode(hash_val).unwrap())),
            //"sha512" => Ok(Self::Value::SHA512(hash_val.to_string())),
            _ => Err(de::Error::custom(format!(
                "{hash_type} not recognised",
                hash_type = hash_type
            ))),
        }
    }
}
