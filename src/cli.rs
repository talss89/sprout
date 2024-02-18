use ::clap::Parser;
use colored::*;

use std::time::SystemTime;

use crate::{cli::clap::Options, engine::Engine};

pub mod clap;
mod commands;
mod repo;
mod snapshot;

pub fn entrypoint(engine: &Engine) {
    let now = SystemTime::now();
    let options = Options::parse();

    match commands::run(engine) {
        Ok(response) => {
            let elapsed = now.elapsed().unwrap();
            eprintln!();
            eprintln!(
                "{: ^9} {}",
                "SUCCESS".bold().black().on_green(),
                response.msg.bold().dimmed()
            );
            eprintln!(
                "{: ^9} {}",
                "",
                format!("in {}", indicatif::HumanDuration(elapsed))
                    .dimmed()
                    .italic()
            );

            if options.json {
                if let Some(data) = response.data {
                    println!("{}", data)
                }
            }

            std::process::exit(0);
        }

        Err(e) => {
            let elapsed = now.elapsed().unwrap();
            eprintln!();
            eprintln!(
                "{: ^9} {}",
                "ERROR".bold().white().on_red(),
                e.to_string().bold().dimmed()
            );
            eprintln!(
                "{: ^9} {}",
                "",
                format!("in {}", indicatif::HumanDuration(elapsed))
                    .dimmed()
                    .italic()
            );
            std::process::exit(101);
        }
    }
}
