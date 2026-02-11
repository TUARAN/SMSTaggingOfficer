use serde::{Deserialize, Serialize};

use crate::model::batch::BatchProgress;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStatus {
  pub ok: bool,
  pub path: Option<String>,
  pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbMeta {
  pub messages_count: i64,
  pub messages_max_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelftestStatus {
  pub running: bool,
  pub ok: Option<bool>,
  pub message: String,
  pub started_at_ms: Option<i64>,
  pub finished_at_ms: Option<i64>,
  pub out_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
  pub kind: String,
  pub model_path: Option<String>,
  pub llama_cli_path: Option<String>,
  pub ollama_base_url: Option<String>,
  pub ollama_model: Option<String>,
  pub temperature: f32,
  pub max_tokens: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusSnapshot {
  pub db: DbStatus,
  pub provider_health: crate::model::provider::ProviderHealth,
  pub provider: ProviderInfo,
  pub batch: Option<BatchProgress>,
  pub selftest: SelftestStatus,
}
