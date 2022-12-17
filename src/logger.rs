use anyhow::{Context, Result};
use clap_verbosity_flag::LogLevel;
use log::{Level, LevelFilter, Record};
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::Config;

pub(crate) fn setup(verbosity: LevelFilter) -> Result<()> {
  const PATTERN: &str = "{d(%m-%d %H:%M)} {h({l:.1})} - {h({m})}{n}";
  let stdout = ConsoleAppender::builder()
    .encoder(Box::new(PatternEncoder::new(PATTERN)))
    .build();
  let config = Config::builder()
    .appender(Appender::builder().build("stdout", Box::new(stdout)))
    .build(Root::builder().appender("stdout").build(verbosity))
    .unwrap();
  log4rs::init_config(config).context("Failed to init log4rs config")?;
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
