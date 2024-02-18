use ::clap::Parser;
use colored::*;
use dialoguer::{Confirm, Input};

use env_logger::Builder;
use log::{info, warn};
use passwords::PasswordGenerator;
use rustic_backend::BackendOptions;
use rustic_core::{ConfigOptions, Id, KeyOptions, Progress, ProgressBars, RepositoryOptions};
use std::io::Write;

use crate::{
    cli::clap::{CliResponse, Options, RepoCommand, StashCommand, SubCommand},
    engine::Engine,
    facts::wordpress::WordPress,
    progress::SproutProgressBar,
    project::Project,
    repo::{definition::RepositoryDefinition, ProjectRepository},
    stash::Stash,
    theme::CliTheme,
};

#[allow(clippy::format_in_format_args)]
pub fn run(engine: &Engine) -> anyhow::Result<CliResponse> {
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
        format!("Sprout {}", crate::PKG_VERSION).bold().green(),
        "Content and database seeding for WordPress"
            .white()
            .bold()
            .dimmed(),
        format!("{} | https://github.com/talss89/sprout", crate::TARGET)
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
                if std::env::var("SPROUT_DEBUG_RUSTIC").is_err() {
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

    let sprout_home = engine.get_home();
    engine.ensure_home()?;

    std::env::set_current_dir(&options.path)?;

    let facts = Box::new(WordPress {
        path: options.path.to_owned(),
    });

    match options.subcommand {
        SubCommand::Init => {
            info!("Creating a `sprout.yaml` for your project and opening it in the default text editor...");
            let project = Project::initialise(engine, options.path.to_owned(), facts)?;

            edit::edit_file(options.path.join("./sprout.yaml"))?;

            info!("Your project is ready.");

            Ok(CliResponse {
                msg: "Project initialised".to_string(),
                data: Some(serde_json::to_string(&project)?),
            })
        }

        SubCommand::Repo(args) => match args.subcommand {
            RepoCommand::Use(args) => {
                info!("Setting default repo to {}", &args.label);

                let mut sprout_config = engine.get_config()?;

                let (_, definition) = RepositoryDefinition::get(engine, &args.label)?;

                sprout_config.default_repo = args.label.to_owned();

                engine.write_config(&sprout_config)?;

                Ok(CliResponse {
                    msg: format!("Set default repo to {}", args.label),
                    data: Some(serde_json::to_string(&definition)?),
                })
            }
            RepoCommand::List => {
                let defs = RepositoryDefinition::list(engine)?;

                info!(
                    "Your repository definitions are stored at {}",
                    engine.get_home().join("repos").display()
                );

                info!("Listing all repository definitions");

                eprint!("\n{}", crate::cli::repo::definition_table(engine, &defs)?);

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

                RepositoryDefinition::create(&definition, &repo_file)?;

                edit::edit_file(&repo_file)?;

                let mut sprout_config = engine.get_config()?;

                if sprout_config.default_repo.is_empty() {
                    info!("Setting default repo to {}", &args.label);

                    sprout_config.default_repo = args.label.to_owned();

                    engine.write_config(&sprout_config)?;
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

                let (definition_path, mut definition) =
                    RepositoryDefinition::get(engine, &args.label)?;

                let access_key = match args.access_key {
                    Some(access_key) => access_key,
                    None => {
                        if definition.access_key.is_empty() {
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

                let _ = ProjectRepository::initialise(
                    definition.repo.clone(),
                    repo_opts,
                    KeyOptions::default(),
                    ConfigOptions::default(),
                )?;

                spinner.finish();

                definition.access_key = access_key;

                RepositoryDefinition::save(&definition, &definition_path)?;

                info!("Sprout repo created at {}", &args.label);

                Ok(CliResponse {
                    msg: "Sprout repository initialised".to_string(),
                    data: Some(serde_json::to_string(&definition)?),
                })
            }
        },

        SubCommand::Snap(args) => {
            let mut project = Project::new(engine, options.path.to_owned(), facts)?;

            project.print_header();

            project.determine_home_url()?;

            let (_, definition) = RepositoryDefinition::get(engine, &project.config.repo)?;

            let repo = project.open_repo(&definition.access_key)?;

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

            let latest_hash = repo.get_latest_unique_hash()?;

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

            let snapshot = repo.snapshot(false)?;

            project.update_snapshot_id(snapshot.id, project.config.branch.to_owned())?;

            Ok(CliResponse {
                msg: "Snapshot created".to_string(),
                data: Some(serde_json::to_string(&snapshot.id)?),
            })
        }

        SubCommand::Seed(args) => {
            let mut project = Project::new(engine, options.path.to_owned(), facts)?;

            project.print_header();

            project.determine_home_url()?;

            let (_, definition) = RepositoryDefinition::get(engine, &project.config.repo)?;

            if !args.no_stash {
                warn!("This command is destructive. Stashing your database and uploads locally.");
                let stash = Stash::new(engine, engine.get_stash_path())?;
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

            let repo = project.open_repo(&definition.access_key)?;

            let snapshot = project.get_active_snapshot(&repo)?;

            project.restore_from_snapshot(&repo, &snapshot)?;

            Ok(CliResponse {
                msg: "Content and database seeded".to_string(),
                data: Some(serde_json::to_string(&project)?),
            })
        }

        SubCommand::Ls => {
            let project = Project::new(engine, options.path.to_owned(), facts)?;

            project.print_header();

            info!(
                "Listing all snapshots for the current project ({})",
                project.config.name
            );

            let (_, definition) = RepositoryDefinition::get(engine, &project.config.repo)?;

            let repo = project.open_repo(&definition.access_key)?;

            let (snapshots, errors) = project.get_all_snapshots(&repo)?;

            for err in errors {
                warn!("{}", err);
            }

            eprint!("\n{}", crate::cli::snapshot::project_table(&snapshots)?);

            Ok(CliResponse {
                msg: format!(
                    "Listed all snapshots for {} on {} - {}",
                    project.config.name,
                    project.config.repo,
                    RepositoryDefinition::display_path(&definition)?
                ),
                data: Some(serde_json::to_string(&snapshots)?),
            })
        }

        SubCommand::UnStash(args) => {
            let project = Project::new(engine, options.path.to_owned(), facts)?;

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

            let stash = Stash::new(engine, engine.get_stash_path())?;

            let snap_id = match args.snapshot_id {
                Some(id) => Id::from_hex(&id)?,
                None => stash.get_latest_stash(&project)?.id,
            };

            stash.restore(&project, snap_id)?;

            Ok(CliResponse {
                msg: format!(
                    "Restored the stash of {} ({})",
                    project.config.name, &snap_id
                )
                .to_string(),
                data: None,
            })
        }

        SubCommand::Stash(args) => match args.subcommand {
            None => {
                let mut project = Project::new(engine, options.path.to_owned(), facts)?;

                project.print_header();

                project.determine_home_url()?;

                let stash = Stash::new(engine, engine.get_stash_path())?;
                stash.stash(&project)?;

                Ok(CliResponse {
                    msg: "Created new stash".to_string(),
                    data: None,
                })
            }
            Some(subcommand) => match subcommand {
                StashCommand::List => {
                    let project = Project::new(engine, options.path.to_owned(), facts)?;

                    project.print_header();

                    let stash = Stash::new(engine, engine.get_stash_path())?;

                    let (stashes, errors) = stash.get_all_stashes_for_project(&project)?;

                    for err in errors {
                        warn!("{}", err);
                    }

                    info!(
                        "Listing all stashes for the {} project",
                        project.config.name
                    );

                    eprint!("\n{}", crate::cli::snapshot::project_table(&stashes)?);

                    Ok(CliResponse {
                        msg: format!("Listed all local stashes for {}", project.config.name),
                        data: Some(serde_json::to_string(&stashes)?),
                    })
                }
                StashCommand::Drop(args) => {
                    let stash = Stash::new(engine, engine.get_stash_path())?;

                    let snapshot = stash.get_stash_by_id(Id::from_hex(&args.snapshot_id)?)?;

                    info!("Stash snapshot found: {}", snapshot.id);

                    info!(
                        "Stashed snapshot is for the {} branch on the {} project",
                        snapshot.get_branch()?,
                        snapshot.get_project_name()
                    );

                    let confirmation = Confirm::with_theme(&CliTheme::default())
                        .with_prompt("Are you sure you want to drop this stashed snapshot?")
                        .interact()
                        .unwrap();

                    if !confirmation {
                        return Ok(CliResponse {
                            msg: "Aborted by user, but no error".to_string(),
                            data: None,
                        });
                    }

                    stash.drop(Id::from_hex(&args.snapshot_id)?)?;

                    Ok(CliResponse {
                        msg: "Dropped the stashed snapshot".to_string(),
                        data: None,
                    })
                }
            },
        },
    }
}
