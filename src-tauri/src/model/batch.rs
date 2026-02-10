use std::{
  collections::VecDeque,
  fs::OpenOptions,
  io::Write,
  path::PathBuf,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  thread,
  time::{Duration, Instant},
};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::{
  db::Db,
  model::{
    fusion::{self, FusionInput},
    provider::{self, Provider},
    schema::{ClassifyPayload, LabelOutput},
  },
  rules,
  settings::SettingsStore,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOptions {
  pub mode: String, // all | unlabeled | needs_review
  pub concurrency: usize,
  pub timeout_ms: u64,
  pub max_retries: i32,
}

impl Default for BatchOptions {
  fn default() -> Self {
    Self {
      mode: "unlabeled".to_string(),
      concurrency: 2,
      timeout_ms: 15000,
      max_retries: 1,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProgress {
  pub running: bool,
  pub total: i64,
  pub done: i64,
  pub failed: i64,
  pub current_message_id: Option<i64>,
  pub started_at_ms: Option<i64>,
  pub elapsed_ms: i64,
}

struct Inner {
  progress: BatchProgress,
  stop: Arc<AtomicBool>,
  failed_ids: Vec<i64>,
  pending: VecDeque<i64>,
}

pub struct BatchManager {
  inner: Mutex<Inner>,
  db: Arc<Db>,
  settings: Arc<SettingsStore>,
  log_dir: PathBuf,
}

impl BatchManager {
  pub fn new(db: Arc<Db>, settings: Arc<SettingsStore>, log_dir: PathBuf) -> Self {
    Self {
      inner: Mutex::new(Inner {
        progress: BatchProgress {
          running: false,
          total: 0,
          done: 0,
          failed: 0,
          current_message_id: None,
          started_at_ms: None,
          elapsed_ms: 0,
        },
        stop: Arc::new(AtomicBool::new(false)),
        failed_ids: vec![],
        pending: VecDeque::new(),
      }),
      db,
      settings,
      log_dir,
    }
  }

  pub fn status(&self) -> BatchProgress {
    self.inner.lock().progress.clone()
  }

  pub fn stop(&self) {
    self.inner.lock().stop.store(true, Ordering::SeqCst);
  }

  pub fn retry_failed(&self) -> Result<(), String> {
    let mut inner = self.inner.lock();
    if inner.progress.running {
      return Err("batch is running".to_string());
    }
    let ids = std::mem::take(&mut inner.failed_ids);
    inner.progress.failed = 0;
    inner.pending = ids.into();
    Ok(())
  }

  pub fn start(self: &Arc<Self>, options: BatchOptions, app: AppHandle) -> Result<(), String> {
    {
      let mut inner = self.inner.lock();
      if inner.progress.running {
        return Err("batch already running".to_string());
      }
      inner.stop.store(false, Ordering::SeqCst);
      inner.failed_ids.clear();
      inner.pending.clear();

      let ids = self.db.dao().fetch_batch_candidates(&options.mode, 100000)?;
      inner.pending = ids.into();
      inner.progress.total = inner.pending.len() as i64;
      inner.progress.done = 0;
      inner.progress.failed = 0;
      inner.progress.current_message_id = None;
      inner.progress.running = true;
      inner.progress.started_at_ms = Some(now_ms());
      inner.progress.elapsed_ms = 0;
    }

    let mgr = Arc::clone(self);
    thread::spawn(move || {
      mgr.run_loop(options, app);
    });

    Ok(())
  }

  fn run_loop(self: Arc<Self>, options: BatchOptions, app: AppHandle) {
    let stop = { self.inner.lock().stop.clone() };
    let started = Instant::now();

    let (tx_job, rx_job) = std::sync::mpsc::channel::<i64>();
    let rx_job = Arc::new(Mutex::new(rx_job));
    let (tx_res, rx_res) = std::sync::mpsc::channel::<(i64, Result<(), String>)>();

    let worker_n = options.concurrency.clamp(1, 8);
    let timeout = Duration::from_millis(options.timeout_ms.max(1000));
    let max_retries = options.max_retries.max(0);

    // Snapshot provider (per worker) from settings at start.
    let settings_snapshot = self.settings.get().clone();

    for _ in 0..worker_n {
      let rx_job = rx_job.clone();
      let tx_res = tx_res.clone();
      let db = self.db.clone();
      let log_dir = self.log_dir.clone();
      let stop2 = stop.clone();
      let provider_res = provider::build_provider(&settings_snapshot);

      thread::spawn(move || {
        let provider = match provider_res {
          Ok(p) => Some(p),
          Err(e) => {
            let _ = append_log(&log_dir, &format!("provider build failed: {e}"));
            None
          }
        };

        loop {
          let id = {
            let guard = rx_job.lock();
            match guard.recv() {
              Ok(v) => v,
              Err(_) => break,
            }
          };

          if stop2.load(Ordering::SeqCst) {
            let _ = tx_res.send((id, Err("stopped".to_string())));
            continue;
          }

          let res = process_one(&db, provider.as_deref(), &log_dir, id, timeout, max_retries);
          let _ = tx_res.send((id, res));
        }
      });
    }

    // Feed jobs
    loop {
      if stop.load(Ordering::SeqCst) {
        break;
      }

      let next_id = {
        let mut inner = self.inner.lock();
        inner.progress.elapsed_ms = started.elapsed().as_millis() as i64;
        inner.pending.pop_front()
      };

      if let Some(id) = next_id {
        {
          let mut inner = self.inner.lock();
          inner.progress.current_message_id = Some(id);
        }
        if tx_job.send(id).is_err() {
          break;
        }
      } else {
        break;
      }

      while let Ok((id, r)) = rx_res.try_recv() {
        self.on_one_done(id, r);
      }

      self.emit_progress(&app);
      thread::sleep(Duration::from_millis(4));
    }

    drop(tx_job);

    // Drain remaining results (best-effort)
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
      match rx_res.try_recv() {
        Ok((id, r)) => self.on_one_done(id, r),
        Err(_) => break,
      }
    }

    {
      let mut inner = self.inner.lock();
      inner.progress.running = false;
      inner.progress.current_message_id = None;
      inner.progress.elapsed_ms = started.elapsed().as_millis() as i64;
    }

    self.emit_progress(&app);
  }

  fn on_one_done(&self, id: i64, r: Result<(), String>) {
    let mut inner = self.inner.lock();
    match r {
      Ok(_) => {
        inner.progress.done += 1;
      }
      Err(e) => {
        if e != "stopped" {
          inner.progress.done += 1;
          inner.progress.failed += 1;
          inner.failed_ids.push(id);
        }
      }
    }
  }

  fn emit_progress(&self, app: &AppHandle) {
    let _ = app.emit_all("batch_progress", self.status());
  }
}

fn process_one(
  db: &Db,
  provider: Option<&dyn Provider>,
  log_dir: &PathBuf,
  message_id: i64,
  timeout: Duration,
  max_retries: i32,
) -> Result<(), String> {
  let content = db.dao().get_message_content(message_id)?;

  let rule = rules::run_rules(&content, None);

  let payload = ClassifyPayload {
    message_id,
    content: content.clone(),
    entities: rule.entities.clone(),
    signals: rule.signals.clone(),
  };

  let rule_label = rule.label;

  let model_label: Option<LabelOutput> = if rule.strong_hit {
    None
  } else {
    let provider = provider.ok_or_else(|| "provider unavailable".to_string());
    let provider = match provider {
      Ok(p) => p,
      Err(e) => {
        let fallback = LabelOutput::error_fallback(rule.entities.clone(), rule.signals.clone(), &e);
        let _ = db.dao().upsert_label_auto(message_id, &fallback);
        let _ = append_log(log_dir, &format!("message_id={message_id} provider unavailable: {e}"));
        return Err(e);
      }
    };

    let mut got: Option<LabelOutput> = None;
    let mut last_err: Option<String> = None;

    for attempt in 0..=max_retries {
      match provider.classify(&payload, timeout) {
        Ok(v) => {
          got = Some(v);
          last_err = None;
          break;
        }
        Err(e) => {
          last_err = Some(e);
          if attempt < max_retries {
            thread::sleep(Duration::from_millis(120));
          }
        }
      }
    }

    if got.is_none() {
      let e = last_err.unwrap_or_else(|| "unknown provider error".to_string());
      let fallback = LabelOutput::error_fallback(rule.entities.clone(), rule.signals.clone(), &e);
      let _ = db.dao().upsert_label_auto(message_id, &fallback);
      let _ = append_log(log_dir, &format!("message_id={message_id} classify failed: {e}"));
      return Err(e);
    }

    got
  };

  let fused = fusion::fuse(FusionInput {
    rule: rule_label,
    model: model_label,
    rule_strong_hit: rule.strong_hit,
  });

  db.dao().upsert_label_auto(message_id, &fused)?;
  Ok(())
}

fn append_log(log_dir: &PathBuf, line: &str) -> Result<(), String> {
  std::fs::create_dir_all(log_dir).map_err(|e| e.to_string())?;
  let path = log_dir.join("batch_errors.log");
  let mut f = OpenOptions::new()
    .create(true)
    .append(true)
    .open(path)
    .map_err(|e| e.to_string())?;
  writeln!(f, "{}", line).map_err(|e| e.to_string())
}

fn now_ms() -> i64 {
  use std::time::{SystemTime, UNIX_EPOCH};
  let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
  dur.as_millis() as i64
}
