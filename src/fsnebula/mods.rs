use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

use crate::common::{self, ModType};

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Repo {
    pub mods: Vec<FSNMod>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FSNMod {
    pub id: String,
    pub title: String,
    pub version: String,
    pub private: bool,
    pub stability: Option<common::Stability>,
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
    pub first_release: String,
    pub last_update: String,
    pub cmdline: String,
    pub mod_flag: Vec<String>,
    #[serde(rename = "type")]
    pub mod_type: ModType,
    pub packages: Vec<FSNPackage>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FSNDependency {
    pub id: String,
    pub version: Option<String>,
    pub packages: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FSNExecutable {
    pub file: String,
    pub label: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FSNZipFile {
    pub filename: String,
    pub dest: String,
    pub checksum: FSNChecksum,
    pub filesize: usize,
    pub urls: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FSNModFile {
    pub filename: String,
    pub archive: String,
    pub orig_name: String,
    pub checksum: FSNChecksum,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]

pub enum FSNChecksum {
    SHA256(String),
    SHA512(String), // doesn't exist but w/e, don't know if we should bother supporting other types,
                    // unlikely to see an update to cover more of them
}

// Need a custom Deserializer as Checksum is a list and not a map, i.e:
// fsnebula: ['sha256', '<HASH>']
impl<'de> Deserialize<'de> for FSNChecksum {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ChecksumVisitor)
    }
}

pub(crate) struct ChecksumVisitor;

impl<'de> Visitor<'de> for ChecksumVisitor {
    type Value = FSNChecksum;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("list of 2 strings")
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
            "sha256" => Ok(Self::Value::SHA256(hash_val.to_string())),
            "sha512" => Ok(Self::Value::SHA512(hash_val.to_string())),
            _ => Err(de::Error::custom(format!(
                "{hash_type} not recognised",
                hash_type = hash_type
            ))),
        }
    }
}

impl From<FSNMod> for common::Item {
    fn from(fsn_mod: FSNMod) -> Self {
        match fsn_mod.mod_type {
            ModType::Engine => Self::Engine(common::Engine {
                id: fsn_mod.id,
                version: fsn_mod.version,
                private: fsn_mod.private,
                stability: fsn_mod
                    .stability
                    .expect("Engine build doesn't have stability set!"),
                details: common::Details {
                    description: fsn_mod.description,
                    logo: fsn_mod.logo,
                    tile: fsn_mod.tile,
                    banner: fsn_mod.banner,
                    screenshots: fsn_mod.screenshots,
                    attachments: fsn_mod.attachments,
                    release_thread: fsn_mod.release_thread,
                    videos: fsn_mod.videos,
                    notes: fsn_mod.notes,
                    first_release: fsn_mod.first_release,
                    last_update: fsn_mod.last_update,
                },
                builds: fsn_mod
                    .packages
                    .iter()
                    .map(|p| common::Build::from(p.clone()))
                    .collect(),
            }),
            _ => Self::Mod(common::Mod {
                id: fsn_mod.id,
                title: fsn_mod.title,
                version: fsn_mod.version,
                private: fsn_mod.private,
                parent: fsn_mod.parent,
                details: common::Details {
                    description: fsn_mod.description,
                    logo: fsn_mod.logo,
                    tile: fsn_mod.tile,
                    banner: fsn_mod.banner,
                    screenshots: fsn_mod.screenshots,
                    attachments: fsn_mod.attachments,
                    release_thread: fsn_mod.release_thread,
                    videos: fsn_mod.videos,
                    notes: fsn_mod.notes,
                    first_release: fsn_mod.first_release,
                    last_update: fsn_mod.last_update,
                },
                cmdline: fsn_mod.cmdline,
                mod_flag: fsn_mod.mod_flag,
                mod_type: fsn_mod.mod_type,
                packages: fsn_mod
                    .packages
                    .iter()
                    .map(|pak| common::Package::from(pak.clone()))
                    .collect(),
                // ),
            }),
        }
    }
}

impl From<FSNPackage> for common::Package {
    fn from(_: FSNPackage) -> Self {
        todo!()
    }
}

impl From<FSNPackage> for common::Build {
    fn from(pack: FSNPackage) -> Self {
        todo!()
    }
}
