use log::{error, trace};
use std::io::stdin;
use std::process::exit;
use std::sync::Arc;

use crate::cmd::Commands;
use crate::cmd::Sender::Console;

pub async fn handle_line(cmds: Arc<Commands>) {
  loop {
    let Some(line) = stdin().lines().next() else {
      println!("Ctrl + D, exiting...");
      exit(0);
    };
    let line = match line {
      Ok(line) => line,
      Err(err) => {
        error!("Failed to readline: {err}");
        continue;
      },
    };
    let args = shlex::split(line.as_str());
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
