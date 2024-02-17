use colored::*;
use indicatif::HumanBytes;
use std::io::Write;
use tabwriter::TabWriter;

use crate::snapshot::Snapshot;

pub fn project_table(snapshots: &Vec<Snapshot>) -> anyhow::Result<String> {
    let mut tw = TabWriter::new(vec![]).ansi(true);

    write!(
        &mut tw,
        "{}",
        "ID\tBranch\tFiles (New/Changed/Unchanged)\tSize\tDate / Time\n"
            .dimmed()
            .bold()
    )?;

    for stash in snapshots {
        write!(
            &mut tw,
            "{}",
            format!(
                "{}\t{}\t{} {}\t{}\t{}\n",
                stash.id.to_hex().to_string(),
                stash.get_branch().unwrap_or("???".to_string()),
                stash.get_total_files(),
                format!(
                    "({}/{}/{})",
                    stash.get_files_new().to_string().green(),
                    stash.get_files_changed().to_string().blue(),
                    stash.get_files_unmodified().to_string().yellow()
                )
                .dimmed(),
                format!(
                    "{} {}",
                    HumanBytes(stash.get_total_bytes()),
                    format!("(+{})", HumanBytes(stash.get_data_added())).dimmed()
                ),
                stash.db_snapshot.time
            )
            .normal()
        )?;
    }

    tw.flush().unwrap();

    Ok(String::from_utf8(tw.into_inner().unwrap()).unwrap())
}
