use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// Fast, consistent WordPress environments for dev or production
#[derive(Parser)]
#[command(author, about, version)]
pub struct Options {
    #[arg(short, long, default_value = "./")]
    pub path: PathBuf,

    #[clap(subcommand)]
    pub subcommand: SubCommand,
}

#[derive(Subcommand, Debug)]
pub enum SubCommand {
    Init,
    Snap(SnapArgs),
}

#[derive(Args, Debug)]
pub struct SnapArgs {
    pub branch: String,
}
