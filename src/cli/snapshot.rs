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
        let stats = stash.get_stats();
        write!(
            &mut tw,
            "{}",
            format!(
                "{}\t{}\t{} {}\t{}\t{}\n",
                stash.id.to_hex().to_string(),
                stash.get_branch().unwrap_or("???".to_string()),
                stash.get_total_files(),
                match &stats {
                    Ok(stats) => format!(
                        "({}/{}/{})",
                        stats.new.to_string().green(),
                        stats.changed.to_string().blue(),
                        stats.unmodified.to_string().yellow()
                    )
                    .dimmed(),
                    Err(_) => "(Err)".to_string().red(),
                },
                format!(
                    "{} {}",
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
