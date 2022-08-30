use platform_dirs::{AppDirs, UserDirs};
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fs::{self, File};
use std::io::{self, BufReader, Error, ErrorKind};
use std::path::{self, Path, PathBuf};
use toml;

use crate::fsnebula::FSNPaths;

//TOML crate can't serialize Enums, so be careful here.
#[derive(Deserialize, Serialize, Debug)]
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
    pub fn new(paths: Paths) -> Self {
        Config {
            paths,
            ..Config::default()
        }
    }

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
    pub fn save(&self) -> Result<(), Error> {
        let config_string: String = toml::to_string_pretty(&self).unwrap();
        fs::write(&self.config_path, config_string.as_str())
    }
}

pub fn default_dir() -> PathBuf {
    let app_dirs = AppDirs::new(Some("sol-gate"), true).unwrap();
    fs::create_dir_all(&app_dirs.config_dir).unwrap();
    app_dirs.config_dir
}

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct Paths {
    pub fs2_root: String,
    pub base_dir: String,
}

#[derive(Deserialize, Serialize, Default, Debug)]
pub struct Joystick {
    pub guid: String,
    pub id: i32, // TODO - need SDL2 bindings for this.
}

#[derive(Debug)]
pub enum ReadError {
    ParseError(toml::de::Error),
    IOError(std::io::Error),
}

fn get_conf_path(path: Option<PathBuf>) -> PathBuf {
    if path.is_some() {
        path.unwrap()
    } else {
        let app_dirs = AppDirs::new(Some("sol-gate"), true).unwrap();
        fs::create_dir_all(&app_dirs.config_dir).unwrap();
        app_dirs.config_dir.join("config.toml")
    }
}

pub fn setup() -> Config {
    println!("Set FS2 Directory:");
    let mut fs2_dir = String::new();
    match io::stdin().read_line(&mut fs2_dir) {
        Ok(_) => println!("FS2 root directory:\n{fs2_dir}"),
        Err(error) => println!("error:\n{error}"),
    }
    println!("Set sol-gate root directory:");
    let mut sol_gate_dir = String::new();
    match io::stdin().read_line(&mut sol_gate_dir) {
        Ok(_) => println!("sol-gate root directory:\n{sol_gate_dir}"),
        Err(error) => println!("error:\n{error}"),
    }
    let paths = Paths {
        fs2_root: fs2_dir.trim_end().to_string(),
        base_dir: sol_gate_dir.trim_end().to_string(),
    };
    Config::new(paths)
}
