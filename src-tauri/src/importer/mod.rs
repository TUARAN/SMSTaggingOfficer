use std::{collections::HashMap, path::PathBuf};

use calamine::{open_workbook_auto, Data, Reader};
use csv::StringRecord;
use serde::{Deserialize, Serialize};

use crate::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreview {
  pub headers: Vec<String>,
  pub rows: Vec<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnMapping {
  pub content: String,
  pub received_at: Option<String>,
  pub sender: Option<String>,
  pub phone: Option<String>,
  pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportExecuteResult {
  pub total_rows: i64,
  pub valid_rows: i64,
  pub inserted: i64,
  pub skipped_empty_content: i64,
  pub first_insert_id: Option<i64>,
  pub last_insert_id: Option<i64>,
}

pub fn preview(path: PathBuf, max_rows: usize) -> Result<ImportPreview, String> {
  let ext = path
    .extension()
    .and_then(|s| s.to_str())
    .unwrap_or("")
    .to_ascii_lowercase();

  match ext.as_str() {
    "csv" => preview_csv(path, max_rows),
    "xlsx" => preview_xlsx(path, max_rows),
    _ => Err("unsupported file extension (csv/xlsx)".to_string()),
  }
}

pub fn execute(db: &Db, path: PathBuf, mapping: ColumnMapping) -> Result<ImportExecuteResult, String> {
  let ext = path
    .extension()
    .and_then(|s| s.to_str())
    .unwrap_or("")
    .to_ascii_lowercase();

  match ext.as_str() {
    "csv" => execute_csv(db, path, mapping),
    "xlsx" => execute_xlsx(db, path, mapping),
    _ => Err("unsupported file extension (csv/xlsx)".to_string()),
  }
}

fn preview_csv(path: PathBuf, max_rows: usize) -> Result<ImportPreview, String> {
  let mut rdr = csv::Reader::from_path(path).map_err(|e| e.to_string())?;
  let headers = rdr
    .headers()
    .map_err(|e| e.to_string())?
    .iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();

  let mut rows: Vec<HashMap<String, String>> = vec![];
  for (idx, rec) in rdr.records().enumerate() {
    if idx >= max_rows {
      break;
    }
    let rec = rec.map_err(|e| e.to_string())?;
    rows.push(record_to_map(&headers, &rec));
  }

  Ok(ImportPreview { headers, rows })
}

fn execute_csv(db: &Db, path: PathBuf, mapping: ColumnMapping) -> Result<ImportExecuteResult, String> {
  let mut rdr = csv::Reader::from_path(path).map_err(|e| e.to_string())?;
  let headers = rdr
    .headers()
    .map_err(|e| e.to_string())?
    .iter()
    .map(|s| s.to_string())
    .collect::<Vec<_>>();

  let idx_content = header_index(&headers, &mapping.content)?;
  let idx_received_at = mapping
    .received_at
    .as_deref()
    .map(|h| header_index(&headers, h))
    .transpose()?;
  let idx_sender = mapping
    .sender
    .as_deref()
    .map(|h| header_index(&headers, h))
    .transpose()?;
  let idx_phone = mapping
    .phone
    .as_deref()
    .map(|h| header_index(&headers, h))
    .transpose()?;
  let idx_source = mapping
    .source
    .as_deref()
    .map(|h| header_index(&headers, h))
    .transpose()?;

  let mut total_rows = 0i64;
  let mut valid_rows = 0i64;
  let mut inserted = 0i64;
  let mut first_insert_id: Option<i64> = None;
  let mut last_insert_id: Option<i64> = None;

  for rec in rdr.records() {
    let rec = rec.map_err(|e| e.to_string())?;
    total_rows += 1;
    let content = rec.get(idx_content).unwrap_or("").trim();
    if content.is_empty() {
      continue;
    }

    valid_rows += 1;

    let received_at = idx_received_at.and_then(|i| rec.get(i)).map(|s| s.trim()).filter(|s| !s.is_empty());
    let sender = idx_sender.and_then(|i| rec.get(i)).map(|s| s.trim()).filter(|s| !s.is_empty());
    let phone = idx_phone.and_then(|i| rec.get(i)).map(|s| s.trim()).filter(|s| !s.is_empty());
    let source = idx_source.and_then(|i| rec.get(i)).map(|s| s.trim()).filter(|s| !s.is_empty());

    let id = db.dao().insert_message(content, received_at, sender, phone, source)?;
    if first_insert_id.is_none() {
      first_insert_id = Some(id);
    }
    last_insert_id = Some(id);
    inserted += 1;
  }

  Ok(ImportExecuteResult {
    total_rows,
    valid_rows,
    inserted,
    skipped_empty_content: total_rows - valid_rows,
    first_insert_id,
    last_insert_id,
  })
}

fn preview_xlsx(path: PathBuf, max_rows: usize) -> Result<ImportPreview, String> {
  let mut wb = open_workbook_auto(&path).map_err(|e| e.to_string())?;
  let sheet_name = wb
    .sheet_names()
    .get(0)
    .cloned()
    .ok_or_else(|| "no sheet in xlsx".to_string())?;

  let range = wb
    .worksheet_range(&sheet_name)
    .map_err(|e| e.to_string())?;

  let mut rows_iter = range.rows();
  let header_row = rows_iter.next().ok_or_else(|| "empty sheet".to_string())?;
  let headers = header_row.iter().map(cell_to_string).collect::<Vec<_>>();

  let mut rows: Vec<HashMap<String, String>> = vec![];
  for (idx, row) in rows_iter.enumerate() {
    if idx >= max_rows {
      break;
    }
    let mut map = HashMap::new();
    for (i, h) in headers.iter().enumerate() {
      let v = row.get(i).map(cell_to_string).unwrap_or_default();
      map.insert(h.clone(), v);
    }
    rows.push(map);
  }

  Ok(ImportPreview { headers, rows })
}

fn execute_xlsx(db: &Db, path: PathBuf, mapping: ColumnMapping) -> Result<ImportExecuteResult, String> {
  let mut wb = open_workbook_auto(&path).map_err(|e| e.to_string())?;
  let sheet_name = wb
    .sheet_names()
    .get(0)
    .cloned()
    .ok_or_else(|| "no sheet in xlsx".to_string())?;

  let range = wb
    .worksheet_range(&sheet_name)
    .map_err(|e| e.to_string())?;

  let mut rows_iter = range.rows();
  let header_row = rows_iter.next().ok_or_else(|| "empty sheet".to_string())?;
  let headers = header_row.iter().map(cell_to_string).collect::<Vec<_>>();

  let idx_content = header_index(&headers, &mapping.content)?;
  let idx_received_at = mapping
    .received_at
    .as_deref()
    .map(|h| header_index(&headers, h))
    .transpose()?;
  let idx_sender = mapping
    .sender
    .as_deref()
    .map(|h| header_index(&headers, h))
    .transpose()?;
  let idx_phone = mapping
    .phone
    .as_deref()
    .map(|h| header_index(&headers, h))
    .transpose()?;
  let idx_source = mapping
    .source
    .as_deref()
    .map(|h| header_index(&headers, h))
    .transpose()?;

  let mut total_rows = 0i64;
  let mut valid_rows = 0i64;
  let mut inserted = 0i64;
  let mut first_insert_id: Option<i64> = None;
  let mut last_insert_id: Option<i64> = None;

  for row in rows_iter {
    total_rows += 1;
    let content = row.get(idx_content).map(cell_to_string).unwrap_or_default();
    let content = content.trim();
    if content.is_empty() {
      continue;
    }

    valid_rows += 1;

    let received_at = idx_received_at
      .and_then(|i| row.get(i))
      .map(cell_to_string)
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty());
    let sender = idx_sender
      .and_then(|i| row.get(i))
      .map(cell_to_string)
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty());
    let phone = idx_phone
      .and_then(|i| row.get(i))
      .map(cell_to_string)
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty());
    let source = idx_source
      .and_then(|i| row.get(i))
      .map(cell_to_string)
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty());

    let id = db.dao().insert_message(
      content,
      received_at.as_deref(),
      sender.as_deref(),
      phone.as_deref(),
      source.as_deref(),
    )?;

    if first_insert_id.is_none() {
      first_insert_id = Some(id);
    }
    last_insert_id = Some(id);
    inserted += 1;
  }

  Ok(ImportExecuteResult {
    total_rows,
    valid_rows,
    inserted,
    skipped_empty_content: total_rows - valid_rows,
    first_insert_id,
    last_insert_id,
  })
}

fn header_index(headers: &[String], name: &str) -> Result<usize, String> {
  headers
    .iter()
    .position(|h| h == name)
    .ok_or_else(|| format!("header not found: {name}"))
}

fn record_to_map(headers: &[String], rec: &StringRecord) -> HashMap<String, String> {
  let mut map = HashMap::new();
  for (i, h) in headers.iter().enumerate() {
    map.insert(h.clone(), rec.get(i).unwrap_or("").to_string());
  }
  map
}

fn cell_to_string(cell: &Data) -> String {
  match cell {
    Data::Empty => "".to_string(),
    Data::String(s) => s.clone(),
    Data::Float(f) => {
      if f.fract() == 0.0 {
        format!("{}", *f as i64)
      } else {
        format!("{f}")
      }
    }
    Data::Int(i) => i.to_string(),
    Data::Bool(b) => b.to_string(),
    Data::DateTime(f) => format!("{f}"),
    Data::DateTimeIso(s) => s.clone(),
    Data::DurationIso(s) => s.clone(),
    Data::Error(e) => format!("{e:?}"),
  }
}
