use std::fmt::Display;
use std::time::Duration;

use tokio::time::sleep;

use crate::cmd::Sender;
use crate::config::Config;
use crate::events::onmsg::on_message;
use crate::{BOT_USERNAME, COMMANDS, START_TIME};
use anyhow::{Context, Result};
use async_stream::stream;
use frankenstein::{
  AllowedUpdate, AsyncApi, AsyncTelegramApi, ChatId, GetUpdatesParams, Update, UpdateContent, User,
};
use futures::pin_mut;
use futures_util::StreamExt;
use log::{error, info, trace};
use reqwest::Client;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::spawn;

pub fn build_tg_client(reqwest: Client, token: &str) -> Arc<AsyncApi> {
  let tg_api = AsyncApi::builder()
    .api_url(format!("{}{}", frankenstein::BASE_API_URL, token,))
    .client(reqwest)
    .build();
  Arc::new(tg_api)
}

fn update_params(offset: u32, limit: u32) -> GetUpdatesParams {
  GetUpdatesParams::builder()
    .allowed_updates(vec![AllowedUpdate::Message])
    .offset(offset)
    .limit(limit)
    .build()
}

pub async fn tg_pull(tg_api: Arc<AsyncApi>, config: Arc<Config>) {
  let update_seq = AtomicU32::new(0);

  let stream = {
    let tg_api = Arc::clone(&tg_api);
    let config = Arc::clone(&config);
    stream! {
      loop {
        let result = tg_api.get_updates(&update_params(update_seq.load(Ordering::Acquire), 500)).await;
        let updates = match result {
          Ok(msg) => msg.result,
          Err(err) => {
            error!(
              "Failed to get updates, retry after {}ms: {:?}",
              config.telegram.time.failed_delay,
              err.to_string(),
            );
            sleep(Duration::from_millis(config.telegram.time.failed_delay)).await;
            continue;
          },
        };
        if let Some(last) = updates.iter().last() {
          let new_id = last.update_id + 1;
          update_seq.store(new_id, Ordering::Release);
        }
        for update in updates.into_iter() {
          yield update;
        }
        trace!("Yield updates..");
        sleep(Duration::from_millis(config.telegram.time.fetch_delay)).await;
      }
    }
  };

  pin_mut!(stream);

  while let Some(value) = stream.next().await {
    let tg_api = Arc::clone(&tg_api);
    let config = Arc::clone(&config);
    spawn(async move {
      if let Err(err) = process_update(tg_api, config, value).await {
        error!("Error during processing update: {err}")
      };
    });
  }
}

pub async fn tg_log_self(tg_api: Arc<AsyncApi>) -> Result<User> {
  let me = tg_api
    .get_me()
    .await
    .context("Failed to get telegram bot self info")?;
  info!(
    "Current tg bot: {}",
    me.result
      .username
      .clone()
      .context("Failed to get username for bot, maybe token is invalid")?
  );
  Ok(me.result)
}

pub async fn process_update(api: Arc<AsyncApi>, config: Arc<Config>, update: Update) -> Result<()> {
  match update.content {
    UpdateContent::Message(msg) => {
      if msg.date < *START_TIME {
        return Ok(());
      }

      let enabled = config
        .telegram
        .enabled_chats
        .contains(&msg.chat.id.to_string());
      if !enabled {
        return Ok(());
      };

      let Some(text) = &msg.text else { return Ok(()) };

      if text.is_empty() {
        return Ok(());
      }

      if let Some(stripped) = text.strip_prefix('/') {
        let args = shlex::split(stripped);
        let Some(mut args) = args else { return Ok(()) };
        if args.is_empty() {
          return Ok(());
        }
        let main = args[0].clone();
        let main_split: Vec<_> = main.split('@').filter(|s| !s.is_empty()).collect();
        if main_split.len() == 2 {
          let at = main_split[1];
          let Some(bot) = BOT_USERNAME.get() else { return Ok(()) };
          if !at.eq_ignore_ascii_case(bot) && !at.eq_ignore_ascii_case("bolide") {
            return Ok(());
          } else {
            args.remove(0);
            args.insert(0, main_split[0].to_string());
          }
        }

        let sender: Sender = Sender::Telegram {
          api,
          chat: ChatId::Integer(msg.chat.id),
        };
        if let Some(cmd) = COMMANDS
          .key_map
          .get(args[0].as_str())
          .or_else(|| COMMANDS.alias_map.get(args[0].as_str()))
        {
          if let Err(err) = cmd.runner.run(args, &sender).await {
            error!("Error during executing command {}: {err}", cmd.name)
          }
        } else if let Err(err) = sender
          .send_text(format!("No such command `{}`", args[0].as_str()))
          .await
        {
          error!("Failed to send message to {:?}: {err}", msg.chat);
        }
      } else {
        on_message(api, msg);
      }

      Ok(())
    },
    _ => {
      info!("Unsupported message type: {}", MessageType(update.content));
      Ok(())
    },
  }
}

struct MessageType(UpdateContent);

impl Display for MessageType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let str = match self.0 {
      UpdateContent::Message(_) => "Message",
      UpdateContent::EditedMessage(_) => "EditedMessage",
      UpdateContent::ChannelPost(_) => "ChannelPost",
      UpdateContent::EditedChannelPost(_) => "EditedChannelPost",
      UpdateContent::InlineQuery(_) => "InlineQuery",
      UpdateContent::ChosenInlineResult(_) => "ChosenInlineResult",
      UpdateContent::CallbackQuery(_) => "CallbackQuery",
      UpdateContent::ShippingQuery(_) => "ShippingQuery",
      UpdateContent::PreCheckoutQuery(_) => "PreCheckoutQuery",
      UpdateContent::Poll(_) => "Poll",
      UpdateContent::PollAnswer(_) => "PollAnswer",
      UpdateContent::MyChatMember(_) => "MyChatMember",
      UpdateContent::ChatMember(_) => "ChatMember",
      UpdateContent::ChatJoinRequest(_) => "ChatJoinRequest",
    };
    f.write_str(str)
  }
}
