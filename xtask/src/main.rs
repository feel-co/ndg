use std::{
  fs,
  path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate_to, shells};
use clap_mangen::Man;

#[derive(Parser)]
#[command(author, version, about)]
struct Xtask {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  /// Build distribution artifacts for the ndg CLI
  Dist {
    /// Output directory for generated files.
    #[arg(short, long, default_value = "dist", value_parser = clap::value_parser!(std::path::PathBuf))]
    output_dir: PathBuf,

    /// Only generate shell completions.
    #[arg(long, conflicts_with = "manpage_only")]
    completions_only: bool,

    /// Only generate manpage.
    #[arg(long, conflicts_with = "completions_only")]
    manpage_only: bool,
  },
}

fn main() -> Result<()> {
  let xtask = Xtask::parse();

  match xtask.command {
    Commands::Dist {
      output_dir,
      completions_only,
      manpage_only,
    } => {
      if completions_only {
        generate_completions(&output_dir)?;
      } else if manpage_only {
        generate_manpage(&output_dir)?;
      } else {
        generate_completions(&output_dir)?;
        generate_manpage(&output_dir)?;
      }
    },
  }

  Ok(())
}

/// Generate shell completions for various shells.
fn generate_completions(output_dir: &Path) -> Result<()> {
  let completions_dir = output_dir.join("completions");
  fs::create_dir_all(&completions_dir)?;
  let mut cmd = ndg::cli::Cli::command();
  generate_to(shells::Bash, &mut cmd, "ndg", &completions_dir)?;
  generate_to(shells::Zsh, &mut cmd, "ndg", &completions_dir)?;
  generate_to(shells::Fish, &mut cmd, "ndg", &completions_dir)?;
  generate_to(shells::PowerShell, &mut cmd, "ndg", &completions_dir)?;
  println!(
    "Shell completions generated in {}",
    completions_dir.display()
  );
  Ok(())
}

/// Generate manpage for the NDG CLI.
fn generate_manpage(output_dir: &Path) -> Result<()> {
  let man_dir = output_dir.join("man");
  fs::create_dir_all(&man_dir)?;
  let cmd = ndg::cli::Cli::command();
  let man = Man::new(cmd);
  let file_path = man_dir.join("ndg.1");
  let mut file = fs::File::create(&file_path).with_context(|| {
    format!("Failed to create manpage file at {}", file_path.display())
  })?;
  man
    .render(&mut file)
    .with_context(|| "Failed to render manpage")?;
  println!("Manpage generated in {}", man_dir.display());
  Ok(())
}
