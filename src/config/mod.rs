use anyhow::{bail, Context, Result};
use log::info;
use serde::Deserialize;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::{fs, process};

#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
pub struct Config {
  #[serde(default = "Default::default")]
  pub enabled_chats: Vec<String>,
  pub telegram: Telegram,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
pub struct Telegram {
  pub telegram_token: String,
  pub proxy: Option<String>,
  #[serde(default = "Default::default")]
  pub time: Time,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
pub struct Time {
  fetch_delay: u64,
  failed_delay: u64,
}

impl Default for Time {
  fn default() -> Self {
    Self {
      fetch_delay: 1000,
      failed_delay: 5000,
    }
  }
}

pub fn init(path: &PathBuf) -> Result<Config> {
  let path = path.join("config.toml");

  info!("Initializing config file...");

  if path.exists() && path.is_file() {
    info!("Reading config from {}...", &path.to_string_lossy());
    let file = File::open(&path).context("Failed to")?;
    let mut buf_reader = BufReader::new(file);
    let mut config_str = String::new();
    buf_reader
      .read_to_string(&mut config_str)
      .with_context(|| {
        format!(
          "Failed to read config file as String: {}",
          &path.to_string_lossy()
        )
      })?;
    let config: Config = toml::from_str(&*config_str)
      .with_context(|| format!("Failed to parse config file: {}", &path.to_string_lossy()))?;
    Ok(config)
  } else if !path.exists() {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create folder: {}", parent.to_string_lossy()))?;
    };
    let config = File::create(&path).with_context(|| {
      format!(
        "Failed to create default config: {}",
        &path.to_string_lossy()
      )
    })?;
    const DEFAULT_CONFIG: &[u8] = include_bytes!("../../config/config.example.toml");

    {
      let mut buf_writer = BufWriter::new(config);
      buf_writer.write_all(DEFAULT_CONFIG).with_context(|| {
        format!(
          "Failed to write default config to: {}",
          &path.to_string_lossy()
        )
      })?;
    }
    info!("Default config writed to {}", &path.to_string_lossy());
    info!("Please take a look and configure bot, exiting...");
    process::exit(0)
  } else {
    bail!("Path is not a file: {}", path.to_string_lossy())
  }
}
