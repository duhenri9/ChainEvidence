use chain_evidence::{collect_evm_evidence, write_evm_report, EvmAdapterConfig, REFERENCE_DECODER_ID};
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn value(args: &[String], flag: &str) -> Result<String, String> {
    let index = args
        .iter()
        .position(|item| item == flag)
        .ok_or_else(|| format!("missing required argument {flag}"))?;
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn usage() -> &'static str {
    "usage: evm_evidence --rpc-url <loopback-url> --expected-chain-id <u64> --contract <address> --deployment-tx <hash> --out <path> [--decoder-id <id>] [--abi <path>] [--source <path>]"
}

fn run() -> Result<ExitCode, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|item| item == "--help" || item == "-h") {
        println!("{}", usage());
        return Ok(ExitCode::SUCCESS);
    }

    let expected_chain_id = value(&args, "--expected-chain-id")?
        .parse::<u64>()
        .map_err(|error| format!("invalid --expected-chain-id: {error}"))?;
    let config = EvmAdapterConfig {
        rpc_url: value(&args, "--rpc-url")?,
        expected_chain_id,
        contract_address: value(&args, "--contract")?,
        deployment_tx_hash: value(&args, "--deployment-tx")?,
        decoder_id: args
            .iter()
            .position(|item| item == "--decoder-id")
            .and_then(|index| args.get(index + 1))
            .cloned()
            .unwrap_or_else(|| REFERENCE_DECODER_ID.to_owned()),
        abi_path: args
            .iter()
            .position(|item| item == "--abi")
            .and_then(|index| args.get(index + 1))
            .map_or_else(
                || PathBuf::from("evm/abi/ReferenceLifecycle.v1.json"),
                PathBuf::from,
            ),
        source_path: args
            .iter()
            .position(|item| item == "--source")
            .and_then(|index| args.get(index + 1))
            .map_or_else(
                || PathBuf::from("evm/src/ReferenceLifecycle.sol"),
                PathBuf::from,
            ),
    };
    let output = PathBuf::from(value(&args, "--out")?);
    let report = collect_evm_evidence(&config);
    write_evm_report(&output, &report)
        .map_err(|error| format!("failed to write evidence report: {error}"))?;
    println!(
        "outcome={} chain={} events={} blocks={} report_sha256={}",
        report.outcome,
        report
            .observed_chain_id
            .map_or_else(|| "unknown".to_owned(), |value| value.to_string()),
        report.adapted_event_count,
        report.observed_block_count,
        report.report_sha256
    );
    if let Some(code) = &report.error_code {
        eprintln!("error_code={code}");
    }
    Ok(if report.outcome == "PASS" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(3)
    })
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            eprintln!("{}", usage());
            ExitCode::from(2)
        }
    }
}
