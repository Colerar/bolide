use anyhow::{Context, Result};

use clap::Parser;
use clap_verbosity_flag::Verbosity;

use reedline::{ExternalPrinter, Reedline};

use crate::cmd::Commands;
use crate::logger::DefaultLevel;
use crate::term::handle_line;

use log::error;
use std::sync::Arc;

pub mod cmd;
pub mod logger;
pub mod term;

#[derive(Parser, Debug)]
struct Cli {
  #[clap(flatten)]
  verbose: Verbosity<DefaultLevel>,
}

#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse();
  let printer = ExternalPrinter::default();
  let mut line_editor = Reedline::create().with_external_printer(printer.clone());
  let commands = Arc::new(Commands::default());

  {
    logger::setup(cli.verbose.log_level_filter(), &printer).context("Failed to setup logger")?;
  }

  let handle = async move {
    handle_line(&mut line_editor, commands).await;
  };
  let job_vec = vec![tokio::spawn(handle)];

  for job in job_vec {
    if let Err(err) = job.await {
      error!("Failed to await job: {}", err)
    };
  }
  Ok(())
}
