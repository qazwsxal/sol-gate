pub use crate::db::*;
use hex;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Engine {
    pub id: String,
    pub version: String,
    pub private: bool,
    pub stability: Stability,
    pub builds: Vec<Build>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ModDetails {
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
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Build {
    pub platform: Platform,
    pub cpu: CPU,
    pub executables: Vec<Executable>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Platform {
    Windows,
    OSX,
    Linux,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum CPU {
    X86(X86features),
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct X86features {
    x64: bool,
    sse2: bool,
    avx: bool,
    avx2: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Package {
    pub name: String,
    pub notes: String,
    pub status: String,
    pub dependencies: Vec<Dependency>,
    pub environment: Option<String>,
    pub folder: Option<String>,
    pub is_vp: bool,
    pub executables: Vec<Executable>,
    pub files: Vec<ZipFile>,
    pub filelist: Vec<ModFile>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Dependency {
    pub id: String,
    pub version: Option<String>,
    pub packages: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Executable {
    pub file: String,
    pub label: ExeType,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum ExeType {
    Release,
    FastDbg,
    Debug,
    Fred,
    FredFastDbg,
    FredFullDbg,
    QtFred,
    QtFredFastDbg,
    QtFredFullDbg,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct ZipFile {
    pub filename: String,
    pub dest: String,
    pub checksum: Checksum,
    pub filesize: usize,
    pub urls: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct ModFile {
    pub filename: String,
    pub archive: String,
    pub orig_name: String,
    pub checksum: Checksum,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Checksum {
    pub hash_id: u64,
    #[serde(with = "hex")]
    pub sha256: [u8; 32],
}
