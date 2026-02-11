use std::{path::PathBuf, sync::Arc};

use tauri::{AppHandle, Manager, State};

use crate::{
  db::Db,
  exporter,
  importer,
  model::batch::{BatchManager, BatchOptions, BatchProgress},
  model::provider::ProviderHealth,
  selftest::SelftestRunner,
  status::{DbStatus, ProviderInfo, StatusSnapshot},
  settings::{AppSettings, SettingsStore},
};

#[derive(Clone)]
pub struct AppState {
  pub db: Arc<Db>,
  pub settings: Arc<SettingsStore>,
  pub batch: Arc<BatchManager>,
  pub selftest: Arc<SelftestRunner>,
}

pub fn run() {
  env_logger::init();

  tauri::Builder::default()
    .setup(|app| -> Result<(), Box<dyn std::error::Error>> {
      let app_data_dir = app
        .path_resolver()
        .app_data_dir()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "failed to resolve app_data_dir"))?;
      std::fs::create_dir_all(&app_data_dir)?;

      let db_file = if cfg!(debug_assertions) {
        "smsto.dev.sqlite3"
      } else {
        "smsto.sqlite3"
      };
      let db_path = app_data_dir.join(db_file);
      let log_dir = app_data_dir.join("logs");
      std::fs::create_dir_all(&log_dir)?;

      let db = Arc::new(
        Db::open(db_path)
          .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
      );
      db.migrate()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

      let settings = Arc::new(
        SettingsStore::load(app_data_dir.join("settings.json"))
          .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
      );

      let batch = Arc::new(BatchManager::new(db.clone(), settings.clone(), log_dir));

      let selftest = Arc::new(SelftestRunner::new());

      app.manage(AppState {
        db,
        settings,
        batch,
        selftest,
      });
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      // status
      status_snapshot,
      selftest_run,
      // settings + provider
      settings_get,
      settings_set,
      provider_health_check,
      // import/export
      import_preview,
      import_execute,
      export_execute,
      // list/filter
      messages_list,
      messages_meta,
      // manual review
      label_update_manual,
      // batch
      batch_start,
      batch_stop,
      batch_status,
      batch_retry_failed,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> Result<AppSettings, String> {
  Ok(state.settings.get().clone())
}

#[tauri::command]
pub fn settings_set(state: State<'_, AppState>, settings: AppSettings) -> Result<(), String> {
  state.settings.set(settings).map_err(to_string_err)
}

#[tauri::command]
pub fn provider_health_check(state: State<'_, AppState>) -> Result<ProviderHealth, String> {
  let settings = state.settings.get().clone();
  crate::model::provider::health_check(&settings).map_err(to_string_err)
}

#[tauri::command]
pub fn status_snapshot(state: State<'_, AppState>) -> Result<StatusSnapshot, String> {
  let settings = state.settings.get().clone();

  let db_status = match state.db.ping() {
    Ok(_) => DbStatus {
      ok: true,
      path: Some(state.db.path().display().to_string()),
      message: "connected".to_string(),
    },
    Err(e) => DbStatus {
      ok: false,
      path: Some(state.db.path().display().to_string()),
      message: e,
    },
  };

  let provider_health = crate::model::provider::health_check(&settings).map_err(to_string_err)?;

  let provider = ProviderInfo {
    kind: settings.provider.kind,
    model_path: settings.provider.model_path,
    llama_cli_path: settings.provider.llama_cli_path,
    ollama_base_url: settings.provider.ollama_base_url,
    ollama_model: settings.provider.ollama_model,
    temperature: settings.provider.temperature,
    max_tokens: settings.provider.max_tokens,
  };

  Ok(StatusSnapshot {
    db: db_status,
    provider_health,
    provider,
    batch: Some(state.batch.status()),
    selftest: state.selftest.snapshot(),
  })
}

#[tauri::command]
pub fn selftest_run(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
  let app_data_dir = app
    .path_resolver()
    .app_data_dir()
    .ok_or_else(|| "failed to resolve app_data_dir".to_string())?;
  let out_dir = app_data_dir.join("selftest");
  state.selftest.start(out_dir)
}

#[tauri::command]
pub fn import_preview(_state: State<'_, AppState>, path: String) -> Result<importer::ImportPreview, String> {
  importer::preview(PathBuf::from(path), 20).map_err(to_string_err)
}

#[tauri::command]
pub fn import_execute(
  state: State<'_, AppState>,
  path: String,
  mapping: importer::ColumnMapping,
) -> Result<importer::ImportExecuteResult, String> {
  importer::execute(&state.db, PathBuf::from(path), mapping).map_err(to_string_err)
}

#[tauri::command]
pub fn messages_meta(state: State<'_, AppState>) -> Result<crate::status::DbMeta, String> {
  let (count, max_id) = state.db.dao().messages_meta().map_err(to_string_err)?;
  Ok(crate::status::DbMeta { messages_count: count, messages_max_id: max_id })
}

#[tauri::command]
pub fn export_execute(
  state: State<'_, AppState>,
  path: String,
  options: exporter::ExportOptions,
) -> Result<i64, String> {
  exporter::execute(&state.db, PathBuf::from(path), options).map_err(to_string_err)
}

#[tauri::command]
pub fn messages_list(
  state: State<'_, AppState>,
  query: crate::db::dao::ListQuery,
) -> Result<crate::db::dao::ListResult, String> {
  state.db.dao().messages_list(query).map_err(to_string_err)
}

#[tauri::command]
pub fn label_update_manual(
  state: State<'_, AppState>,
  message_id: i64,
  operator: String,
  new_label: crate::model::schema::LabelOutput,
) -> Result<(), String> {
  state
    .db
    .dao()
    .label_update_manual(message_id, &operator, new_label)
    .map_err(to_string_err)
}

#[tauri::command]
pub fn batch_start(state: State<'_, AppState>, app: AppHandle, options: BatchOptions) -> Result<(), String> {
  state.batch.start(options, app).map_err(to_string_err)
}

#[tauri::command]
pub fn batch_stop(state: State<'_, AppState>) -> Result<(), String> {
  state.batch.stop();
  Ok(())
}

#[tauri::command]
pub fn batch_status(state: State<'_, AppState>) -> Result<BatchProgress, String> {
  Ok(state.batch.status())
}

#[tauri::command]
pub fn batch_retry_failed(state: State<'_, AppState>) -> Result<(), String> {
  state.batch.retry_failed().map_err(to_string_err)
}

fn to_string_err<E: std::fmt::Display>(e: E) -> String {
  e.to_string()
}
