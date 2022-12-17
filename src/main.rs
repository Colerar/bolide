extern crate core;

use crate::cmd::Commands;
use crate::logger::DefaultLevel;
use crate::tg::{build_tg_client, tg_log_self, tg_pull};
use anyhow::{Context, Result};
use clap::{Parser, ValueHint};
use clap_verbosity_flag::Verbosity;
use lazy_static::lazy_static;
use log::{debug, error, info};
use once_cell::sync::OnceCell;
use reqwest::{Client, Proxy};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};
use tokio::spawn;
pub mod cmd;
pub mod config;
pub mod events;
pub mod logger;
pub mod term;
pub mod tg;

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

lazy_static! {
  static ref START_TIME: u64 = init_start_time();
  static ref COMMANDS: Commands = Commands::default();
}

static BOT_USERNAME: OnceCell<String> = OnceCell::new();

fn init_start_time() -> u64 {
  let start = SystemTime::now();
  let since_the_epoch = start
    .duration_since(UNIX_EPOCH)
    .expect("Time went backwards");
  since_the_epoch.as_secs()
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
  let config = Arc::new(config);
  debug!("{config:?}");

  let cli = build_reqwest(&config.proxy)?;

  let tg_api = build_tg_client(cli, &config.telegram.token);
  info!(
    "Bot starts at: {}, any updates after it will be ignored.",
    *START_TIME
  );
  {
    let bot = tg_log_self(tg_api.clone()).await?;
    BOT_USERNAME
      .set(bot.username.context("Cannot get bot username")?)
      .expect("Bot username is already set");
  }
  let telegram_pull = async {
    tg_pull(tg_api, config).await;
  };

  let console_input = async {
    term::handle_line().await;
  };
  let job_vec = vec![spawn(console_input), spawn(telegram_pull)];

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

fn build_reqwest(proxy: &config::Proxy) -> Result<Client> {
  let mut cli = Client::builder();
  if let Some(proxy) = &proxy.connection {
    let proxy =
      Proxy::all(proxy.clone()).with_context(|| format!("Failed to set `{proxy}` as proxy"))?;
    cli = cli.proxy(proxy);
  }
  if let Some(_user) = &proxy.user {
    if let Some(_pwd) = &proxy.password {
      unimplemented!("Basic Auth for Proxy");
    }
  }
  cli.build().context("Failed to build reqwest client")
}
