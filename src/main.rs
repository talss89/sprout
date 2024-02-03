use clap::Parser;
use std::{path::PathBuf, process::Command};

use crate::{
    cli::{Options, SubCommand},
    wordpress::WordPress,
};

mod cli;
mod repo;
mod wordpress;

fn main() -> anyhow::Result<()> {
    let options = Options::parse();

    match options.subcommand {
        SubCommand::Init => {
            crate::repo::initialise();
        }

        SubCommand::Snap(args) => {
            let wp = WordPress::new(options.path.to_owned())?;
            crate::repo::snapshot(&wp, args.branch)?;
        }

        SubCommand::Snap(args) => {
            let wp = WordPress::new(options.path.to_owned())?;
            crate::repo::snapshot(&wp, args.branch)?;
        }
    }

    println!("Hello, world");

    Ok(())
}
