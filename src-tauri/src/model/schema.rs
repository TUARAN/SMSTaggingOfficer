use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "schema_v1";
pub const RULES_VERSION: &str = "rules_v1";

// 一级标签：行业大类（固定枚举）
pub const INDUSTRIES: [&str; 6] = ["金融", "通用", "政务", "渠道", "互联网", "其他"];

// 二级标签：短信类型（固定枚举）
pub const SMS_TYPES: [&str; 10] = [
  "验证码",
  "交易提醒",
  "账单催缴",
  "保险续保",
  "物流取件",
  "会员账号变更",
  "政务通知",
  "风险提示",
  "营销推广",
  "其他",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Entities {
  pub brand: Option<String>,
  pub verification_code: Option<String>,
  pub amount: Option<f64>,
  pub balance: Option<f64>,
  pub account_suffix: Option<String>,
  pub time_text: Option<String>,
  pub url: Option<String>,
  pub phone_in_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelOutput {
  pub industry: String,
  #[serde(rename = "type")]
  pub sms_type: String,
  pub entities: Entities,
  pub confidence: f64,
  pub needs_review: bool,
  pub reasons: Vec<String>,
  pub signals: HashMap<String, serde_json::Value>,
  pub rules_version: String,
  pub model_version: String,
  pub schema_version: String,
}

impl LabelOutput {
  pub fn normalize(mut self) -> Self {
    if !INDUSTRIES.contains(&self.industry.as_str()) {
      self.industry = "其他".to_string();
      self.needs_review = true;
      self.reasons.push("normalize:invalid_industry".to_string());
    }
    if !SMS_TYPES.contains(&self.sms_type.as_str()) {
      self.sms_type = "其他".to_string();
      self.needs_review = true;
      self.reasons.push("normalize:invalid_type".to_string());
    }

    if !self.confidence.is_finite() {
      self.confidence = 0.5;
      self.needs_review = true;
      self.reasons.push("normalize:invalid_confidence".to_string());
    }
    if self.confidence < 0.0 {
      self.confidence = 0.0;
    }
    if self.confidence > 1.0 {
      self.confidence = 1.0;
    }

    if self.rules_version.is_empty() {
      self.rules_version = RULES_VERSION.to_string();
    }
    if self.schema_version.is_empty() {
      self.schema_version = SCHEMA_VERSION.to_string();
    }
    if self.reasons.is_empty() {
      self.reasons.push("no_reason".to_string());
    }
    self.schema_version = SCHEMA_VERSION.to_string();
    self
  }

  pub fn error_fallback(
    entities: Entities,
    signals: HashMap<String, serde_json::Value>,
    err: &str,
  ) -> Self {
    LabelOutput {
      industry: "其他".to_string(),
      sms_type: "其他".to_string(),
      entities,
      confidence: 0.25,
      needs_review: true,
      reasons: vec![format!("model_error:{err}")],
      signals,
      rules_version: RULES_VERSION.to_string(),
      model_version: "error".to_string(),
      schema_version: SCHEMA_VERSION.to_string(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRow {
  pub id: i64,
  pub content: String,
  pub received_at: Option<String>,
  pub sender: Option<String>,
  pub phone: Option<String>,
  pub source: Option<String>,
  pub has_url: bool,
  pub has_amount: bool,
  pub has_verification_code: bool,
  pub label: Option<LabelOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyPayload {
  pub message_id: i64,
  pub content: String,
  pub entities: Entities,
  pub signals: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyResult {
  pub label: LabelOutput,
}
