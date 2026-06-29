mod fetch;
mod write;

use clap::Parser;
use color_eyre::Result;
use fetch::fetch_crate;
use write::write_skill;

/// Generate agent skills from crate documentation.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Crate to fetch, e.g. `clap` or `clap@4.5`
    crate_spec: String,

    /// Override the base output path (default: .agents/skills)
    #[arg(short, long)]
    output: Option<std::path::PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();
    let target = parse_spec(&args.crate_spec);

    let base = args
        .output
        .unwrap_or_else(|| std::path::PathBuf::from(".agents/skills"));

    let client = reqwest::Client::new();
    let info = fetch_crate(&client, &target).await?;
    write_skill(&info, &base)?;

    println!("name:        {}", info.name);
    println!("version:     {}", info.version);
    println!("license:     {}", info.license);
    println!("description: {}", info.description);
    if info.references.len() > 0 {
        println!(
            "pages:       {} ({} references)",
            info.references.len() + 1,
            info.references.len()
        );
    }
    println!("output:      {}", base.display());

    Ok(())
}

pub struct CrateTarget {
    pub name: String,
    pub version: Option<String>,
}

fn parse_spec(spec: &str) -> CrateTarget {
    match spec.split_once('@') {
        Some((name, version)) => CrateTarget {
            name: name.to_string(),
            version: Some(version.to_string()),
        },
        None => CrateTarget {
            name: spec.to_string(),
            version: None,
        },
    }
}
