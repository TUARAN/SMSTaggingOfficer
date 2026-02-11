use std::collections::HashMap;

use regex::Regex;
use rusqlite::{params, params_from_iter, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::model::schema::{LabelOutput, MessageRow};

use super::Db;

pub struct Dao<'a> {
  db: &'a Db,
}

impl<'a> Dao<'a> {
  pub fn new(db: &'a Db) -> Self {
    Self { db }
  }

  pub fn insert_message(
    &self,
    content: &str,
    received_at: Option<&str>,
    sender: Option<&str>,
    phone: Option<&str>,
    source: Option<&str>,
  ) -> Result<i64, String> {
    let (has_url, has_amount, has_verification_code) = compute_flags(content);

    let conn = self.db.conn();
    conn
      .execute(
        "INSERT INTO messages(content, received_at, sender, phone, source, has_url, has_amount, has_verification_code) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
          content,
          received_at,
          sender,
          phone,
          source,
          has_url as i32,
          has_amount as i32,
          has_verification_code as i32
        ],
      )
      .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
  }

  pub fn messages_meta(&self) -> Result<(i64, i64), String> {
    let conn = self.db.conn();
    let (count, max_id): (i64, i64) = conn
      .query_row(
        "SELECT COUNT(1) as cnt, COALESCE(MAX(id), 0) as max_id FROM messages",
        params![],
        |r| Ok((r.get(0)?, r.get(1)?)),
      )
      .map_err(|e| e.to_string())?;
    Ok((count, max_id))
  }

  pub fn get_message_content(&self, message_id: i64) -> Result<String, String> {
    let conn = self.db.conn();
    let content: String = conn
      .query_row(
        "SELECT content FROM messages WHERE id=?1",
        params![message_id],
        |r| r.get(0),
      )
      .map_err(|e| e.to_string())?;
    Ok(content)
  }

  pub fn get_label(&self, message_id: i64) -> Result<Option<LabelOutput>, String> {
    let conn = self.db.conn();
    let row = conn
      .query_row(
        "SELECT industry, sms_type, confidence, needs_review, reasons_json, signals_json, rules_version, model_version, schema_version, entities_json FROM labels WHERE message_id=?1",
        params![message_id],
        |r| {
          let reasons_json: String = r.get(4)?;
          let signals_json: String = r.get(5)?;
          let entities_json: String = r.get(9)?;
          Ok(LabelOutput {
            industry: r.get(0)?,
            sms_type: r.get(1)?,
            confidence: r.get(2)?,
            needs_review: (r.get::<_, i32>(3)? != 0),
            reasons: serde_json::from_str(&reasons_json).unwrap_or_default(),
            signals: serde_json::from_str(&signals_json).unwrap_or_default(),
            rules_version: r.get(6)?,
            model_version: r.get(7)?,
            schema_version: r.get(8)?,
            entities: serde_json::from_str(&entities_json).unwrap_or_default(),
          })
        },
      )
      .optional()
      .map_err(|e| e.to_string())?;
    Ok(row)
  }

  pub fn upsert_label_auto(&self, message_id: i64, label: &LabelOutput) -> Result<(), String> {
    let conn = self.db.conn();
    let reasons_json = serde_json::to_string(&label.reasons).map_err(|e| e.to_string())?;
    let signals_json = serde_json::to_string(&label.signals).map_err(|e| e.to_string())?;
    let entities_json = serde_json::to_string(&label.entities).map_err(|e| e.to_string())?;

    conn
      .execute(
        "INSERT INTO labels(message_id, industry, sms_type, confidence, needs_review, reasons_json, signals_json, rules_version, model_version, schema_version, entities_json, updated_by, is_manual)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,'system',0)
         ON CONFLICT(message_id) DO UPDATE SET
           industry=excluded.industry,
           sms_type=excluded.sms_type,
           confidence=excluded.confidence,
           needs_review=excluded.needs_review,
           reasons_json=excluded.reasons_json,
           signals_json=excluded.signals_json,
           rules_version=excluded.rules_version,
           model_version=excluded.model_version,
           schema_version=excluded.schema_version,
           entities_json=excluded.entities_json,
           updated_by='system',
           updated_at=(strftime('%Y-%m-%dT%H:%M:%fZ','now')),
           is_manual=0",
        params![
          message_id,
          label.industry,
          label.sms_type,
          label.confidence,
          if label.needs_review { 1 } else { 0 },
          reasons_json,
          signals_json,
          label.rules_version,
          label.model_version,
          label.schema_version,
          entities_json
        ],
      )
      .map_err(|e| e.to_string())?;
    Ok(())
  }

  pub fn label_update_manual(
    &self,
    message_id: i64,
    operator: &str,
    new_label: LabelOutput,
  ) -> Result<(), String> {
    let before = self.get_label(message_id)?;

    let reasons_json = serde_json::to_string(&new_label.reasons).map_err(|e| e.to_string())?;
    let signals_json = serde_json::to_string(&new_label.signals).map_err(|e| e.to_string())?;
    let entities_json = serde_json::to_string(&new_label.entities).map_err(|e| e.to_string())?;

    let conn = self.db.conn();
    conn
      .execute(
        "INSERT INTO labels(message_id, industry, sms_type, confidence, needs_review, reasons_json, signals_json, rules_version, model_version, schema_version, entities_json, updated_by, is_manual)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,1)
         ON CONFLICT(message_id) DO UPDATE SET
           industry=excluded.industry,
           sms_type=excluded.sms_type,
           confidence=excluded.confidence,
           needs_review=excluded.needs_review,
           reasons_json=excluded.reasons_json,
           signals_json=excluded.signals_json,
           rules_version=excluded.rules_version,
           model_version=excluded.model_version,
           schema_version=excluded.schema_version,
           entities_json=excluded.entities_json,
           updated_by=excluded.updated_by,
           updated_at=(strftime('%Y-%m-%dT%H:%M:%fZ','now')),
           is_manual=1",
        params![
          message_id,
          new_label.industry,
          new_label.sms_type,
          new_label.confidence,
          if new_label.needs_review { 1 } else { 0 },
          reasons_json,
          signals_json,
          new_label.rules_version,
          new_label.model_version,
          new_label.schema_version,
          entities_json,
          operator
        ],
      )
      .map_err(|e| e.to_string())?;

    let before_json = before
      .as_ref()
      .map(|b| serde_json::to_string(b).unwrap_or_else(|_| "null".to_string()));
    let after_json = serde_json::to_string(&new_label).map_err(|e| e.to_string())?;
    let diff_json = compute_diff(before.as_ref(), &new_label);

    conn
      .execute(
        "INSERT INTO audit_logs(message_id, operator, before_json, after_json, diff_json) VALUES (?1,?2,?3,?4,?5)",
        params![message_id, operator, before_json, after_json, diff_json],
      )
      .map_err(|e| e.to_string())?;

    Ok(())
  }

  pub fn messages_list(&self, query: ListQuery) -> Result<ListResult, String> {
    let mut where_sql: Vec<String> = vec![];
    let mut args: Vec<rusqlite::types::Value> = vec![];

    if let Some(industry) = query.industry.clone().flatten() {
      where_sql.push("l.industry = ?".to_string());
      args.push(industry.into());
    }
    if let Some(sms_type) = query.sms_type.clone().flatten() {
      where_sql.push("l.sms_type = ?".to_string());
      args.push(sms_type.into());
    }
    if let Some(needs_review) = query.needs_review {
      where_sql.push("l.needs_review = ?".to_string());
      args.push((if needs_review { 1 } else { 0 }).into());
    }
    if let Some(conf_min) = query.conf_min {
      where_sql.push("l.confidence >= ?".to_string());
      args.push(conf_min.into());
    }
    if let Some(conf_max) = query.conf_max {
      where_sql.push("l.confidence <= ?".to_string());
      args.push(conf_max.into());
    }
    if let Some(has_url) = query.has_url {
      where_sql.push("m.has_url = ?".to_string());
      args.push((if has_url { 1 } else { 0 }).into());
    }
    if let Some(has_verification_code) = query.has_verification_code {
      where_sql.push("m.has_verification_code = ?".to_string());
      args.push((if has_verification_code { 1 } else { 0 }).into());
    }
    if let Some(has_amount) = query.has_amount {
      where_sql.push("m.has_amount = ?".to_string());
      args.push((if has_amount { 1 } else { 0 }).into());
    }
    if let Some(q) = query.q.clone().flatten() {
      where_sql.push("(m.content LIKE ? OR m.sender LIKE ? OR m.source LIKE ?)".to_string());
      let like = format!("%{}%", q);
      args.push(like.clone().into());
      args.push(like.clone().into());
      args.push(like.into());
    }

    let where_clause = if where_sql.is_empty() {
      "".to_string()
    } else {
      format!("WHERE {}", where_sql.join(" AND "))
    };

    let conn = self.db.conn();

    let total_sql = format!(
      "SELECT COUNT(1) FROM messages m LEFT JOIN labels l ON l.message_id=m.id {where_clause}"
    );
    let mut total_stmt = conn.prepare(&total_sql).map_err(|e| e.to_string())?;
    let total: i64 = total_stmt
      .query_row(params_from_iter(args.clone()), |r| r.get(0))
      .map_err(|e| e.to_string())?;

    let list_sql = format!(
      "SELECT m.id, m.content, m.received_at, m.sender, m.phone, m.source, m.has_url, m.has_amount, m.has_verification_code,
              l.industry, l.sms_type, l.confidence, l.needs_review, l.reasons_json, l.signals_json, l.rules_version, l.model_version, l.schema_version, l.entities_json
       FROM messages m
       LEFT JOIN labels l ON l.message_id=m.id
       {where_clause}
       ORDER BY m.id DESC
       LIMIT ? OFFSET ?"
    );

    let mut args2 = args.clone();
    args2.push(query.limit.into());
    args2.push(query.offset.into());

    let mut stmt = conn.prepare(&list_sql).map_err(|e| e.to_string())?;
    let mut rows_iter = stmt
      .query(params_from_iter(args2))
      .map_err(|e| e.to_string())?;

    let mut rows: Vec<MessageRow> = vec![];
    while let Some(r) = rows_iter.next().map_err(|e| e.to_string())? {
      let industry_opt: Option<String> = r.get::<_, Option<String>>(9).map_err(|e| e.to_string())?;
      let label_opt: Option<LabelOutput> = industry_opt.map(|industry| {
        let reasons_json: String = r.get(13).unwrap_or_else(|_| "[]".to_string());
        let signals_json: String = r.get(14).unwrap_or_else(|_| "{}".to_string());
        let entities_json: String = r.get(18).unwrap_or_else(|_| "{}".to_string());
        LabelOutput {
          industry,
          sms_type: r.get(10).unwrap_or_else(|_| "其他".to_string()),
          confidence: r.get(11).unwrap_or(0.0),
          needs_review: r.get::<_, Option<i32>>(12).unwrap_or(Some(1)).unwrap_or(1) != 0,
          reasons: serde_json::from_str(&reasons_json).unwrap_or_default(),
          signals: serde_json::from_str(&signals_json).unwrap_or_default(),
          rules_version: r.get(15).unwrap_or_else(|_| "rules_v1".to_string()),
          model_version: r.get(16).unwrap_or_else(|_| "n/a".to_string()),
          schema_version: r.get(17).unwrap_or_else(|_| "schema_v1".to_string()),
          entities: serde_json::from_str(&entities_json).unwrap_or_default(),
        }
      });

      rows.push(MessageRow {
        id: r.get(0).map_err(|e| e.to_string())?,
        content: r.get(1).map_err(|e| e.to_string())?,
        received_at: r.get(2).ok(),
        sender: r.get(3).ok(),
        phone: r.get(4).ok(),
        source: r.get(5).ok(),
        has_url: r.get::<_, i32>(6).unwrap_or(0) != 0,
        has_amount: r.get::<_, i32>(7).unwrap_or(0) != 0,
        has_verification_code: r.get::<_, i32>(8).unwrap_or(0) != 0,
        label: label_opt,
      });
    }

    Ok(ListResult { total, rows })
  }

  pub fn fetch_batch_candidates(
    &self,
    mode: &str,
    limit: i64,
    id_min: Option<i64>,
    id_max: Option<i64>,
  ) -> Result<Vec<i64>, String> {
    let mut where_sql: Vec<String> = vec![];
    let mut args: Vec<rusqlite::types::Value> = vec![];

    match mode {
      "all" => {}
      "unlabeled" => where_sql.push("l.message_id IS NULL".to_string()),
      "needs_review" => where_sql.push("l.needs_review=1".to_string()),
      _ => where_sql.push("l.message_id IS NULL".to_string()),
    };

    if let Some(v) = id_min {
      where_sql.push("m.id >= ?".to_string());
      args.push(v.into());
    }
    if let Some(v) = id_max {
      where_sql.push("m.id <= ?".to_string());
      args.push(v.into());
    }

    let where_clause = if where_sql.is_empty() {
      "".to_string()
    } else {
      format!("WHERE {}", where_sql.join(" AND "))
    };

    let sql = format!(
      "SELECT m.id FROM messages m LEFT JOIN labels l ON l.message_id=m.id {where_clause} ORDER BY m.id ASC LIMIT ?"
    );

    args.push(limit.into());

    let conn = self.db.conn();
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let mut rows = stmt
      .query(params_from_iter(args))
      .map_err(|e| e.to_string())?;
    let mut ids = vec![];
    while let Some(r) = rows.next().map_err(|e| e.to_string())? {
      ids.push(r.get::<_, i64>(0).map_err(|e| e.to_string())?);
    }
    Ok(ids)
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListQuery {
  pub industry: Option<Option<String>>,
  pub sms_type: Option<Option<String>>,
  pub needs_review: Option<bool>,
  pub conf_min: Option<f64>,
  pub conf_max: Option<f64>,
  pub has_url: Option<bool>,
  pub has_verification_code: Option<bool>,
  pub has_amount: Option<bool>,
  pub q: Option<Option<String>>,
  pub limit: i64,
  pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResult {
  pub total: i64,
  pub rows: Vec<MessageRow>,
}

fn compute_flags(content: &str) -> (bool, bool, bool) {
  let url_re = Regex::new(r"https?://\S+|www\.[^\s]+\.[^\s]+" ).unwrap();
  let amount_re = Regex::new(r"(￥|¥|RMB|CNY)\s*\d+(?:[\.,]\d+)?|\d+(?:[\.,]\d+)?\s*(元|块|人民币)" ).unwrap();
  let code_re = Regex::new(r"\b\d{4,8}\b" ).unwrap();

  let has_url = url_re.is_match(content);
  let has_amount = amount_re.is_match(content);
  let has_code = code_re.is_match(content) && (content.contains("验证码") || content.contains("校验码") || content.contains("动态码") || content.contains("OTP"));

  (has_url, has_amount, has_code)
}

fn compute_diff(before: Option<&LabelOutput>, after: &LabelOutput) -> String {
  let mut diff: HashMap<String, serde_json::Value> = HashMap::new();
  diff.insert(
    "industry".to_string(),
    serde_json::json!({"before": before.map(|b| b.industry.clone()), "after": after.industry}),
  );
  diff.insert(
    "sms_type".to_string(),
    serde_json::json!({"before": before.map(|b| b.sms_type.clone()), "after": after.sms_type}),
  );
  diff.insert(
    "needs_review".to_string(),
    serde_json::json!({"before": before.map(|b| b.needs_review), "after": after.needs_review}),
  );
  diff.insert(
    "confidence".to_string(),
    serde_json::json!({"before": before.map(|b| b.confidence), "after": after.confidence}),
  );
  diff.insert(
    "entities".to_string(),
    serde_json::json!({"before": before.map(|b| &b.entities), "after": &after.entities}),
  );
  serde_json::to_string(&diff).unwrap_or_else(|_| "{}".to_string())
}
