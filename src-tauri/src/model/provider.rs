use std::{path::PathBuf, process::Command, time::Duration};

use serde::{Deserialize, Serialize};

use crate::{settings::AppSettings, model::schema::{ClassifyPayload, LabelOutput, RULES_VERSION, SCHEMA_VERSION}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
  pub ok: bool,
  pub message: String,
  pub model_version: String,
}

#[derive(Debug, Clone)]
pub enum ProviderKind {
  Mock,
  LlamaCli,
}

pub fn parse_kind(kind: &str) -> ProviderKind {
  match kind {
    "llama_cli" => ProviderKind::LlamaCli,
    _ => ProviderKind::Mock,
  }
}

pub fn health_check(settings: &AppSettings) -> Result<ProviderHealth, String> {
  let kind = parse_kind(&settings.provider.kind);
  match kind {
    ProviderKind::Mock => Ok(ProviderHealth {
      ok: true,
      message: "mock provider (rules-only)".to_string(),
      model_version: "mock".to_string(),
    }),
    ProviderKind::LlamaCli => {
      let model_path = settings
        .provider
        .model_path
        .clone()
        .ok_or_else(|| "model_path is required".to_string())?;
      let model_path = PathBuf::from(model_path);
      if !model_path.exists() {
        return Ok(ProviderHealth {
          ok: false,
          message: "model file not found".to_string(),
          model_version: "unknown".to_string(),
        });
      }

      let cli_path = resolve_llama_cli(settings);
      if !cli_path.exists() {
        return Ok(ProviderHealth {
          ok: false,
          message: format!("llama-cli not found: {}", cli_path.display()),
          model_version: "unknown".to_string(),
        });
      }

      Ok(ProviderHealth {
        ok: true,
        message: "llama-cli ready".to_string(),
        model_version: model_path
          .file_name()
          .and_then(|s| s.to_str())
          .unwrap_or("gguf")
          .to_string(),
      })
    }
  }
}

pub trait Provider: Send + Sync {
  fn classify(&self, payload: &ClassifyPayload, timeout: Duration) -> Result<LabelOutput, String>;
  fn model_version(&self) -> String;
}

pub struct MockProvider;

impl Provider for MockProvider {
  fn classify(&self, payload: &ClassifyPayload, _timeout: Duration) -> Result<LabelOutput, String> {
    Ok(LabelOutput {
      industry: "其他".to_string(),
      sms_type: "其他".to_string(),
      entities: payload.entities.clone(),
      confidence: 0.55,
      needs_review: true,
      reasons: vec!["mock_provider".to_string()],
      signals: payload.signals.clone(),
      rules_version: RULES_VERSION.to_string(),
      model_version: "mock".to_string(),
      schema_version: SCHEMA_VERSION.to_string(),
    })
  }

  fn model_version(&self) -> String {
    "mock".to_string()
  }
}

pub struct LlamaCliProvider {
  pub llama_cli_path: PathBuf,
  pub model_path: PathBuf,
  pub temperature: f32,
  pub max_tokens: i32,
}

impl Provider for LlamaCliProvider {
  fn classify(&self, payload: &ClassifyPayload, timeout: Duration) -> Result<LabelOutput, String> {
    // NOTE: For full offline embedding, bundle llama-cli in src-tauri/resources and point settings to it.
    // We run llama-cli with a strict prompt and parse the returned JSON.
    let prompt = crate::model::prompt::build_prompt(payload);

    let mut cmd = Command::new(&self.llama_cli_path);
    cmd.arg("-m")
      .arg(&self.model_path)
      .arg("-p")
      .arg(prompt)
      .arg("-n")
      .arg(self.max_tokens.to_string())
      .arg("--temp")
      .arg(self.temperature.to_string())
      .arg("--no-display-prompt");

    let output = run_with_timeout(cmd, timeout)?;
    let text = String::from_utf8_lossy(&output).to_string();
    let json = crate::model::prompt::extract_json(&text).ok_or_else(|| "model output has no JSON".to_string())?;

    let mut parsed: LabelOutput = serde_json::from_str(&json).map_err(|e| format!("invalid JSON: {e}"))?;
    parsed.model_version = self.model_version();
    parsed.schema_version = SCHEMA_VERSION.to_string();
    Ok(parsed.normalize())
  }

  fn model_version(&self) -> String {
    self
      .model_path
      .file_name()
      .and_then(|s| s.to_str())
      .unwrap_or("gguf")
      .to_string()
  }
}

pub fn build_provider(settings: &AppSettings) -> Result<Box<dyn Provider>, String> {
  match parse_kind(&settings.provider.kind) {
    ProviderKind::Mock => Ok(Box::new(MockProvider)),
    ProviderKind::LlamaCli => {
      let model_path = settings
        .provider
        .model_path
        .clone()
        .ok_or_else(|| "model_path is required".to_string())?;
      let model_path = PathBuf::from(model_path);
      if !model_path.exists() {
        return Err("model file not found".to_string());
      }
      let llama_cli_path = resolve_llama_cli(settings);
      if !llama_cli_path.exists() {
        return Err(format!("llama-cli not found: {}", llama_cli_path.display()));
      }
      Ok(Box::new(LlamaCliProvider {
        llama_cli_path,
        model_path,
        temperature: settings.provider.temperature,
        max_tokens: settings.provider.max_tokens,
      }))
    }
  }
}

fn resolve_llama_cli(settings: &AppSettings) -> PathBuf {
  if let Some(p) = settings.provider.llama_cli_path.as_ref() {
    return PathBuf::from(p);
  }
  // default bundled path: src-tauri/resources/llama-cli (user should place it there for offline run)
  PathBuf::from("resources").join("llama-cli")
}

fn run_with_timeout(mut cmd: Command, timeout: Duration) -> Result<Vec<u8>, String> {
  // Minimal cross-platform timeout: spawn then poll.
  // If timeout reached, kill the child.
  use std::time::Instant;

  let mut child = cmd
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()
    .map_err(|e| e.to_string())?;

  let start = Instant::now();
  loop {
    if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
      let out = child
        .wait_with_output()
        .map_err(|e| e.to_string())?;
      if !status.success() {
        let err = String::from_utf8_lossy(&out.stderr).to_string();
        return Err(format!("llama-cli failed: {err}"));
      }
      return Ok(out.stdout);
    }

    if start.elapsed() >= timeout {
      let _ = child.kill();
      return Err("llama-cli timeout".to_string());
    }
    std::thread::sleep(Duration::from_millis(20));
  }
}
