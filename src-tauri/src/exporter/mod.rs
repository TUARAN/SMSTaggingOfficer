use std::{fs::File, io::Write, path::PathBuf};

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::{db::Db, model::schema::LabelOutput};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
  pub only_reviewed: bool,
  pub format: String, // csv/jsonl
}

pub fn execute(db: &Db, path: PathBuf, options: ExportOptions) -> Result<i64, String> {
  let fmt = options.format.to_ascii_lowercase();
  match fmt.as_str() {
    "csv" => export_csv(db, path, options.only_reviewed),
    "jsonl" => export_jsonl(db, path, options.only_reviewed),
    _ => Err("unsupported export format (csv/jsonl)".to_string()),
  }
}

fn export_jsonl(db: &Db, path: PathBuf, only_reviewed: bool) -> Result<i64, String> {
  let mut file = File::create(path).map_err(|e| e.to_string())?;

  let sql = if only_reviewed {
    "SELECT l.reasons_json, l.signals_json, l.entities_json, l.industry, l.sms_type, l.confidence, l.needs_review, l.rules_version, l.model_version, l.schema_version
     FROM labels l WHERE l.needs_review=0 ORDER BY l.message_id ASC"
  } else {
    "SELECT l.reasons_json, l.signals_json, l.entities_json, l.industry, l.sms_type, l.confidence, l.needs_review, l.rules_version, l.model_version, l.schema_version
     FROM labels l ORDER BY l.message_id ASC"
  };

  let conn = db.conn();
  let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
  let mut rows = stmt.query(params![]).map_err(|e| e.to_string())?;

  let mut written = 0i64;
  while let Some(r) = rows.next().map_err(|e| e.to_string())? {
    let reasons_json: String = r.get(0).map_err(|e| e.to_string())?;
    let signals_json: String = r.get(1).map_err(|e| e.to_string())?;
    let entities_json: String = r.get(2).map_err(|e| e.to_string())?;

    let label = LabelOutput {
      industry: r.get(3).map_err(|e| e.to_string())?,
      sms_type: r.get(4).map_err(|e| e.to_string())?,
      confidence: r.get(5).map_err(|e| e.to_string())?,
      needs_review: r.get::<_, i32>(6).map_err(|e| e.to_string())? != 0,
      reasons: serde_json::from_str(&reasons_json).unwrap_or_default(),
      signals: serde_json::from_str(&signals_json).unwrap_or_default(),
      rules_version: r.get(7).map_err(|e| e.to_string())?,
      model_version: r.get(8).map_err(|e| e.to_string())?,
      schema_version: r.get(9).map_err(|e| e.to_string())?,
      entities: serde_json::from_str(&entities_json).unwrap_or_default(),
    };

    let line = serde_json::to_string(&label).map_err(|e| e.to_string())?;
    file.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
    file.write_all(b"\n").map_err(|e| e.to_string())?;
    written += 1;
  }

  Ok(written)
}

fn export_csv(db: &Db, path: PathBuf, only_reviewed: bool) -> Result<i64, String> {
  let mut wtr = csv::Writer::from_path(path).map_err(|e| e.to_string())?;

  wtr
    .write_record([
      "industry",
      "type",
      "confidence",
      "needs_review",
      "brand",
      "verification_code",
      "amount",
      "balance",
      "account_suffix",
      "time_text",
      "url",
      "phone_in_text",
      "rules_version",
      "model_version",
      "schema_version",
      "reasons",
    ])
    .map_err(|e| e.to_string())?;

  let sql = if only_reviewed {
    "SELECT l.industry, l.sms_type, l.confidence, l.needs_review, l.entities_json, l.rules_version, l.model_version, l.schema_version, l.reasons_json
     FROM labels l WHERE l.needs_review=0 ORDER BY l.message_id ASC"
  } else {
    "SELECT l.industry, l.sms_type, l.confidence, l.needs_review, l.entities_json, l.rules_version, l.model_version, l.schema_version, l.reasons_json
     FROM labels l ORDER BY l.message_id ASC"
  };

  let conn = db.conn();
  let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
  let mut rows = stmt.query(params![]).map_err(|e| e.to_string())?;

  let mut written = 0i64;
  while let Some(r) = rows.next().map_err(|e| e.to_string())? {
    let entities_json: String = r.get(4).map_err(|e| e.to_string())?;
    let reasons_json: String = r.get(8).map_err(|e| e.to_string())?;
    let entities: crate::model::schema::Entities = serde_json::from_str(&entities_json).unwrap_or_default();

    wtr
      .write_record([
        r.get::<_, String>(0).unwrap_or_else(|_| "".to_string()),
        r.get::<_, String>(1).unwrap_or_else(|_| "".to_string()),
        format!("{:.4}", r.get::<_, f64>(2).unwrap_or(0.0)),
        (r.get::<_, i32>(3).unwrap_or(1) != 0).to_string(),
        entities.brand.unwrap_or_default(),
        entities.verification_code.unwrap_or_default(),
        entities.amount.map(|v| v.to_string()).unwrap_or_default(),
        entities.balance.map(|v| v.to_string()).unwrap_or_default(),
        entities.account_suffix.unwrap_or_default(),
        entities.time_text.unwrap_or_default(),
        entities.url.unwrap_or_default(),
        entities.phone_in_text.unwrap_or_default(),
        r.get::<_, String>(5).unwrap_or_else(|_| "".to_string()),
        r.get::<_, String>(6).unwrap_or_else(|_| "".to_string()),
        r.get::<_, String>(7).unwrap_or_else(|_| "".to_string()),
        serde_json::from_str::<Vec<String>>(&reasons_json)
          .unwrap_or_default()
          .join(" | "),
      ])
      .map_err(|e| e.to_string())?;
    written += 1;
  }

  wtr.flush().map_err(|e| e.to_string())?;
  Ok(written)
}
