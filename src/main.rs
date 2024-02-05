use clap::Parser;
use cli::CliResponse;
use colored::*;
use dialoguer::{Confirm, Input};

use env_logger::Builder;
use homedir::get_my_home;
use log::{info, warn};
use passwords::PasswordGenerator;
use rustic_backend::BackendOptions;
use rustic_core::{Progress, ProgressBars, RepositoryOptions};
use std::{io::Write, time::SystemTime};
use theme::CliTheme;

use crate::{
    cli::{Options, RepoCommand, SubCommand},
    project::Project,
    repo::SproutProgressBar,
    stash::Stash,
};

mod cli;
mod project;
mod repo;
mod sproutfile;
mod stash;
mod theme;

include!(concat!(env!("OUT_DIR"), "/built.rs"));

fn main() {
    let now = SystemTime::now();
    let options = Options::parse();

    match run() {
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
                match response.data {
                    Some(data) => {
                        println!("{}", data)
                    }
                    None => {}
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

fn run() -> anyhow::Result<CliResponse> {
    let options = Options::parse();

    let logo = format!(
        r"                         
          +++++          
          ++ ++          
   +++++  +++++  +++++     {}
  ++  ++    +    ++  ++    {}
   ++++++   +   ++++++     {}
       +++  +  +++       
         +++++++         
          ++ ++          
          +++++          
                         ",
        format!("Sprout {}", PKG_VERSION).bold().green(),
        "Content and database seeding for WordPress"
            .white()
            .bold()
            .dimmed(),
        format!("{} | https://github.com/talss89/sprout", TARGET)
            .white()
            .dimmed(),
    );
    eprintln!("{:^26}", logo.green());

    let env = env_logger::Env::default()
        .filter_or("SPROUT_LOG_LEVEL", "info")
        .write_style_or("SPROUT_LOG_STYLE", "always");

    Builder::from_env(env)
        .format(|buf, record| {
            let mut level_style = buf.default_level_style(record.level());
            if record.target().starts_with("rustic_core")
                || record.target().starts_with("rustic_backend")
            {
                if let Err(_) = std::env::var("SPROUT_DEBUG_RUSTIC") {
                    return Ok(());
                }

                let mut dimmed = buf.style();
                dimmed.set_dimmed(true);
                level_style.set_dimmed(true);
                writeln!(
                    buf,
                    "{: ^9} [{}] {}",
                    "---".dimmed(),
                    level_style.value(record.target().replace("_subproc_", "")),
                    dimmed.value(record.args())
                )
            } else {
                writeln!(
                    buf,
                    "{: ^9} {}",
                    level_style.value(record.level()),
                    record.args()
                )
            }
        })
        .init();

    let sprout_home = get_my_home().unwrap().unwrap().as_path().join(".sprout");

    std::env::set_current_dir(&options.path)?;

    match options.subcommand {
        SubCommand::Init => {
            info!("Creating a `sprout.yaml` for your project...");
            let project = Project::initialise(options.path.to_owned())?;
            info!("Your project is ready. Please check and customise `sprout.yaml` as required");

            return Ok(CliResponse {
                msg: "Project initialised".to_string(),
                data: Some(serde_json::to_string(&project)?),
            });
        }

        SubCommand::Repo(args) => match args.subcommand {
            RepoCommand::Init(args) => {
                info!("Initialising new Sprout repository...");

                let pg = PasswordGenerator::new()
                    .length(64)
                    .numbers(true)
                    .lowercase_letters(true)
                    .uppercase_letters(true)
                    .symbols(false)
                    .spaces(false)
                    .strict(true);

                let generated_access_key = pg.generate_one().unwrap();

                let access_key = match args.access_key {
                    Some(access_key) => access_key,
                    None => Input::with_theme(&CliTheme::default())
                        .with_prompt("Please set a secure access key for this repository.")
                        .default(generated_access_key.to_string())
                        .interact_text()
                        .unwrap(),
                };

                let progress = SproutProgressBar {};
                let spinner =
                    progress.progress_spinner(format!("Creating repository at {}", &args.path));

                let repo_opts = RepositoryOptions::default().password(access_key);
                let backend = BackendOptions::default().repository(&args.path);
                let repo = crate::repo::open_repo(&backend, repo_opts)?;
                let repo = crate::repo::initialise(repo)?;

                spinner.finish();

                info!("Sprout repo created at {}", &args.path);

                return Ok(CliResponse {
                    msg: "Sprout repository initialised".to_string(),
                    data: Some(serde_json::to_string(&repo)?),
                });
            }
        },

        SubCommand::Snap(args) => {
            let mut project = Project::new(options.path.to_owned())?;

            project.print_header();

            let access_key = project.obtain_access_key()?;

            let repo = project.open_repo(access_key)?;

            if let Some(branch) = args.branch {
                if branch != project.config.branch {
                    let confirmation = Confirm::with_theme(&CliTheme::default())
                        .with_prompt(format!(
                            "Do you wish to switch content branch from {} to {}?",
                            project.config.branch, branch
                        ))
                        .interact()
                        .unwrap();

                    if !confirmation {
                        return Ok(CliResponse {
                            msg: "Aborted by user, but no error".to_string(),
                            data: None,
                        });
                    }

                    project.config.branch = branch;
                }
            }

            info!(
                "Checking the project uniqueness digest against the remote repo for {}:{}...",
                project.config.name, project.config.branch
            );

            let latest_hash = project.get_latest_unique_hash(&repo)?;

            if let Some(id) = latest_hash {
                if let Some(local_id) = &project.unique_hash {
                    if id != *local_id {
                        return Err(anyhow::anyhow!(
                            "The project uniqueness digest doesn't match the latest snapshot. Perhaps you're trying to re-use a project name?"
                        ));
                    }
                } else {
                    warn!("The local project is not in version control.");
                }
            } else {
                info!("This project or branch appears to be new.");
            }

            info!("Starting snapshot...");

            let id = project.snapshot(&repo)?;

            project.update_snapshot_id(id, project.config.branch.to_owned())?;

            return Ok(CliResponse {
                msg: "Snapshot created".to_string(),
                data: Some(serde_json::to_string(&id)?),
            });
        }

        SubCommand::Seed(args) => {
            let project = Project::new(options.path.to_owned())?;

            project.print_header();
            let access_key = project.obtain_access_key()?;

            if !args.no_stash {
                warn!("This command is destructive. Stashing your database and uploads locally.");
                let stash = Stash::new(sprout_home)?;
                stash.stash(&project)?;
            } else {
                let confirmation = Confirm::with_theme(&CliTheme::default())
                    .with_prompt("This command is destructive, and stashing has been disabled. Do you want to continue?")
                    .interact()
                    .unwrap();

                if confirmation {
                    warn!("Continuing without stashing. This will overwrite your database and uploads directory.");
                } else {
                    return Ok(CliResponse {
                        msg: "Aborted by user, but no error".to_string(),
                        data: None,
                    });
                }
            }

            let repo = project.open_repo(access_key)?;

            let snap_id = project.get_active_snapshot_id(&repo)?;

            crate::repo::restore(repo, &project, snap_id.to_string())?;

            return Ok(CliResponse {
                msg: "Content and database seeded".to_string(),
                data: Some(serde_json::to_string(&project)?),
            });
        }

        SubCommand::UnStash(args) => {
            let project = Project::new(options.path.to_owned())?;

            project.print_header();

            info!("Restoring stashed database and uploads...");

            let confirmation = Confirm::with_theme(&CliTheme::default())
                    .with_prompt("This command is destructive. This will overwrite your database and uploads directory. Do you want to continue?")
                    .interact()
                    .unwrap();

            if !confirmation {
                return Ok(CliResponse {
                    msg: "Aborted by user, but no error".to_string(),
                    data: None,
                });
            }

            let stash = Stash::new(sprout_home)?;

            let snap_id = match args.snapshot_id {
                Some(id) => id,
                None => stash.get_latest_stash(&project)?.id.to_string(),
            };

            stash.restore(&project, snap_id.to_owned())?;

            return Ok(CliResponse {
                msg: format!(
                    "Restored the stash of {} ({})",
                    project.config.name, &snap_id
                )
                .to_string(),
                data: None,
            });
        }
    }
}
