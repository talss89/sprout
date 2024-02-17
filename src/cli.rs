use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use std::fmt::Debug;

/// Content and database seeding for WordPress
#[derive(Parser)]
#[command(author, about, version)]
pub struct Options {
    /// Path to your project
    #[arg(short, long, default_value = "./")]
    pub path: PathBuf,

    /// Output JSON on stdout, useful for CI
    #[arg(short, long)]
    pub json: bool,

    #[clap(subcommand)]
    pub subcommand: SubCommand,
}

#[derive(Subcommand, Debug)]
pub enum SubCommand {
    /// Initialise a new project and create a sprout.yaml
    Init,
    /// Remote repository commands
    Repo(RepoArgs),
    /// Create a new snapshot
    Snap(SnapArgs),
    /// Seed the database and uploads directory from a snapshot
    Seed(SeedArgs),
    /// Restore a locally stashed database and uploads directory
    UnStash(UnStashArgs),
    /// Manage your stashes
    Stash(StashArgs),
}

#[derive(Args, Debug)]
pub struct SnapArgs {
    /// Create a snapshot on a new named content branch
    pub branch: Option<String>,
}

#[derive(Args, Debug)]
pub struct SeedArgs {
    /// Do not stash
    #[arg(short, long)]
    pub no_stash: bool,
}

#[derive(Args, Debug)]
pub struct RepoArgs {
    #[clap(subcommand)]
    pub subcommand: RepoCommand,
}

#[derive(Args, Debug)]
pub struct StashArgs {
    #[clap(subcommand)]
    pub subcommand: Option<StashCommand>,
}

#[derive(Subcommand, Debug)]
pub enum RepoCommand {
    /// Initialise a new repository
    Init(RepoInitArgs),
    /// Create a new repository definition
    New(RepoNewArgs),
    /// List all repositories
    List,
    /// Set a default respoitory used when creating new projects
    Use(RepoUseArgs),
}

#[derive(Subcommand, Debug)]
pub enum StashCommand {
    List,
    Drop(StashDropArgs),
}

#[derive(Args, Debug)]
pub struct RepoInitArgs {
    /// Respository URI or path. See the readme for more information on how to connect to S3 or other remote storage providers.
    #[arg(index = 1, value_name = "LABEL")]
    pub label: String,

    /// Set the repository access key
    #[arg(short, long)]
    pub access_key: Option<String>,
}

#[derive(Args, Debug)]
pub struct RepoNewArgs {
    /// Respository URI or path. See the readme for more information on how to connect to S3 or other remote storage providers.
    #[arg(index = 1, value_name = "LABEL")]
    pub label: String,
}

#[derive(Args, Debug)]
pub struct RepoUseArgs {
    /// Respository URI or path. See the readme for more information on how to connect to S3 or other remote storage providers.
    #[arg(index = 1, value_name = "LABEL")]
    pub label: String,
}

#[derive(Args, Debug)]
pub struct UnStashArgs {
    /// Restore a particular stash snapshot by ID. This will not check project or branch constraints - use with caution.
    #[arg(index = 1)]
    pub snapshot_id: Option<String>,
}

#[derive(Args, Debug)]
pub struct StashDropArgs {
    /// Drop a particular stash snapshot by ID. This will not check project or branch constraints - use with caution.
    #[arg(index = 1)]
    pub snapshot_id: String,
}

pub struct CliResponse {
    pub msg: String,
    pub data: Option<String>,
}
