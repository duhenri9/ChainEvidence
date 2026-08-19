use chain_evidence::{clear_chain, connect, persist_fixture, recover, Fixture, PersistMode};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Serialize)]
struct EvidencePack {
    schema: &'static str,
    persistence: chain_evidence::PersistenceEvidence,
    recovery: chain_evidence::RecoveryEvidence,
    pack_sha256: String,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("chain-evidence-persistence: {error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let input = PathBuf::from(
        args.next()
            .ok_or("usage: persistence_evidence <fixture.json> <report.json>")?,
    );
    let output = PathBuf::from(
        args.next()
            .ok_or("usage: persistence_evidence <fixture.json> <report.json>")?,
    );
    if args.next().is_some() {
        return Err("usage: persistence_evidence <fixture.json> <report.json>".into());
    }

    let database_url = env::var("DATABASE_URL")?;
    let fixture: Fixture = serde_json::from_slice(&fs::read(input)?)?;

    let mut client = connect(&database_url)?;
    clear_chain(&mut client, fixture.chain_id)?;
    let persistence = persist_fixture(&mut client, &fixture, PersistMode::Commit)?;
    drop(client);

    let mut restarted = connect(&database_url)?;
    let recovery = recover(&mut restarted, fixture.chain_id)?;

    let unsigned = serde_json::json!({
        "schema": "chain-evidence.persistence-pack.v0.2",
        "persistence": &persistence,
        "recovery": &recovery,
    });
    let bytes = serde_json::to_vec(&unsigned)?;
    let pack_sha256 = format!("{:x}", Sha256::digest(bytes));
    let pack = EvidencePack {
        schema: "chain-evidence.persistence-pack.v0.2",
        persistence,
        recovery,
        pack_sha256,
    };

    let rendered = serde_json::to_vec_pretty(&pack)?;
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(&output, &rendered)?;
    println!("{}", String::from_utf8_lossy(&rendered));
    Ok(())
}
