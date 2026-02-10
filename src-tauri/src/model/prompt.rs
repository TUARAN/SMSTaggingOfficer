use crate::model::schema::{ClassifyPayload, INDUSTRIES, SMS_TYPES, RULES_VERSION, SCHEMA_VERSION};

pub fn build_prompt(payload: &ClassifyPayload) -> String {
  // Strict JSON-only instruction.
  // Model must output ONLY a JSON object. No extra text.
  let industry_list = INDUSTRIES.join("、");
  let type_list = SMS_TYPES.join("、");

  let entities_json = serde_json::to_string(&payload.entities).unwrap_or_else(|_| "{}".to_string());
  let signals_json = serde_json::to_string(&payload.signals).unwrap_or_else(|_| "{}".to_string());

  format!(
    r#"你是“短信智标官”的离线分类与抽取模型。
你必须严格输出 JSON（只输出一个 JSON 对象，禁止输出其他任何字符）。

任务：对短信 content 做两层标签与实体抽取补全。

约束：
- industry 只能取以下枚举之一：{industry_list}
- type 只能取以下枚举之一：{type_list}
- entities 必须包含字段：brand, verification_code, amount, balance, account_suffix, time_text, url, phone_in_text；缺失填 null
- confidence 为 0~1 的小数
- reasons 为字符串数组（简短、可解释）
- needs_review 为 true/false
- rules_version 固定为 {rules_version}
- schema_version 固定为 {schema_version}
- model_version 你可以填 "llama"（实际版本由应用覆盖）

输入：
content: {content}
rule_entities: {entities_json}
rule_signals: {signals_json}

输出 JSON schema（示例结构，不要照抄示例值）：
{{
  "industry": "其他",
  "type": "其他",
  "entities": {{
    "brand": null,
    "verification_code": null,
    "amount": null,
    "balance": null,
    "account_suffix": null,
    "time_text": null,
    "url": null,
    "phone_in_text": null
  }},
  "confidence": 0.5,
  "needs_review": true,
  "reasons": ["..."],
  "signals": {{}},
  "rules_version": "{rules_version}",
  "model_version": "llama",
  "schema_version": "{schema_version}"
}}
"#,
    industry_list = industry_list,
    type_list = type_list,
    rules_version = RULES_VERSION,
    schema_version = SCHEMA_VERSION,
    content = json_escape(&payload.content),
    entities_json = entities_json,
    signals_json = signals_json
  )
}

fn json_escape(s: &str) -> String {
  // Keep prompt robust for quotes/newlines.
  serde_json::to_string(s).unwrap_or_else(|_| format!("\"{}\"", s.replace('"', "\\\"")))
}

pub fn extract_json(text: &str) -> Option<String> {
  // Try to locate the first JSON object in output.
  let start = text.find('{')?;
  let mut depth = 0i32;
  for (i, ch) in text[start..].char_indices() {
    match ch {
      '{' => depth += 1,
      '}' => {
        depth -= 1;
        if depth == 0 {
          let end = start + i + 1;
          return Some(text[start..end].trim().to_string());
        }
      }
      _ => {}
    }
  }
  None
}
