use anyhow::Context;
use async_trait::async_trait;
use clap::{Parser};
use frankenstein::{AsyncTelegramApi, ChatId, SendMessageParams};
use lazy_static::lazy_static;
use log::{error, info};
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::ffi::OsString;
use std::sync::Arc;

mod help;
mod version;

lazy_static! {
  static ref CMDS: Vec<Command> = vec![help::command(), version::command()];
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
  Telegram {
    api: Arc<frankenstein::AsyncApi>,
    chat: ChatId,
  },
}

impl Sender {
  pub async fn send_text<I>(&self, text: I) -> anyhow::Result<()> where
     I: Into<Cow<'static, str>> {
    match self {
      Sender::Console => {
        info!("{}", text.into());
      },
      Sender::Telegram { api, chat } => {
        api
          .send_message(
            &SendMessageParams::builder()
              .text(text.into())
              .chat_id(chat.clone())
              .build(),
          )
          .await
          .with_context(|| format!("Failed to send message to chat {chat:?}"))?;
      },
    }
    Ok(())
  }
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
  async fn parse_print<I, T>(itr: I, sender: &Sender) -> Option<Self>
  where
    I: IntoIterator<Item = T> + Send,
    T: Into<OsString> + Clone,
  {
    match Self::try_parse_from(itr) {
      Ok(ok) => Some(ok),
      Err(err) => {
        let send_result = match sender {
          Sender::Console => err.print().context("Failed to print err to console"),
          Sender::Telegram { .. } => sender.send_text(err.render().to_string()).await,
        };
        if let Err(err) = send_result {
          error!("Failed to send send text to sender {sender:?}: {err}");
        };
        None
      },
    }
  }
}

impl<T: Parser> ParseAndSend for T {}
