use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::model::schema::{Entities, LabelOutput, RULES_VERSION, SCHEMA_VERSION};

#[derive(Debug, Clone)]
pub struct RuleResult {
  pub label: Option<LabelOutput>,
  pub entities: Entities,
  pub signals: HashMap<String, serde_json::Value>,
  pub strong_hit: bool,
}

pub fn run_rules(content: &str, sender: Option<&str>) -> RuleResult {
  let mut signals: HashMap<String, serde_json::Value> = HashMap::new();
  let entities = extract_entities(content, sender, &mut signals);

  // Strong patterns first
  if let Some(code) = entities.verification_code.clone() {
    if contains_any(content, &["验证码", "校验码", "动态码", "OTP"]) {
      signals.insert("rule".to_string(), serde_json::json!("verification_code"));
      return RuleResult {
        label: Some(LabelOutput {
          industry: guess_industry_from_sender(sender).unwrap_or_else(|| "通用".to_string()),
          sms_type: "验证码".to_string(),
          entities: entities.clone(),
          confidence: 0.98,
          needs_review: false,
          reasons: vec![format!("rule: verification_code={code}")],
          signals: signals.clone(),
          rules_version: RULES_VERSION.to_string(),
          model_version: "n/a".to_string(),
          schema_version: SCHEMA_VERSION.to_string(),
        }),
        entities,
        signals,
        strong_hit: true,
      };
    }
  }

  if contains_any(content, &["取件码", "快递", "驿站", "柜", "丰巢", "菜鸟", "中通", "圆通", "申通", "韵达", "顺丰", "京东物流"]) {
    signals.insert("rule".to_string(), serde_json::json!("logistics_pickup"));
    return RuleResult {
      label: Some(LabelOutput {
        industry: "通用".to_string(),
        sms_type: "物流取件".to_string(),
        entities: entities.clone(),
        confidence: 0.92,
        needs_review: false,
        reasons: vec!["rule: logistics_pickup".to_string()],
        signals: signals.clone(),
        rules_version: RULES_VERSION.to_string(),
        model_version: "n/a".to_string(),
        schema_version: SCHEMA_VERSION.to_string(),
      }),
      entities,
      signals,
      strong_hit: true,
    };
  }

  if contains_any(content, &["公安", "税务", "社保", "公积金", "政府", "政务", "人民法院", "检察院", "交警", "医保"]) {
    signals.insert("rule".to_string(), serde_json::json!("gov_notice"));
    return RuleResult {
      label: Some(LabelOutput {
        industry: "政务".to_string(),
        sms_type: "政务通知".to_string(),
        entities: entities.clone(),
        confidence: 0.93,
        needs_review: false,
        reasons: vec!["rule: gov_org_keyword".to_string()],
        signals: signals.clone(),
        rules_version: RULES_VERSION.to_string(),
        model_version: "n/a".to_string(),
        schema_version: SCHEMA_VERSION.to_string(),
      }),
      entities,
      signals,
      strong_hit: true,
    };
  }

  if is_financial_transaction_like(content, sender) {
    signals.insert("rule".to_string(), serde_json::json!("financial_transaction"));
    return RuleResult {
      label: Some(LabelOutput {
        industry: "金融".to_string(),
        sms_type: "交易提醒".to_string(),
        entities: entities.clone(),
        confidence: 0.90,
        needs_review: false,
        reasons: vec!["rule: financial_transaction".to_string()],
        signals: signals.clone(),
        rules_version: RULES_VERSION.to_string(),
        model_version: "n/a".to_string(),
        schema_version: SCHEMA_VERSION.to_string(),
      }),
      entities,
      signals,
      strong_hit: true,
    };
  }

  // No strong hit: only return entities+signals; model will decide.
  RuleResult {
    label: None,
    entities,
    signals,
    strong_hit: false,
  }
}

