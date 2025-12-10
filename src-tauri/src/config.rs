//! `lore_lib` configuration.

use serde::{Deserialize, Serialize, de};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// The path to the config file.
pub static CONFIG_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let sys_cfg_dir =
        dirs::config_dir().expect("Supported operating systems are Linux, macOS, and Windows");
    sys_cfg_dir.join("lore/config.json")
});

/// The path to the default store location.
pub static DEFAULT_STORE_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let sys_cfg_dir =
        dirs::config_dir().expect("Supported operating systems are Linux, macOS, and Windows");
    
    // Windows
    #[cfg(target_os = "windows")]
    {
        sys_cfg_dir.join("lore\\store")
    }

    // Unix
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        sys_cfg_dir.join("lore/store")
    }
});

/// The application config.
///
/// Stored as a JSON object in the user's config directory (e.g. `AppData\Roaming`, `~/.config`).
#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    /// The store directory. Invariant that it must never be
    #[serde(deserialize_with = "deserialize_store_dir_or_default")]
    store_dir: PathBuf,
}

fn deserialize_store_dir_or_default<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: de::Deserializer<'de>,
{
    let path_from_config: String = de::Deserialize::deserialize(deserializer)?;
    let path = PathBuf::from(path_from_config);

    if !path.exists() || !path.is_dir() {
        return Ok(DEFAULT_STORE_PATH.clone());
    }

    Ok(path)
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            store_dir: DEFAULT_STORE_PATH.clone(),
        }
    }
}

impl AppConfig {
    pub fn store_dir(&self) -> &Path {
        &self.store_dir
    }
}
