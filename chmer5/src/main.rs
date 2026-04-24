use std::{fs, path::PathBuf, process::ExitCode};

use anyhow::Context;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "chmer", version, about = "CHMER 5 - Chess Machine Engine Runtime v5")]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run a CHMER program (.ch)
    Run { file: PathBuf },
    /// Analyze a CHMER program for syntax/runtime entry safety
    Analyze { file: PathBuf },
    /// Build a CHMER program (bytecode artifact) [WIP]
    Build { file: PathBuf },
    /// Start interactive REPL [WIP]
    Repl,
    /// Compile a CTL module (.ctl) to a CHMER module artifact [WIP]
    Module { file: PathBuf },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if cli.cmd.is_none() {
        print_banner();
        println!("Type `chmer help` or `chmer run <file.ch>`");
        return ExitCode::SUCCESS;
    }

    match cli.cmd.unwrap() {
        Cmd::Run { file } => match cmd_run(file) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{e:?}");
                ExitCode::from(1)
            }
        },
        Cmd::Analyze { file } => match cmd_analyze(file) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{e:?}");
                ExitCode::from(1)
            }
        },
        Cmd::Build { file: _ } => {
            eprintln!("build: WIP");
            ExitCode::from(2)
        }
        Cmd::Repl => {
            eprintln!("repl: WIP");
            ExitCode::from(2)
        }
        Cmd::Module { file: _ } => {
            eprintln!("module: WIP");
            ExitCode::from(2)
        }
    }
}

fn print_banner() {
    println!("CHMER 5 (c) HSR-projects  LGPL license");
}

fn cmd_run(file: PathBuf) -> anyhow::Result<()> {
    let src = fs::read_to_string(&file).with_context(|| format!("read {}", file.display()))?;
    if file.extension().and_then(|s| s.to_str()) != Some("ch") {
        anyhow::bail!("CHMER source files must use .ch extension");
    }
    let chunk = chmer5_compiler::compile_chmer(&file.display().to_string(), &src)?;
    let mut vm = chmer5_vm::Vm::new();
    let _ = vm.run(chunk)?;
    Ok(())
}

fn cmd_analyze(file: PathBuf) -> anyhow::Result<()> {
    let src = fs::read_to_string(&file).with_context(|| format!("read {}", file.display()))?;
    if file.extension().and_then(|s| s.to_str()) != Some("ch") {
        anyhow::bail!("CHMER source files must use .ch extension");
    }

    // Compile-only analysis pass for syntax + bytecode validity.
    let _chunk = chmer5_compiler::compile_chmer(&file.display().to_string(), &src)?;

    let line_count = src.lines().count();
    let imports = src.matches("(#import)").count();
    let funcs = src.matches("func ").count();
    let semicolons = src.matches(';').count();

    println!("CHMER Analyze Report");
    println!("file: {}", file.display());
    println!("lines: {}", line_count);
    println!("imports: {}", imports);
    println!("functions: {}", funcs);
    println!("statements (approx): {}", semicolons);
    println!("status: OK");
    Ok(())
}
