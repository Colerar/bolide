extern crate core;

use anyhow::{Context, Result};

use std::path::PathBuf;

use clap::{Parser, ValueHint};
use clap_verbosity_flag::Verbosity;

use crate::cmd::Commands;
use crate::logger::DefaultLevel;

use log::error;
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
  #[clap(value_name = "DIR", default_value = ".")]
  #[arg(value_hint = ValueHint::DirPath)]
  working_directory: PathBuf,
  #[clap(flatten)]
  verbose: Verbosity<DefaultLevel>,
}

#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse();
  logger::setup(cli.verbose.log_level_filter()).context("Failed to setup logger")?;

  if let Err(err) = ctrlc::set_handler(|| {
    println!(" <Interrupted>");
  }) {
    error!("Failed to setup Ctrl + C handler: {err}");
  };

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
