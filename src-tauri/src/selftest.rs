use std::{
  fs,
  path::PathBuf,
  sync::Arc,
  time::{Duration, SystemTime, UNIX_EPOCH},
};

use parking_lot::Mutex;

use crate::{
  db::Db,
  exporter::{self, ExportOptions},
  model::{
    fusion::{self, FusionInput},
    provider::{MockProvider, Provider},
    schema::ClassifyPayload,
  },
  rules,
  status::SelftestStatus,
};

#[derive(Debug, Clone)]
pub struct SelftestRunResult {
  pub inserted: i64,
  pub labeled: i64,
  pub written_jsonl: i64,
  pub written_csv: i64,
  pub db_path: PathBuf,
  pub jsonl_path: PathBuf,
  pub csv_path: PathBuf,
}

pub struct SelftestRunner {
  status: Mutex<SelftestStatus>,
}

impl SelftestRunner {
  pub fn new() -> Self {
    Self {
      status: Mutex::new(SelftestStatus {
        running: false,
        ok: None,
        message: "idle".to_string(),
        started_at_ms: None,
        finished_at_ms: None,
        out_dir: None,
      }),
    }
  }

  pub fn snapshot(&self) -> SelftestStatus {
    self.status.lock().clone()
  }

  pub fn start(self: &Arc<Self>, out_dir: PathBuf) -> Result<(), String> {
    {
      let mut s = self.status.lock();
      if s.running {
        return Err("selftest already running".to_string());
      }
      s.running = true;
      s.ok = None;
      s.message = "running".to_string();
      s.started_at_ms = Some(now_ms());
      s.finished_at_ms = None;
      s.out_dir = Some(out_dir.display().to_string());
    }

    let runner = Arc::clone(self);
    std::thread::spawn(move || {
      let result = run(out_dir);
      let mut s = runner.status.lock();
      s.running = false;
      s.finished_at_ms = Some(now_ms());
      match result {
        Ok(r) => {
          s.ok = Some(true);
          s.message = format!(
            "ok: inserted={}, labeled={}, jsonl={}, csv={}",
            r.inserted, r.labeled, r.written_jsonl, r.written_csv
          );
        }
        Err(e) => {
          s.ok = Some(false);
          s.message = format!("failed: {e}");
        }
      }
    });

    Ok(())
  }
}

pub fn run(out_dir: PathBuf) -> Result<SelftestRunResult, String> {
  fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

  let db_path = out_dir.join("selftest.sqlite3");
  if db_path.exists() {
    let _ = fs::remove_file(&db_path);
  }

  let db = Db::open(db_path.clone())?;
  db.migrate()?;

  // Fully offline: generate a few deterministic sample messages.
  let samples: [&str; 6] = [
    "【银行】您尾号1234的信用卡本期账单已出，最低还款200元，点击查看。",
    "验证码：839204（5分钟内有效），请勿泄露。",
    "【快递】您的包裹已到驿站，请凭取件码A1234领取。",
    "【外卖】骑手已到达，请保持电话畅通。",
    "【政务】您有一条新的政务通知，请登录查看。",
    "【电商】订单已发货，预计明天送达，感谢您的购买。",
  ];

  let mut inserted = 0i64;
  for content in samples {
    db.dao().insert_message(content, None, None, None, Some("selftest"))?;
    inserted += 1;
  }

  let ids = db
    .dao()
    .fetch_batch_candidates("all", 100000, None, None)?;
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

  Ok(SelftestRunResult {
    inserted,
    labeled,
    written_jsonl,
    written_csv,
    db_path,
    jsonl_path,
    csv_path,
  })
}

fn now_ms() -> i64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis() as i64
}
