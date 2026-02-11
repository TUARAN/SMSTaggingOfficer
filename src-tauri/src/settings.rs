use std::{fs, path::PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSettings {
  pub kind: String,
  pub model_path: Option<String>,
  pub llama_cli_path: Option<String>,
  #[serde(default)]
  pub ollama_base_url: Option<String>,
  #[serde(default)]
  pub ollama_model: Option<String>,
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
        kind: "ollama".to_string(),
        model_path: None,
        llama_cli_path: None,
        ollama_base_url: Some("http://127.0.0.1:11434".to_string()),
        ollama_model: Some("llama3.2:1b".to_string()),
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
        let mut parsed = parsed;
        let migrated = migrate_default_mock_to_ollama(&mut parsed);

        let store = Self {
          path,
          inner: Mutex::new(parsed),
        };

        if migrated {
          store.persist()?;
        }
        return Ok(store);
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

fn migrate_default_mock_to_ollama(settings: &mut AppSettings) -> bool {
  // Only migrate if the user is effectively on the *old default* mock settings.
  // This avoids surprising users who intentionally chose mock or configured llama-cli.
  let p = &settings.provider;
  let looks_like_old_default_mock = p.kind == "mock"
    && p.model_path.is_none()
    && p.llama_cli_path.is_none()
    && p.temperature == 0.1
    && p.max_tokens == 512
    && p.ollama_base_url.is_none()
    && p.ollama_model.is_none();

  if !looks_like_old_default_mock {
    return false;
  }

  settings.provider.kind = "ollama".to_string();
  settings.provider.ollama_base_url = Some("http://127.0.0.1:11434".to_string());
  settings.provider.ollama_model = Some("llama3.2:1b".to_string());
  true
}
