use std::path::PathBuf;

use clap::{Parser, Subcommand};
use mcconvert_core::pipeline;
use mcconvert_core::version;
use mcconvert_core::world::layout::Layout;
use mcconvert_core::Error;

#[derive(Parser)]
#[command(
    name = "mcconvert",
    version,
    about = "Converts Minecraft: Java Edition 26.x worlds into a format older versions (1.21.x) can open.",
    after_help = "The original world is never modified; results are written to a new folder.\n\
                  Always back up your world and test the result with a copy first.\n\n\
                  Error codes:\n  \
                  E10(10) I/O  E20(20) NBT data  E30(30) world format\n  \
                  E40(40) target version  E50(50) output folder  E60(60) partial batch failure"
)]
struct Cli {
    #[arg(short, long, global = true, help = "Suppress progress logs")]
    quiet: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Convert every world in the input folder at once")]
    Batch {
        #[arg(help = "Target version — run `mcconvert versions` for the list")]
        target: String,
        #[arg(long, default_value = "input_worlds", help = "Input folder")]
        input_dir: PathBuf,
        #[arg(long, default_value = "output_worlds", help = "Output folder")]
        output_dir: PathBuf,
        #[arg(long, help = "JSON block allowlist for the target version")]
        block_table: Option<PathBuf>,
    },
    #[command(about = "Show a world's version and layout information")]
    Info {
        #[arg(help = "World folder path")]
        world: PathBuf,
    },
    #[command(about = "List supported versions")]
    Versions,
}

fn main() {
    let cli = Cli::parse();
    mcconvert_core::logging::set_verbose(!cli.quiet);
    if let Err(error) = run(cli.command) {
        eprintln!("[ERROR {}] {error}", error.code());
        std::process::exit(error.exit_code());
    }
}

fn run(command: Command) -> mcconvert_core::Result<()> {
    match command {
        Command::Batch {
            target,
            input_dir,
            output_dir,
            block_table,
        } => {
            if !input_dir.is_dir() {
                std::fs::create_dir_all(&input_dir).map_err(|e| Error::io(&input_dir, e))?;
                std::fs::create_dir_all(&output_dir).map_err(|e| Error::io(&output_dir, e))?;
                println!("Created '{}'.", input_dir.display());
                println!("Put worlds (as folders) inside it and run again.");
                println!("(Worlds live in %appdata%\\.minecraft\\saves)");
                return Ok(());
            }
            std::fs::create_dir_all(&output_dir).map_err(|e| Error::io(&output_dir, e))?;

            let outcomes = pipeline::batch::convert_batch(
                &input_dir,
                &output_dir,
                &target,
                block_table.as_deref(),
            )?;
            if outcomes.is_empty() {
                println!("No worlds found in '{}'.", input_dir.display());
                println!("Put worlds in as folders (each must contain level.dat).");
                return Ok(());
            }

            let mut ok_count = 0;
            for outcome in &outcomes {
                println!();
                println!("=== {} ===", outcome.world_name);
                match &outcome.result {
                    Ok(report) => {
                        ok_count += 1;
                        println!(
                            "OK: {} region file(s), {} chunk(s) rewritten → {}",
                            report.region_files,
                            report.chunks_rewritten,
                            outcome.output.display()
                        );
                        if report.items_converted > 0 || report.dropped_data > 0 {
                            println!(
                                "Legacy NBT conversion: {} item stack(s) converted, {} data entr(ies) dropped",
                                report.items_converted, report.dropped_data
                            );
                        }
                        if report.chunks_skipped > 0 {
                            println!(
                                "Skipped {} corrupt chunk(s) — see warnings below",
                                report.chunks_skipped
                            );
                        }
                        if report.block_entities_added > 0 {
                            println!(
                                "Restored {} block entit(ies) removed by 26.x (beds etc.)",
                                report.block_entities_added
                            );
                        }
                        for warning in &report.warnings {
                            println!("  [WARN] {warning}");
                        }
                    }
                    Err(error) => println!("FAILED [{}]: {error}", error.code()),
                }
            }
            println!();
            println!(
                "Batch finished: {}/{} succeeded (target {target})",
                ok_count,
                outcomes.len()
            );
            println!("The original worlds were not modified.");
            if ok_count < outcomes.len() {
                eprintln!("[ERROR E60] Some worlds failed to convert.");
                std::process::exit(60);
            }
        }
        Command::Info { world } => {
            let info = pipeline::inspect(&world)?;
            if let Some(name) = &info.level_name {
                println!("World name  : {name}");
            }
            println!(
                "Layout      : {}",
                match info.layout {
                    Layout::New26 => "26.x structure (dimensions/)",
                    Layout::Legacy => "legacy structure (root region/ + DIM-1/ + DIM1/)",
                }
            );
            match (info.data_version, info.version_name) {
                (Some(dv), Some(name)) => println!("Game version: {name} (DataVersion {dv})"),
                (Some(dv), None) => println!("Game version: unknown (DataVersion {dv})"),
                _ => println!("Game version: DataVersion not found in level.dat"),
            }
        }
        Command::Versions => {
            println!("Version list (final patch of each minor line):");
            for v in version::VERSIONS {
                let status = if version::is_supported_target(v) {
                    "supported target"
                } else {
                    "input (source) only"
                };
                println!("  {:8} DataVersion {:5}  {status}", v.name, v.data_version);
            }
        }
    }
    Ok(())
}
