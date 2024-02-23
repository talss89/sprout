use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use std::fmt::Debug;

/// Fast, deduplicated content and database seeding for WordPress.
#[derive(Parser)]
#[command(author, about, version)]
pub struct Options {
    /// Optional path to your project. Default: current directory.
    #[arg(short, long, default_value = "./")]
    pub path: PathBuf,

    /// Output JSON on stdout - useful for CI or piping into other utilities
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
    /// Create and push new snapshot to your remote repo
    Snap(SnapArgs),
    /// Seed the database and uploads directory from a remote snapshot
    Seed(SeedArgs),
    /// Restore a locally stashed database and uploads directory
    UnStash(UnStashArgs),
    /// Stash your current database and uploads locally (see subcommands to manage your stashes)
    Stash(StashArgs),
    /// List available remote snapshots
    Ls,
    /// Update Sprout to latest release
    Update,
}

#[derive(Args, Debug)]
pub struct SnapArgs {
    /// Create a snapshot on a specific content branch
    pub branch: Option<String>,

    #[arg(short, long)]
    /// Add a label to the snapshot
    pub label: Option<String>,

    #[arg(short, long)]
    /// Add a description to the snapshot
    pub desc: Option<String>,
}

#[derive(Args, Debug)]
pub struct SeedArgs {
    /// Do not stash current database and uploads before seeding
    #[arg(short, long)]
    pub no_stash: bool,

    /// Restore a particular snapshot ID
    #[arg(index = 1)]
    pub snapshot_id: Option<String>,
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
    /// Create a new repository definition
    New(RepoNewArgs),

    /// Initialise a brand new repository
    Init(RepoInitArgs),

    /// List all repository definitions
    List,

    /// Set a default repository used when creating new projects
    Use(RepoUseArgs),
}

#[derive(Subcommand, Debug)]
pub enum StashCommand {
    /// List all local stashes for the current project
    List,

    /// Drop a particular stash by snapshot ID
    Drop(StashDropArgs),
}

#[derive(Args, Debug)]
pub struct RepoInitArgs {
    /// Repository definition label to initialise. See the readme for more information on how to connect to S3 or other remote storage providers.
    #[arg(index = 1, value_name = "LABEL")]
    pub label: String,

    /// Set the repository access key
    #[arg(short, long)]
    pub repo_key: Option<String>,
}

#[derive(Args, Debug)]
pub struct RepoNewArgs {
    /// Your new repository definition label
    #[arg(index = 1, value_name = "LABEL")]
    pub label: String,
}

#[derive(Args, Debug)]
pub struct RepoUseArgs {
    /// Your new repository definition label to use as the default
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
