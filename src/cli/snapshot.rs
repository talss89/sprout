use colored::*;
use indicatif::HumanBytes;
use std::io::Write;
use tabwriter::TabWriter;

use crate::{project::Project, snapshot::Snapshot};
/// Generates a table showing all snapshots passed in
pub fn project_table(
    snapshots: &Vec<Snapshot>,
    project: Option<&Project>,
) -> anyhow::Result<String> {
    let mut tw = TabWriter::new(vec![]).ansi(true);

    write!(
        &mut tw,
        "\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
        "Label".dimmed().bold(),
        "ID".dimmed().bold(),
        "Branch".dimmed().bold(),
        "Files".dimmed().bold(),
        "New".dimmed().italic().cyan(),
        "Change".dimmed().italic().cyan(),
        "Same".dimmed().italic().cyan(),
        "Size".dimmed().bold(),
        "+Diff".dimmed().italic().cyan(),
        "Date / Time".dimmed().bold()
    )?;

    for stash in snapshots {
        let stats = stash.get_stats();
        write!(
            &mut tw,
            "{}",
            format!(
                "{:^8}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                match project {
                    Some(project) => match project.config.snapshot {
                        None => "".normal(),
                        Some(snapshot) => {
                            if snapshot == stash.id {
                                "active ▶".green().dimmed().bold()
                            } else {
                                "".normal()
                            }
                        }
                    },
                    None => "".normal(),
                },
                stash.get_label(),
                stash.id.to_hex().to_string(),
                stash.get_branch().unwrap_or("???".to_string()),
                stash.get_total_files(),
                match &stats {
                    Ok(stats) => format!(
                        "{}\t{}\t{}",
                        stats.new.to_string().green(),
                        stats.changed.to_string().blue(),
                        stats.unmodified.to_string().yellow()
                    )
                    .dimmed()
                    .italic(),
                    Err(_) => "(Err)".to_string().red(),
                },
                format!(
                    "{}\t{}",
                    HumanBytes(stash.get_total_bytes()),
                    match &stats {
                        Ok(stats) => format!("(+{})", HumanBytes(stats.data_added)).dimmed(),
                        Err(_) => "(Err)".to_string().red(),
                    }
                ),
                stash.snapshot.time
            )
            .normal()
        )?;
    }

    tw.flush().unwrap();

    Ok(String::from_utf8(tw.into_inner().unwrap()).unwrap())
}

pub fn snapshot_describe(snapshot: Snapshot) -> anyhow::Result<String> {
    let mut tw = TabWriter::new(vec![]).ansi(true);

    write!(
        &mut tw,
        "{}\t{}\n",
        "ID:".dimmed(),
        snapshot.id.to_hex().to_string()
    )?;
    write!(
        &mut tw,
        "{}\t{}\n",
        "Project:".dimmed(),
        snapshot.get_project_name()
    )?;
    write!(&mut tw, "{}\t{}\n", "Label:".dimmed(), snapshot.get_label())?;
    write!(
        &mut tw,
        "{}\t{}\n",
        "Branch:".dimmed(),
        snapshot.get_branch()?
    )?;
    write!(
        &mut tw,
        "{}\t{}\n",
        "File Count:".dimmed(),
        snapshot.get_total_files()
    )?;
    write!(
        &mut tw,
        "{}\t{}\n",
        "Total Size:".dimmed(),
        HumanBytes(snapshot.get_total_bytes())
    )?;

    write!(
        &mut tw,
        "{}\n\n{}\n",
        "Description:".dimmed(),
        snapshot.get_description().unwrap_or("-None-".to_string())
    )?;
    tw.flush().unwrap();
    Ok(String::from_utf8(tw.into_inner().unwrap()).unwrap())
}
