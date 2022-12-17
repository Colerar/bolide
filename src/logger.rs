use anyhow::Result;
use chrono::Local;
use clap_verbosity_flag::LogLevel;
use colored::Colorize;
use log::{Level, LevelFilter, Record};

use reedline::ExternalPrinter;

fn record_to_string(record: &Record, color: bool) -> String {
  let colorize: fn(String) -> String = if color {
    match record.level() {
      Level::Error => |c| c.red().bold().to_string(),
      Level::Warn => |c| c.yellow().to_string(),
      Level::Info => |c| c.blue().to_string(),
      Level::Debug => |c| c.cyan().to_string(),
      Level::Trace => |c| c.dimmed().to_string(),
    }
  } else {
    |c| c
  };
  let time = Local::now().format("%m-%d %H:%M");
  let time = if color {
    time.to_string().dimmed().to_string()
  } else {
    time.to_string()
  };
  let level = colorize(format!("{:.1}", record.level()));
  let args = colorize(format!("{}", record.args()));
  format!("{time} {level} >>> {args}")
}

pub fn setup(level: LevelFilter, printer: &ExternalPrinter<String>) -> Result<()> {
  let file_logger = fern::Dispatch::new()
    .level(level)
    .format(|out, _message, record| {
      out.finish(format_args!("{}", record_to_string(record, false)));
    })
    .chain(fern::log_file("bolide-log.log")?);

  let repl_logger = fern::Dispatch::new()
    .level(level)
    .chain(fern::Output::call({
      let printer = printer.clone();
      move |record| {
        let string = record_to_string(record, true);
        if let Err(err) = printer.print(string) {
          println!(
            "Failed to log: message = {}, err = {}",
            record_to_string(record, true),
            err
          )
        };
      }
    }));

  fern::Dispatch::new()
    .chain(repl_logger)
    .chain(file_logger)
    .apply()?;

  Ok(())
}

#[cfg(debug_assertions)]
pub type DefaultLevel = DebugLevel;

#[cfg(not(debug_assertions))]
pub type DefaultLevel = clap_verbosity_flag::InfoLevel;

#[derive(Copy, Clone, Debug, Default)]
pub struct DebugLevel;

impl LogLevel for DebugLevel {
  fn default() -> Option<Level> {
    Some(Level::Debug)
  }
}
