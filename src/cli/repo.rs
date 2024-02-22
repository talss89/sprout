use colored::*;
use std::io::Write;
use tabwriter::TabWriter;

use crate::{engine::Engine, repo::definition::RepositoryDefinition};

pub fn definition_table(
    engine: &Engine,
    defs: &Vec<(String, RepositoryDefinition)>,
) -> anyhow::Result<String> {
    let sprout_config = engine.get_config()?;
    let mut tw = TabWriter::new(vec![]).ansi(true);

    write!(
        &mut tw,
        "{}",
        "\tRepository Label\tRepository Path\n".dimmed().bold()
    )?;

    for (label, definition) in defs {
        let repo = definition.repo.clone();
        if sprout_config.default_repo == *label {
            write!(
                &mut tw,
                "{}\n",
                format!(
                    "{:^8}\t{}\t{}",
                    "deflt. ▶".green().dimmed(),
                    label,
                    format!("{}", RepositoryDefinition::display_path(definition)?)
                )
                .bold()
            )?;
        } else {
            write!(
                &mut tw,
                "{}\n",
                format!(
                    " \t{}\t{}",
                    label,
                    repo.repository.unwrap_or("".to_string())
                )
            )?;
        }
    }

    tw.flush().unwrap();

    Ok(String::from_utf8(tw.into_inner().unwrap()).unwrap())
}
