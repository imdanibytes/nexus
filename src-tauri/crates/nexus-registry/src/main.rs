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
    Plugin {
        /// Plugin ID in reverse-domain format (e.g. com.example.my-plugin)
        #[arg(long)]
        id: Option<String>,
        /// Display name
        #[arg(long)]
        name: Option<String>,
        /// Semver version (e.g. 0.2.0)
        #[arg(long)]
        version: Option<String>,
        /// Short description
        #[arg(long)]
        description: Option<String>,
        /// Author GitHub username
        #[arg(long)]
        author: Option<String>,
        /// Author profile URL
        #[arg(long)]
        author_url: Option<String>,
        /// SPDX license identifier
        #[arg(long)]
        license: Option<String>,
        /// Project homepage URL
        #[arg(long)]
        homepage: Option<String>,
        /// Docker image reference
        #[arg(long)]
        image: Option<String>,
        /// Raw manifest URL (plugin.json)
        #[arg(long)]
        manifest_url: Option<String>,
        /// Comma-separated categories
        #[arg(long)]
        categories: Option<String>,
        /// Package status: active, deprecated, or unlisted
        #[arg(long, default_value = "active")]
        status: String,
    },
    /// Add a new extension
    Extension {
        /// Extension ID in reverse-domain format
        #[arg(long)]
        id: Option<String>,
        /// Display name
        #[arg(long)]
        name: Option<String>,
        /// Semver version
        #[arg(long)]
        version: Option<String>,
        /// Short description
        #[arg(long)]
        description: Option<String>,
        /// Author GitHub username
        #[arg(long)]
        author: Option<String>,
        /// Author profile URL
        #[arg(long)]
        author_url: Option<String>,
        /// SPDX license identifier
        #[arg(long)]
        license: Option<String>,
        /// Project homepage URL
        #[arg(long)]
        homepage: Option<String>,
        /// Raw manifest URL (manifest.json)
        #[arg(long)]
        manifest_url: Option<String>,
        /// Comma-separated platform targets
        #[arg(long)]
        platforms: Option<String>,
        /// Comma-separated categories
        #[arg(long)]
        categories: Option<String>,
        /// Package status: active, deprecated, or unlisted
        #[arg(long, default_value = "active")]
        status: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path } => commands::init::run(&path),
        Commands::Validate { path } => commands::validate::run(&path),
        Commands::Build { path } => commands::build::run(&path),
        Commands::Add { kind } => match kind {
            AddKind::Plugin {
                id,
                name,
                version,
                description,
                author,
                author_url,
                license,
                homepage,
                image,
                manifest_url,
                categories,
                status,
            } => commands::add::run_plugin(commands::add::PluginArgs {
                id,
                name,
                version,
                description,
                author,
                author_url,
                license,
                homepage,
                image,
                manifest_url,
                categories,
                status,
            }),
            AddKind::Extension {
                id,
                name,
                version,
                description,
                author,
                author_url,
                license,
                homepage,
                manifest_url,
                platforms,
                categories,
                status,
            } => commands::add::run_extension(commands::add::ExtensionArgs {
                id,
                name,
                version,
                description,
                author,
                author_url,
                license,
                homepage,
                manifest_url,
                platforms,
                categories,
                status,
            }),
        },
        Commands::Publish { registry, package } => {
            commands::publish::run(&registry, &package)
        }
    }
}
