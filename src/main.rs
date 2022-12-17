extern crate core;

use anyhow::{Context, Result};
use std::{env, fs};

use std::path::PathBuf;

use clap::{Parser, ValueHint};
use clap_verbosity_flag::Verbosity;

use crate::cmd::Commands;
use crate::logger::DefaultLevel;

use log::{debug, error};
use std::sync::Arc;

pub mod cmd;
pub mod config;
pub mod logger;
pub mod term;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
  /// Working directory
  #[clap(short = 'D', long = "workdir")]
  #[clap(value_name = "DIR")]
  #[arg(value_hint = ValueHint::DirPath)]
  working_directory: Option<PathBuf>,
  #[clap(flatten)]
  verbose: Verbosity<DefaultLevel>,
}

#[tokio::main]
async fn main() -> Result<()> {
  #[cfg(debug_assertions)]
  set_debug_env();
  let cli = Cli::parse();

  logger::setup(cli.verbose.log_level_filter()).context("Failed to setup logger")?;
  let work_dir = env::current_dir().context("Failed to get current dir")?;
  let work_dir = cli.working_directory.unwrap_or(work_dir);
  debug!("Bolide is working at {}", &work_dir.to_string_lossy());

  if let Err(err) = ctrlc::set_handler(|| {
    println!(" <Interrupted>");
  }) {
    error!("Failed to setup Ctrl + C handler: {err}");
  };

  let config = config::init(&work_dir).context("Failed to init config file")?;
  debug!("Config: {config:?}");

  let console_input = async {
    let commands = Arc::new(Commands::default());
    term::handle_line(commands).await;
  };
  let job_vec = vec![tokio::spawn(console_input)];

  for job in job_vec {
    if let Err(err) = job.await {
      error!("Failed to await job: {}", err)
    };
  }
  Ok(())
}

#[cfg(debug_assertions)]
fn set_debug_env() {
  let buf = env::current_dir().unwrap().join("work_dir");
  if !buf.exists() {
    fs::create_dir(&buf).unwrap();
  } else if !buf.is_dir() {
    println!(
      "Failed to set, path exists but not a directory: {}",
      &buf.to_string_lossy()
    );
    return;
  };

  env::set_current_dir(&buf).unwrap();
}
