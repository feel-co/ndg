use std::{fs, path::Path};

use anyhow::{Context, Result};
use clap_complete::{generate_to, shells};
use clap_mangen::Man;

use crate::cli::Cli;

/// Generates shell completions for the ndg CLI.
pub fn generate_comp<P: AsRef<Path>>(output_dir: P) -> Result<()> {
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)?;

    let mut cmd = Cli::command();

    // Generate completions for various shells
    generate_to(shells::Bash, &mut cmd, "ndg", output_dir)?;
    generate_to(shells::Zsh, &mut cmd, "ndg", output_dir)?;
    generate_to(shells::Fish, &mut cmd, "ndg", output_dir)?;
    generate_to(shells::PowerShell, &mut cmd, "ndg", output_dir)?;

    Ok(())
}

/// Generates a manpage for the ndg CLI.
pub fn generate_manpage<P: AsRef<Path>>(output_dir: P) -> Result<()> {
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)?;

    let cmd = Cli::command();
    let man = Man::new(cmd);

    let file_path = output_dir.join("ndg.1");
    let mut file = fs::File::create(&file_path)
        .with_context(|| format!("Failed to create manpage file at {}", file_path.display()))?;

    man.render(&mut file)
        .with_context(|| "Failed to render manpage")?;

    Ok(())
}

/// Generates both shell completions and a manpage for the ndg CLI.
pub fn generate_all<P: AsRef<Path>>(output_dir: P) -> Result<()> {
    let output_dir = output_dir.as_ref();

    let completions_dir = output_dir.join("completions");
    generate_comp(&completions_dir)?;
    println!(
        "Shell completions generated in {}",
        completions_dir.display()
    );

    let manpage_dir = output_dir.join("man");
    generate_manpage(&manpage_dir)?;
    println!("Manpage generated in {}", manpage_dir.display());

    Ok(())
}
