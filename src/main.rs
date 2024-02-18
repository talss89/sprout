use homedir::get_my_home;
use sprout::engine::Engine;
use std::env;
use std::path::PathBuf;

#[cfg(feature = "markdown-docs")]
fn main() {
    clap_markdown::print_help_markdown::<Options>();
}

#[cfg(not(feature = "markdown-docs"))]
fn main() {
    let sprout_home = match env::var("SPROUT_HOME") {
        Err(_) => get_my_home().unwrap().unwrap().as_path().join(".sprout"),
        Ok(home) => PathBuf::from(home),
    };

    sprout::cli::entrypoint(&Engine { sprout_home })
}
