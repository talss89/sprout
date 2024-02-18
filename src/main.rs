#[cfg(feature = "markdown-docs")]
fn main() {
    clap_markdown::print_help_markdown::<Options>();
}

#[cfg(not(feature = "markdown-docs"))]
fn main() {
    sprout::cli::entrypoint()
}
