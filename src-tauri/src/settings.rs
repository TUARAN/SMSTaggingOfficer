use std::{fs, path::PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSettings {
  pub kind: String,
  pub model_path: Option<String>,
  pub llama_cli_path: Option<String>,
  pub temperature: f32,
  pub max_tokens: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
  pub provider: ProviderSettings,
}

impl Default for AppSettings {
  fn default() -> Self {
    Self {
      provider: ProviderSettings {
        kind: "mock".to_string(),
        model_path: None,
        llama_cli_path: None,
        temperature: 0.1,
        max_tokens: 512,
      },
    }
  }
}

pub struct SettingsStore {
  path: PathBuf,
  inner: Mutex<AppSettings>,
}

impl SettingsStore {
  pub fn load(path: PathBuf) -> Result<Self, String> {
    if let Ok(text) = fs::read_to_string(&path) {
      if let Ok(parsed) = serde_json::from_str::<AppSettings>(&text) {
        return Ok(Self {
          path,
          inner: Mutex::new(parsed),
        });
      }
    }

    let store = Self {
      path,
      inner: Mutex::new(AppSettings::default()),
    };
    store.persist()?;
    Ok(store)
  }

  pub fn get(&self) -> parking_lot::MutexGuard<'_, AppSettings> {
    self.inner.lock()
  }

  pub fn set(&self, settings: AppSettings) -> Result<(), String> {
    *self.inner.lock() = settings;
    self.persist()
  }

  fn persist(&self) -> Result<(), String> {
    if let Some(dir) = self.path.parent() {
      fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&*self.inner.lock()).map_err(|e| e.to_string())?;
    fs::write(&self.path, text).map_err(|e| e.to_string())
  }
}
