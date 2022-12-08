use reqwest::{
    self,
    header::{ETAG, IF_NONE_MATCH},
    StatusCode,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{fmt, path::Path};

mod db;
pub mod api;
pub mod structs;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FSNPaths {
    pub web: String,
    pub api: String,
    pub repos: Vec<String>,
}

impl Default for FSNPaths {
    fn default() -> Self {
        FSNPaths {
            web: "https://fsnebula.org/".to_string(),
            api: "https://api.fsnebula.org/api/1/".to_string(),
            repos: vec![
                "https://cf.fsnebula.org/storage/repo.json".to_string(),
                "https://fsnebula.org/storage/repo.json".to_string(),
                "https://porphyrion.feralhosting.com/datacorder/nebula/repo.json".to_string(),
            ],
        }
    }
}

#[derive(Debug, Default)]
pub struct FSNebula {
    pub repo: structs::Repo,
    urls: FSNPaths,
    cache: PathBuf,
}

#[derive(Debug)]
pub enum InitError {
    IOError(std::io::Error),
    ParseError(serde_json::Error),
    RequestError(reqwest::Error),
}
impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", &self)
    }
}
impl std::error::Error for InitError {}
impl From<serde_json::Error> for InitError {
    fn from(err: serde_json::Error) -> Self {
        InitError::ParseError(err)
    }
}

impl From<std::io::Error> for InitError {
    fn from(err: std::io::Error) -> Self {
        InitError::IOError(err)
    }
}

impl From<reqwest::Error> for InitError {
    fn from(err: reqwest::Error) -> Self {
        InitError::RequestError(err)
    }
}

impl FSNebula {
    pub async fn init(urls: FSNPaths, cache: impl AsRef<Path>) -> Result<Self, InitError> {
        // let cache = appdir.join("fsnebula");
        tokio::fs::create_dir_all(&cache).await?;
        let mods_path = cache.as_ref().clone().join("mods.json");
        let client = reqwest::Client::new();
        let etag_path = cache.as_ref().clone().join("mods.json.etag");
        let etag: String = tokio::fs::read_to_string(&etag_path)
            .await
            .unwrap_or_else(|_| String::default());
        let mut mods_json: Option<String> = None;
        for repo_url in urls.repos.iter() {
            let req_result = client
                .get(repo_url)
                .header(IF_NONE_MATCH, etag.clone())
                .send()
                .await;
            match req_result {
                Err(_) => continue, // Try next repo.json url
                Ok(response) => match response.status() {
                    StatusCode::OK => {
                        match response.headers().get(ETAG) {
                            Some(tag) => {
                                tokio::fs::write(&etag_path, tag.to_str().unwrap()).await?
                            }
                            None => (),
                        }
                        let resp = response.text().await?;
                        tokio::fs::write(&mods_path, &resp).await?;
                        mods_json = Some(resp);
                        break;
                    }
                    StatusCode::NOT_MODIFIED => break,
                    _ => continue,
                },
            }
        }
        if mods_json.is_none() {
            mods_json = Some(tokio::fs::read_to_string(mods_path).await?);
        }
        Ok(Self {
            repo: serde_json::from_str(&mods_json.unwrap())?,
            urls,
            cache: cache.as_ref().to_path_buf(),
        })
    }
}
