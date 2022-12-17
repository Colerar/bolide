use log::{error, trace};
use std::borrow::Cow;
use std::process::exit;

use reedline::{
  Prompt, PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus, Reedline, Signal,
};

use std::sync::Arc;

use crate::cmd::Commands;
use crate::cmd::Sender::Console;

pub async fn handle_line(reader: &mut Reedline, cmds: Arc<Commands>) {
  loop {
    let line = match reader.read_line(&CustomPrompt) {
      Ok(sig) => match sig {
        Signal::Success(buffer) => buffer,
        Signal::CtrlC => {
          println!("<Interrupted>");
          continue;
        },
        Signal::CtrlD => {
          println!("\nAborted!");
          exit(0);
        },
      },
      Err(err) => {
        println!("{}", err);
        continue;
      },
    };

    let args = shlex::split(&line);
    let Some(args) = args else { return };
    if args.is_empty() {
      continue;
    }
    trace!("{:?}", &args);
    let sender = Console;
    if let Some(cmd) = cmds
      .key_map
      .get(args[0].as_str())
      .or_else(|| cmds.alias_map.get(args[0].as_str()))
    {
      if let Err(err) = cmd.runner.run(args, &sender).await {
        error!("Error during executing command {}: {err}", cmd.name)
      }
    } else {
      error!("No such command `{}`", args[0])
    }
  }
}

#[derive(Clone)]
pub struct CustomPrompt;
impl Prompt for CustomPrompt {
  fn render_prompt_left(&self) -> Cow<str> {
    {
      Cow::Borrowed("")
    }
  }

  fn render_prompt_right(&self) -> Cow<str> {
    Cow::Borrowed("")
  }

  fn render_prompt_indicator(&self, _edit_mode: PromptEditMode) -> Cow<str> {
    Cow::Owned("> ".to_string())
  }

  fn render_prompt_multiline_indicator(&self) -> Cow<str> {
    Cow::Borrowed("::: ")
  }

  fn render_prompt_history_search_indicator(
    &self,
    history_search: PromptHistorySearch,
  ) -> Cow<str> {
    let prefix = match history_search.status {
      PromptHistorySearchStatus::Passing => "",
      PromptHistorySearchStatus::Failing => "failing ",
    };

    Cow::Owned(format!(
      "({}reverse-search: {}) ",
      prefix, history_search.term
    ))
  }
}
