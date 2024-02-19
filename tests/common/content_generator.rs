use std::{fs, path::PathBuf};

use rand::distributions::Alphanumeric;
use rand::{prelude::*, thread_rng, Rng};

pub fn generate_random_uploads(path: &PathBuf, size_limit: u64) -> anyhow::Result<()> {
    let exts = vec!["jpg", "png", "pdf"];
    let mut sz = 0u64;
    let min_sz = 1024 * 100;
    let max_sz = 1024 * 2048;

    if !path.exists() {
        fs::create_dir_all(path)?;
    }

    while sz < size_limit {
        let filename = format!(
            "{}-{}.{}",
            thread_rng()
                .sample_iter(&Alphanumeric)
                .take(10)
                .map(|x| x as char)
                .collect::<String>(),
            sz,
            exts.choose(&mut rand::thread_rng()).unwrap()
        );

        let gen_size = rand::thread_rng().gen_range(min_sz..max_sz);
        sz = sz + gen_size;

        if sz <= size_limit {
            fs::write(
                path.join(filename),
                format!("{:0width$}", "", width = gen_size as usize),
            )?;
        }
    }

    Ok(())
}
