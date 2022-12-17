use crate::cmd::{Command, ParseAndSend, Runner, Sender};
use anyhow::Context;
use async_trait::async_trait;
use clap::Parser;

pub fn version_command() -> Command {
  Command {
    name: "version",
    alias: Vec::new(),
    help: "Show Bolide version".into(),
    runner: Box::new(Version),
  }
}

struct Version;
#[derive(Parser)]
struct VersionOpts {}

const fn version_str() -> &'static str {
  concat!(clap::crate_name!(), " Version: v", clap::crate_version!())
}

#[async_trait]
impl Runner for Version {
  async fn run(&self, args: Vec<String>, sender: &Sender) -> anyhow::Result<()> {
    if VersionOpts::parse_and_print_err(args, sender).await {
      return Ok(());
    };
    sender
      .send_text(version_str().into())
      .await
      .with_context(|| format!("Failed to send command message to {:?}", sender))
  }
}
