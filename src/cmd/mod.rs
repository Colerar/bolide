use crate::cmd::version::version_command;
use anyhow::Context;
use async_trait::async_trait;
use clap::Parser;
use lazy_static::lazy_static;
use log::{error, info};
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::ffi::OsString;

mod version;

#[derive(Parser, Debug)]
struct Test {
  #[arg(short = 'c', long)]
  target_chat: String,
}

pub struct Command {
  pub name: &'static str,
  pub alias: Vec<&'static str>,
  pub help: Cow<'static, str>,
  pub runner: Box<dyn Runner>,
}

#[async_trait]
pub trait Runner: Sync + Send {
  async fn run(&self, args: Vec<String>, sender: &Sender) -> anyhow::Result<()>;
}

#[derive(Debug)]
pub enum Sender {
  Console,
  Telegram(),
}

impl Sender {
  async fn send_text(&self, text: Cow<'static, str>) -> anyhow::Result<()> {
    match self {
      Sender::Console => {
        info!("{}", text);
      },
      Sender::Telegram() => unimplemented!("telegram send"),
    }
    Ok(())
  }
}

lazy_static! {
  static ref CMDS: Vec<Command> = vec![version_command()];
}

pub struct Commands {
  pub key_map: FxHashMap<&'static str, &'static Command>,
  pub alias_map: FxHashMap<&'static str, &'static Command>,
}

impl Default for Commands {
  fn default() -> Self {
    let mut key_map: FxHashMap<&'static str, &'static Command> = FxHashMap::default();
    let mut alias_map: FxHashMap<&'static str, &'static Command> = FxHashMap::default();
    for cmd in CMDS.iter() {
      key_map.insert(cmd.name, cmd);
      for alias in cmd.alias.iter() {
        alias_map.insert(alias, cmd);
      }
    }
    Commands { key_map, alias_map }
  }
}

#[async_trait]
pub trait ParseAndSend: Parser {
  async fn parse_and_print_err<I, T>(itr: I, sender: &Sender) -> bool
  where
    I: IntoIterator<Item = T> + Send,
    T: Into<OsString> + Clone,
  {
    let mut is_err = false;
    let result = if let Err(err) = Self::try_parse_from(itr) {
      is_err = true;
      match sender {
        Sender::Console => err.print().context("Failed to print err to console"),
        Sender::Telegram() => sender.send_text(err.render().to_string().into()).await,
      }
    } else {
      Ok(())
    };
    if let Err(err) = result {
      error!("Failed to send send text to sender {sender:?}: {err}");
    };
    return is_err;
  }
}

impl<T: Parser> ParseAndSend for T {}
