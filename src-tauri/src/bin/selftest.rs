use std::path::PathBuf;

use sms_tagging_officer::selftest;

fn main() -> Result<(), String> {
  let out_dir = PathBuf::from("tools");
  let r = selftest::run(out_dir)?;
  println!("[selftest] db: {}", r.db_path.display());
  println!("[selftest] inserted messages: {}", r.inserted);
  println!("[selftest] labeled messages: {}", r.labeled);
  println!(
    "[selftest] exported jsonl lines: {} -> {}",
    r.written_jsonl,
    r.jsonl_path.display()
  );
  println!(
    "[selftest] exported csv rows: {} -> {}",
    r.written_csv,
    r.csv_path.display()
  );
  Ok(())
}

