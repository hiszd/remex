use clap::{arg, Arg, Command};

pub fn cli() -> Command {
  Command::new("remex_server")
    .about("Remex server")
    .subcommand_required(false)
    .arg_required_else_help(false)
    .allow_external_subcommands(true)
    .arg(Arg::new("verbose").short('v'))
}

pub fn cli_old() -> Command {
  Command::new("remex_server")
    .about("Remex server")
    .subcommand_required(true)
    .arg_required_else_help(true)
    .allow_external_subcommands(true)
    .subcommand(
      Command::new("subscribe")
        .about("Subscribes to a keyboard")
        .alias("sub")
        .arg(
          arg!(-s --serial <SERIAL> "The serial number of the keyboard")
            .required_unless_present("name"),
        )
        .arg(arg!(-n --name <NAME> "The name of the keyboard").required_unless_present("serial"))
        .arg_required_else_help(true),
    )
    .subcommand(
      Command::new("exec")
        .about("Executes a command on the keyboard")
        .subcommand_required(true)
        .arg(
          arg!(-s --serial <SERIAL> "The serial number of the keyboard")
            .required_unless_present("name"),
        )
        .arg(arg!(-n --name <NAME> "The name of the keyboard").required_unless_present("serial"))
        .subcommand(Command::from(crate::commands::Commands::LayerSet(0)))
        .arg_required_else_help(true),
    )
    .subcommand(Command::new("list").about("List all keyboard nodes"))
}
