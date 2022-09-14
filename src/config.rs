use platform_dirs::AppDirs;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::error::Error;
use std::fs::{self};
use std::io;
use std::path::PathBuf;
use toml;

use crate::fsnebula::FSNPaths;

//TOML crate can't serialize Enums, so be careful here.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    #[serde(default)]
    pub fsnebula: FSNPaths,
    pub paths: Paths,
    pub joystick: Vec<Joystick>,
    #[serde(skip)]
    pub config_path: PathBuf, // Store the path to the config file in order to make saving easier.
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fsnebula: Default::default(),
            paths: Default::default(),
            joystick: Default::default(),
            config_path: default_dir(),
        }
    }
}

impl Config {
    pub fn read(config_path: Option<PathBuf>) -> Result<Config, ReadError> {
        let path = config_path.unwrap_or_else(|| default_dir().join("config.toml"));
        match fs::read_to_string(&path) {
            Ok(conf_str) => match toml::from_str::<Config>(&conf_str) {
                Ok(mut config) => {
                    config.config_path = path;
                    Ok(config)
                }
                Err(parse_error) => Err(ReadError::ParseError(parse_error)),
            },
            Err(error) => Err(ReadError::IOError(error)),
        }
    }
    pub fn save(&self) -> Result<(), std::io::Error> {
        let config_string: String = toml::to_string_pretty(&self).unwrap();
        fs::write(&self.config_path, config_string.as_str())
    }
}

pub fn default_dir() -> PathBuf {
    let app_dirs = AppDirs::new(Some("sol-gate"), true).unwrap();
    fs::create_dir_all(&app_dirs.config_dir).unwrap();
    app_dirs.config_dir
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct Paths {
    pub fs2_root: String,
    pub base_dir: String,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct Joystick {
    pub guid: String,
    pub id: i32, // TODO - need SDL2 bindings for this.
}

#[derive(Debug)]
pub enum ReadError {
    ParseError(toml::de::Error),
    IOError(std::io::Error),
}
