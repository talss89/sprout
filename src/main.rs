use clap::Parser;
use cli::CliResponse;
use colored::*;
use dialoguer::{Confirm, Input};

use env_logger::Builder;
use log::{info, warn};
use passwords::PasswordGenerator;
use rustic_backend::BackendOptions;
use rustic_core::{Id, Progress, ProgressBars, RepositoryOptions};
use std::{fs, io::Write, time::SystemTime};
use theme::CliTheme;

use crate::{
    cli::{Options, RepoCommand, SubCommand},
    project::Project,
    repo::{Repositories, RepositoryDefinition, SproutProgressBar},
    stash::Stash,
};

mod cli;
mod engine;
mod project;
mod repo;
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
                || (record.target().starts_with("opendal")
                    && record
                        .args()
                        .to_string()
                        .contains("operation=stat path=config -> NotFound (persistent)"))
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

    let sprout_home = crate::engine::get_sprout_home();
    crate::engine::ensure_sprout_home()?;

    std::env::set_current_dir(&options.path)?;

    match options.subcommand {
        SubCommand::Init => {
            info!("Creating a `sprout.yaml` for your project and opening it in the default text editor...");
            let project = Project::initialise(options.path.to_owned())?;

            edit::edit_file(options.path.join("./sprout.yaml"))?;

            info!("Your project is ready.");

            return Ok(CliResponse {
                msg: "Project initialised".to_string(),
                data: Some(serde_json::to_string(&project)?),
            });
        }

        SubCommand::Repo(args) => match args.subcommand {
            RepoCommand::Use(args) => {
                info!("Setting default repo to {}", &args.label);

                let mut sprout_config = crate::engine::get_sprout_config()?;

                let (_, definition) = Repositories::get(&args.label)?;

                sprout_config.default_repo = args.label.to_owned();

                crate::engine::write_sprout_config(&sprout_config)?;

                Ok(CliResponse {
                    msg: format!("Set default repo to {}", args.label),
                    data: Some(serde_json::to_string(&definition)?),
                })
            }
            RepoCommand::List => {
                let defs = Repositories::list()?;
                let sprout_config = crate::engine::get_sprout_config()?;

                info!(
                    "Your repository definitions are stored at {}",
                    crate::engine::get_sprout_home().join("repos").display()
                );

                eprintln!("");
                eprintln!(
                    "{}",
                    format!("{:32} | {}", "Repository Label", "Repository URI / Path")
                        .bold()
                        .dimmed()
                );

                for (label, definition) in &defs {
                    let repo = definition.repo.clone();
                    if sprout_config.default_repo == *label {
                        eprintln!(
                            "{}",
                            format!(
                                "{:32} | {}",
                                label,
                                format!(
                                    "{} {}",
                                    repo.repository.unwrap_or("".to_string()),
                                    "<-- Default".to_string().dimmed()
                                )
                            )
                            .bold()
                            .green()
                        );
                    } else {
                        eprintln!(
                            "{}",
                            format!(
                                "{:32} | {}",
                                label,
                                repo.repository.unwrap_or("".to_string())
                            )
                        );
                    }
                }

                Ok(CliResponse {
                    msg: "Listed all repositories".to_string(),
                    data: Some(serde_json::to_string(&defs)?),
                })
            }
            RepoCommand::New(args) => {
                info!("Creating a new Sprout repository definition...");

                let definition = RepositoryDefinition {
                    access_key: "".to_string(),
                    repo: BackendOptions {
                        ..Default::default()
                    },
                };

                let repo_file = sprout_home.join(format!("repos/{}.yaml", &args.label));

                Repositories::create(&definition, &repo_file)?;

                edit::edit_file(&repo_file)?;

                let mut sprout_config = crate::engine::get_sprout_config()?;

                if sprout_config.default_repo == "" {
                    info!("Setting default repo to {}", &args.label);

                    sprout_config.default_repo = args.label.to_owned();

                    crate::engine::write_sprout_config(&sprout_config)?;
                } else {
                    info!(
                        "Your default repo ({}) is unchanged.",
                        sprout_config.default_repo
                    );
                }

                warn!("If this is a brand new repo, remember to initialise it with `sprout repo init {}`", &args.label);

                Ok(CliResponse {
                    msg: format!(
                        "Created repository definition at {}",
                        &repo_file.to_string_lossy()
                    ),
                    data: Some(serde_json::to_string(&definition)?),
                })
            }
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

                let (definition_path, mut definition) = Repositories::get(&args.label)?;

                let access_key = match args.access_key {
                    Some(access_key) => access_key,
                    None => {
                        if definition.access_key == "" {
                            Input::with_theme(&CliTheme::default())
                                .with_prompt("Please set a secure access key for this repository.")
                                .default(generated_access_key.to_string())
                                .interact_text()
                                .unwrap()
                        } else {
                            definition.access_key
                        }
                    }
                };

                let progress = SproutProgressBar {};
                let spinner =
                    progress.progress_spinner(format!("Initialising repository {}", &args.label));

                let repo_opts = RepositoryOptions::default().password(&access_key);

                let repo = crate::repo::open_repo(&definition.repo, repo_opts)?;
                let repo = crate::repo::initialise(repo)?;

                spinner.finish();

                definition.access_key = access_key;

                Repositories::save(&definition, &definition_path)?;

                info!("Sprout repo created at {}", &args.label);

                return Ok(CliResponse {
                    msg: "Sprout repository initialised".to_string(),
                    data: Some(serde_json::to_string(&repo)?),
                });
            }
        },

        SubCommand::Snap(args) => {
            let mut project = Project::new(options.path.to_owned())?;

            project.print_header();

            project.determine_home_url()?;

            let (_, definition) = Repositories::get(&project.config.repo)?;

            let repo = project.open_repo(definition.access_key)?;

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
            let mut project = Project::new(options.path.to_owned())?;

            project.print_header();

            project.determine_home_url()?;

            let (_, definition) = Repositories::get(&project.config.repo)?;

            if !args.no_stash {
                warn!("This command is destructive. Stashing your database and uploads locally.");
                let stash = Stash::new(sprout_home.join("stash"))?;
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

            let repo = project.open_repo(definition.access_key)?;

            let snap_id = project.get_active_snapshot_id(&repo)?;

            crate::repo::restore(repo, &project, snap_id)?;

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

            let stash = Stash::new(sprout_home.join("stash"))?;

            let snap_id = match args.snapshot_id {
                Some(id) => Id::from_hex(&id)?,
                None => stash.get_latest_stash(&project)?.id,
            };

            stash.restore(&project, snap_id)?;

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
