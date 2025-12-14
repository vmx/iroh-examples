use std::path::PathBuf;

use anyhow::Result;
use blobs_wasm::BlobsNode;
use clap::Parser;

#[derive(Debug, Parser)]
struct Args {
    path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let node = BlobsNode::spawn().await?;
    let ticket = node.import(&args.path).await?;
    println!("ticket:");
    println!("{ticket}");
    println!();
    println!("providing... press Ctrl-C to abort");
    tokio::signal::ctrl_c().await?;

    Ok(())
}
