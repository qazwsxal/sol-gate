use crate::common;
use crate::config;
use hyper::header::{ETAG, IF_NONE_MATCH};
use hyper::StatusCode;
use reqwest;
use serde::{Deserialize, Serialize};
use std::io::Error;
use std::path::PathBuf;

pub mod mods;

#[derive(Deserialize, Serialize, Debug)]
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
    pub repo: mods::Repo,
    urls: FSNPaths,
    cache: PathBuf,
}

pub enum InitError {}

impl FSNebula {
    pub async fn init(urls: FSNPaths, appdir: PathBuf) -> Result<Self, Error> {
        let cache = appdir.join("fsnebula");
        tokio::fs::create_dir_all(&cache).await.unwrap();
        let client = reqwest::Client::new();
        let etag_path = cache.join("mods.json.etag");
        let etag = match tokio::fs::read_to_string(&etag_path).await {
            Ok(val) => val,
            Err(e) => String::default(),
        };
        let mods_json: Option<String> = None;
        for repo_url in urls.repos.iter() {
            let req_result = client
                .get(repo_url)
                .header(IF_NONE_MATCH, etag.clone())
                .send()
                .await;
            match req_result {
                Err(_) => continue, // Try repo.json url
                Ok(response) => match response.status() {
                    StatusCode::OK => {
                        match response.headers().get(ETAG) {
                            Some(tag) => tokio::fs::write(&etag_path, tag.to_str().unwrap())
                                .await
                                .unwrap(),
                            None => (),
                        }
                        mods_json = Some(response.text().await.unwrap());
                        tokio::fs::write(&cache.join("mods.json"), &mods_json.unwrap()).await.unwrap();
                        break;
                    }
                    StatusCode::NOT_MODIFIED => {
                        mods_json = Some(tokio::fs::read_to_string(cache.join("mods.json"))
                            .await
                            .unwrap());
                        break;
                    }
                    _ => todo!(),
                },
            }
        }
        let repo: mods::Repo = match serde_json::from_str(&mods_json) {
            Ok(x) => x,
            Err(_) => todo!(),
        };
        Ok(Self { repo, urls, cache })
    }
}
