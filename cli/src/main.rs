use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "program-registry")]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: CliCommands,
}

#[derive(Debug, Subcommand)]
pub enum CliCommands {
    #[command(arg_required_else_help = true)]
    Upload {
        #[arg(short, long)]
        file_path: PathBuf,
    },

    #[command(arg_required_else_help = true)]
    Download {
        #[arg(short, long)]
        program_hash: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        CliCommands::Upload { file_path } => {
        //     let program_hash = program_registry::upload(file_path).await?;
        //     println!("Program hash: {}", program_hash);
        // }
        // CliCommands::Download { program_hash } => {
        //     let body = reqwest::get("https://www.rust-lang.org")
        //         .await?
        //         .text()
        //         .await?;
        // }
    }
}
