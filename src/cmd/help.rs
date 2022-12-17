use crate::cmd::{Command, ParseAndSend, Runner, Sender, CMDS};
use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::Parser;
use std::cmp::min;

pub fn command() -> Command {
  Command {
    name: "help",
    alias: Vec::new(),
    help: "Show help for certain command".into(),
    runner: Box::new(Help),
  }
}

struct Help;
#[derive(Parser)]
struct Opts {
  /// Page of commands
  #[clap(value_name = "PAGE", default_value_t = 1)]
  #[clap(value_parser = clap::value_parser!(u8).range(1..100))]
  page: u8,
  #[clap(short = 's', long, value_name = "SIZE", default_value_t = 10)]
  #[clap(value_parser = clap::value_parser!(u8).range(1..20))]
  page_size: u8,
}

#[async_trait]
impl Runner for Help {
  async fn run(&self, args: Vec<String>, sender: &Sender) -> Result<()> {
    let Some(opts) = Opts::parse_print(args, sender).await else {
      return Ok(());
    };

    let page_size = opts.page_size as usize;
    let page = opts.page as usize;
    let total_size = CMDS.len();
    let max_page = (f64::from(total_size as u32) / f64::from(page_size as u32)).ceil() as usize;

    if opts.page < 1 || page > max_page {
      return sender
        .send_text(format!("Page out of range {}/{}", opts.page, max_page))
        .await;
    };

    let cmds = CMDS
      .get((page - 1) * page_size..min(page * page_size, total_size as usize))
      .context("Out of range")?;

    let mut string = String::new();
    // max width
    let w = cmds.iter().map(|cmd| cmd.name.len()).max().unwrap_or(5);

    string.push_str(format!("===== Page {}/{} =====\n", opts.page, max_page).as_str());
    for cmd in cmds {
      string.push_str(format!("{: <w$} >> {}\n", cmd.name, cmd.help).as_str())
    }
    string.push_str("Use `<command> -h` for more details");

    sender
      .send_text(string)
      .await
      .context("Failed to send help")?;

    Ok(())
  }
}
