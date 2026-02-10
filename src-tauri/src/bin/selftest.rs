use std::{fs, path::PathBuf, time::Duration};

use sms_tagging_officer::{
  db::Db,
  exporter::{self, ExportOptions},
  importer::{self, ColumnMapping},
  model::{
    fusion::{self, FusionInput},
    provider::{MockProvider, Provider},
    schema::ClassifyPayload,
  },
  rules,
};

fn main() -> Result<(), String> {
  let out_dir = PathBuf::from("tools");
  fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

  let db_path = out_dir.join("selftest.sqlite3");
  if db_path.exists() {
    let _ = fs::remove_file(&db_path);
  }

  let db = Db::open(db_path.clone())?;
  db.migrate()?;

  let sample_path = PathBuf::from("samples").join("sms_samples.csv");
  if !sample_path.exists() {
    return Err(format!("sample file not found: {}", sample_path.display()));
  }

  let inserted = importer::execute(
    &db,
    sample_path,
    ColumnMapping {
      content: "content".to_string(),
      received_at: Some("received_at".to_string()),
      sender: Some("sender".to_string()),
      phone: Some("phone".to_string()),
      source: Some("source".to_string()),
    },
  )?;

  let ids = db.dao().fetch_batch_candidates("all", 100000)?;
  let provider = MockProvider;

  let mut labeled = 0i64;
  for id in ids {
    let content = db.dao().get_message_content(id)?;

    let rule = rules::run_rules(&content, None);
    let payload = ClassifyPayload {
      message_id: id,
      content: content.clone(),
      entities: rule.entities.clone(),
      signals: rule.signals.clone(),
    };

    let model_label = if rule.strong_hit {
      None
    } else {
      Some(provider.classify(&payload, Duration::from_secs(2))?)
    };

    let fused = fusion::fuse(FusionInput {
      rule: rule.label,
      model: model_label,
      rule_strong_hit: rule.strong_hit,
    });

    db.dao().upsert_label_auto(id, &fused.normalize())?;
    labeled += 1;
  }

  let jsonl_path = out_dir.join("selftest_export.jsonl");
  let csv_path = out_dir.join("selftest_export.csv");

  let written_jsonl = exporter::execute(
    &db,
    jsonl_path.clone(),
    ExportOptions {
      only_reviewed: false,
      format: "jsonl".to_string(),
    },
  )?;

  let written_csv = exporter::execute(
    &db,
    csv_path.clone(),
    ExportOptions {
      only_reviewed: false,
      format: "csv".to_string(),
    },
  )?;

  println!("[selftest] db: {}", db_path.display());
  println!("[selftest] inserted messages: {}", inserted);
  println!("[selftest] labeled messages: {}", labeled);
  println!("[selftest] exported jsonl lines: {} -> {}", written_jsonl, jsonl_path.display());
  println!("[selftest] exported csv rows: {} -> {}", written_csv, csv_path.display());

  Ok(())
}
