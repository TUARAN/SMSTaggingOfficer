use std::path::PathBuf;

use parking_lot::Mutex;
use rusqlite::{Connection, OpenFlags};

pub mod dao;

pub struct Db {
  path: PathBuf,
  conn: Mutex<Connection>,
}

impl Db {
  pub fn open(path: PathBuf) -> Result<Self, String> {
    let conn = Connection::open_with_flags(
      &path,
      OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_FULL_MUTEX,
    )
    .map_err(|e| e.to_string())?;

    conn
      .pragma_update(None, "journal_mode", "WAL")
      .map_err(|e| e.to_string())?;
    conn
      .pragma_update(None, "synchronous", "NORMAL")
      .map_err(|e| e.to_string())?;
    conn
      .pragma_update(None, "foreign_keys", "ON")
      .map_err(|e| e.to_string())?;

    Ok(Self {
      path,
      conn: Mutex::new(conn),
    })
  }

  pub fn migrate(&self) -> Result<(), String> {
    let sql = include_str!("./migrations/001_init.sql");
    self.conn.lock().execute_batch(sql).map_err(|e| e.to_string())
  }

  pub fn dao(&self) -> dao::Dao<'_> {
    dao::Dao::new(self)
  }

  pub fn path(&self) -> &PathBuf {
    &self.path
  }

  pub fn ping(&self) -> Result<(), String> {
    self
      .conn
      .lock()
      .execute_batch("SELECT 1;")
      .map_err(|e| e.to_string())
  }

  pub fn conn(&self) -> parking_lot::MutexGuard<'_, Connection> {
    self.conn.lock()
  }
}
