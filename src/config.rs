use platform_dirs::AppDirs;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fs::{self};
use std::path::{Path, PathBuf};
use toml;

pub mod api;
//TOML crate can't serialize Enums, so be careful here.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    #[serde(default)]
    pub fsnebula: FSNPaths,
    pub local_settings: LocalSettings,
    pub gates: Vec<Gate>,
    pub joystick: Vec<Joystick>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fsnebula: FSNPaths::new(Self::default_dir()),
            local_settings: LocalSettings::default(),
            gates: Default::default(),
            joystick: Default::default(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct FSNPaths {
    pub web: String,
    pub api: String,
    pub repos: Vec<String>,
    pub cache: PathBuf,
}

impl FSNPaths {
    fn new(appdir: impl AsRef<Path>) -> Self {
        FSNPaths {
            web: "https://fsnebula.org/".to_string(),
            api: "https://api.fsnebula.org/api/1/".to_string(),
            repos: vec![
                "https://cf.fsnebula.org/storage/repo.json".to_string(),
                "https://fsnebula.org/storage/repo.json".to_string(),
                "https://discovery.aigaion.feralhosting.com/nebula/repo.json".to_string(),
            ],
            cache: appdir.as_ref().clone().join("fsnebula"),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Gate {
    pub name: String,
    pub url: String,
    pub user: Option<String>,
    pub pass: Option<String>,
}

impl Config {
    pub fn read(config_path: Option<PathBuf>) -> Result<Config, ConfigReadError> {
        let path = match config_path {
            Some(path) => path,
            None => Self::default_dir().join("config.toml"),
        };

        let conf_str = fs::read_to_string(&path).map_err(|err| ConfigReadError::IOError(err))?;

        toml::from_str::<Config>(&conf_str).map_err(|err| ConfigReadError::ParseError(err))
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let config_string = toml::to_string_pretty(&self).unwrap();
        let config_str = config_string.as_str();
        fs::write(Self::default_dir(), config_str)
    }

    pub fn default_dir() -> PathBuf {
        let app_dirs = AppDirs::new(Some("sol-gate"), true).unwrap();
        fs::create_dir_all(&app_dirs.config_dir).unwrap();
        app_dirs.config_dir
    }
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct LocalSettings {
    pub fs2_root: PathBuf,
    pub install_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub hdd_mode: bool,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct Joystick {
    pub guid: String,
    pub id: i32, // TODO - need SDL2 bindings for this.
}

#[derive(Debug)]
pub enum ConfigReadError {
    ParseError(toml::de::Error),
    IOError(std::io::Error),
}
