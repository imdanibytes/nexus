mod commands;
mod schema;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "nexus-registry")]
#[command(about = "CLI tool for managing Nexus plugin/extension registries")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scaffold a new registry directory
    Init {
        /// Path to create the registry in (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Validate all YAML files against schemas
    Validate {
        /// Path to the registry root (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Compile YAML files into index.json
    Build {
        /// Path to the registry root (default: current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Interactively add a new plugin or extension
    Add {
        #[command(subcommand)]
        kind: AddKind,
    },
    /// Publish a package to a remote registry
    Publish {
        /// Git URL of the target registry
        #[arg(long)]
        registry: String,
        /// Path to the local package YAML file
        #[arg(long)]
        package: PathBuf,
    },
}

#[derive(Subcommand)]
enum AddKind {
    /// Add a new plugin
    Plugin,
    /// Add a new extension
    Extension,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path } => commands::init::run(&path),
        Commands::Validate { path } => commands::validate::run(&path),
        Commands::Build { path } => commands::build::run(&path),
        Commands::Add { kind } => match kind {
            AddKind::Plugin => commands::add::run_plugin(),
            AddKind::Extension => commands::add::run_extension(),
        },
        Commands::Publish { registry, package } => {
            commands::publish::run(&registry, &package)
        }
    }
}
