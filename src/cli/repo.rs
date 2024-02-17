use colored::*;
use std::io::Write;
use tabwriter::TabWriter;

use crate::repo::definition::RepositoryDefinition;

pub fn definition_table(defs: &Vec<(String, RepositoryDefinition)>) -> anyhow::Result<String> {
    let sprout_config = crate::engine::get_sprout_config()?;
    let mut tw = TabWriter::new(vec![]).ansi(true);

    write!(
        &mut tw,
        "{}",
        "Default?\tRepository Label\tRepository Path\n"
            .dimmed()
            .bold()
    )?;

    for (label, definition) in defs {
        let repo = definition.repo.clone();
        if sprout_config.default_repo == *label {
            write!(
                &mut tw,
                "{}\n",
                format!(
                    "------->\t{}\t{}",
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
