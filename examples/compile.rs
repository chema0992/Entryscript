use std::{env, fs, io, process};

use entscript_converter::{compile_to_json, compile_with_project_json};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let source_path = args.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Usage: cargo run --example compile -- <source.ents> [project.json]",
        )
    })?;
    let source = fs::read_to_string(&source_path)?;

    let output = if let Some(project_path) = args.next() {
        let project_json = fs::read_to_string(project_path)?;
        compile_with_project_json(&source, &project_json)?
    } else {
        compile_to_json(&source)?
    };

    println!("{output}");
    Ok(())
}
