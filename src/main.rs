use chain_evidence::{report_fixture, Fixture};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("chain-evidence: {error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: chain-evidence <fixture.json> <report.json>")?);
    let output = PathBuf::from(args.next().ok_or("usage: chain-evidence <fixture.json> <report.json>")?);
    if args.next().is_some() {
        return Err("usage: chain-evidence <fixture.json> <report.json>".into());
    }

    let fixture: Fixture = serde_json::from_slice(&fs::read(input)?)?;
    let report = report_fixture(&fixture);
    let encoded = serde_json::to_vec_pretty(&report)?;
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(output, &encoded)?;
    println!("{}", String::from_utf8_lossy(&encoded));

    if report.outcome == "PASS" {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(3))
    }
}
