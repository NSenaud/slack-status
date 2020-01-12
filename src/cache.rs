extern crate serde_json;

use std::fs::File;
use std::error::Error;
use std::fs::create_dir_all;
use std::io::prelude::*;
use std::path::PathBuf;
use directories::ProjectDirs;

type BoxResult<T> = Result<T,Box<dyn Error>>;

/// Slack Status, as sent to the API.
#[derive(Serialize, Deserialize, Clone)]
pub struct StatusCache {
    pub text: String,
    pub emoji: String,
    pub expiration: i64,
}

/// Cache status and keep track if it was manually set.
#[derive(Serialize, Deserialize, Clone)]
pub struct Cache {
    pub status: StatusCache,
    pub manually_set: bool,
}

impl Cache {
    /// Get the cache file path either provided by the user or look at
    /// default location:
    ///
    /// * Linux: /home/alice/.cache/slack-status/status.json
    /// * Mac: /Users/Alice/Library/Caches/com.nsd.slack-status/status.json
    /// * Windows: C:\Users\Alice\AppData\Roaming\nsd\slack-status\cache\status.json
    fn get_file_path() -> Option<PathBuf> {
        // Default OS location configuration path.
        if let Some(proj_dirs) = ProjectDirs::from("com", "nsd", "slack-status") {
            let cache_dir = proj_dirs.cache_dir();
            debug!("Looking for cache file in: {:?}", cache_dir);

            if !cache_dir.to_path_buf().exists() {
                debug!("Cache directory does not exists, creating it.");
                create_dir_all(cache_dir.to_str().unwrap()).unwrap();
            }

            Some(cache_dir.to_path_buf().join("status.json"))
        } else {
            warn!("Cannot find application cache directory.");
            None
        }
    }

    /// Read cache file from default OS location.
    pub fn read() -> BoxResult<Option<Cache>> {
        if let Some(cache_file_path) = Cache::get_file_path() {
            match File::open(cache_file_path) {
                Ok(mut f) => {
                    let mut contents = String::new();
                    f.read_to_string(&mut contents)
                        .expect("something went wrong reading the file");

                    let cache = match serde_json::from_str(contents.as_str()) {
                        Ok(c) => c,
                        Err(e) => bail!("Deserialization error: {}", e),
                    };

                    Ok(Some(cache))
                }
                Err(e) => {
                    warn!("Cannot read cache directory: {}.", e);
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }

    /// Save cache file at default OS location.
    pub fn save(&self) -> BoxResult<()> {
        if let Some(cache_file_path) = Cache::get_file_path() {
            match File::create(cache_file_path) {
                Ok(mut f) => {
                    debug!("Cache file: {:#?}", f);

                    let cache = match serde_json::to_string(self) {
                        Ok(c) => c,
                        Err(e) => bail!("Serialization error: {}", e),
                    };
                    debug!("Cache file content: {}", cache);

                    match f.write_all(cache.as_bytes()) {
                        Ok(_) => {
                            info!("Cache file saved");
                            Ok(())
                        },
                        Err(e) => bail!(e),
                    }
                }
                Err(e) => {
                    bail!("Cannot read cache directory: {}.", e)
                }
            }
        } else {
            bail!("Cannot find application cache directory.");
        }
    }

    /// Reset cache (remove cache file).
    pub fn reset() -> BoxResult<()> {
        if let Some(cache_file_path) = Cache::get_file_path() {
            std::fs::remove_file(cache_file_path)?;
            Ok(())
        } else {
            bail!("Cannot find application cache directory.");
        }
    }
}