fn extract_entities(content: &str, sender: Option<&str>, signals: &mut HashMap<String, serde_json::Value>) -> Entities {
  let mut out = Entities::default();

  out.brand = extract_brand(content, sender);
  if let Some(b) = out.brand.as_ref() {
    signals.insert("brand".to_string(), serde_json::json!(b));
  }

  out.url = URL_RE
    .find(content)
    .map(|m| m.as_str().to_string());
  if out.url.is_some() {
    signals.insert("has_url".to_string(), serde_json::json!(true));
  }

  out.phone_in_text = PHONE_RE
    .find(content)
    .map(|m| m.as_str().to_string());

  out.verification_code = extract_verification_code(content);
  if out.verification_code.is_some() {
    signals.insert("has_verification_code".to_string(), serde_json::json!(true));
  }

  out.amount = extract_amount(content, &["金额", "支付", "扣款", "消费", "入账", "转入", "转出", "还款", "退款"]);
  if out.amount.is_some() {
    signals.insert("has_amount".to_string(), serde_json::json!(true));
  }

  out.balance = extract_amount(content, &["余额", "可用余额", "账户余额"]);

  out.account_suffix = ACCOUNT_SUFFIX_RE
    .captures(content)
    .and_then(|c| c.get(1))
    .map(|m| m.as_str().to_string());

  out.time_text = extract_time_text(content);

  out
}

fn extract_brand(content: &str, sender: Option<&str>) -> Option<String> {
  if let Some(s) = sender {
    let s = s.trim();
    if !s.is_empty() {
      return Some(s.to_string());
    }
  }

  for kw in ["中国银行", "工商银行", "建设银行", "农业银行", "招商银行", "交通银行", "邮储银行", "平安银行", "兴业银行", "中信银行", "浦发银行", "光大银行", "民生银行", "支付宝", "微信", "京东", "美团", "饿了么", "拼多多", "顺丰", "京东物流"] {
    if content.contains(kw) {
      return Some(kw.to_string());
    }
  }
  None
}

fn extract_verification_code(content: &str) -> Option<String> {
  // Common: "验证码123456" / "验证码：123456" / "code is 123456"
  if let Some(c) = CODE_NEAR_KEYWORD_RE
    .captures(content)
    .and_then(|c| c.get(1))
  {
    return Some(c.as_str().to_string());
  }

  // fallback: first 4-8 digit token if message indicates verification
  if contains_any(content, &["验证码", "校验码", "动态码", "OTP"]) {
    if let Some(m) = DIGITS_RE.find(content) {
      return Some(m.as_str().to_string());
    }
  }
  None
}

fn extract_amount(content: &str, ctx_keywords: &[&str]) -> Option<f64> {
  if !ctx_keywords.iter().any(|k| content.contains(k)) {
    // still allow if explicit currency symbol exists
    if !content.contains('¥') && !content.contains('￥') {
      return None;
    }
  }

  AMOUNT_RE
    .captures(content)
    .and_then(|c| c.get(2))
    .and_then(|m| parse_amount(m.as_str()))
}

fn parse_amount(s: &str) -> Option<f64> {
  let s = s.replace(',', "").replace('，', "");
  s.parse::<f64>().ok()
}

fn extract_time_text(content: &str) -> Option<String> {
  // Keep original time substring if found.
  TIME_RE.find(content).map(|m| m.as_str().to_string())
}

fn guess_industry_from_sender(sender: Option<&str>) -> Option<String> {
  let s = sender?.to_lowercase();
  if contains_any(&s, &["bank", "银行", "证券", "保险"]) {
    return Some("金融".to_string());
  }
  None
}

fn is_financial_transaction_like(content: &str, sender: Option<&str>) -> bool {
  if contains_any(content, &["银行", "证券", "保险", "信用卡", "贷款", "还款", "入账", "扣款", "消费", "交易", "转账", "转入", "转出"]) {
    return true;
  }
  if let Some(s) = sender {
    if contains_any(s, &["银行", "证券", "保险"]) {
      return true;
    }
  }
  false
}

fn contains_any(s: &str, kws: &[&str]) -> bool {
  kws.iter().any(|k| s.contains(k))
}

static URL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"https?://\S+|www\.[^\s]+\.[^\s]+" ).unwrap());
static PHONE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b1\d{10}\b" ).unwrap());
static DIGITS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b\d{4,8}\b" ).unwrap());
static CODE_NEAR_KEYWORD_RE: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"(?:验证码|校验码|动态码|OTP)\D{0,6}(\d{4,8})" ).unwrap()
});
static AMOUNT_RE: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"((￥|¥|RMB|CNY)\s*)(\d+(?:[\.,]\d+)?)" ).unwrap()
});
static ACCOUNT_SUFFIX_RE: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"(?:尾号|末四位|后四位)\D{0,4}(\d{3,6})" ).unwrap()
});
static TIME_RE: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"\b\d{4}[-/.年]\d{1,2}[-/.月]\d{1,2}(?:日)?(?:\s*\d{1,2}:\d{2}(?::\d{2})?)?\b|\b\d{1,2}:\d{2}(?::\d{2})?\b" ).unwrap()
});
