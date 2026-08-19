use chain_evidence::{
    clear_chain, connect, persist_fixture, recover, report_fixture, Fixture, PersistMode,
};
use std::env;

fn database_url() -> Option<String> {
    match env::var("DATABASE_URL") {
        Ok(value) => Some(value),
        Err(_) if env::var("CHAIN_EVIDENCE_REQUIRE_POSTGRES").as_deref() == Ok("1") => {
            panic!("DATABASE_URL is required by the PostgreSQL gate")
        }
        Err(_) => None,
    }
}

fn reorg_fixture(chain_id: u64) -> Fixture {
    let mut fixture: Fixture = serde_json::from_str(include_str!("../fixtures/reorg.json"))
        .expect("checked-in reorg fixture must parse");
    fixture.chain_id = chain_id;
    for block in &mut fixture.blocks {
        block.chain_id = chain_id;
    }
    fixture
}

fn pre_reorg_fixture(chain_id: u64) -> Fixture {
    let mut fixture = reorg_fixture(chain_id);
    fixture.blocks.truncate(3);
    fixture
}

#[test]
fn persist_restart_recovery_matches_core_state() {
    let Some(url) = database_url() else {
        return;
    };
    let chain_id = 42_001;
    let fixture = reorg_fixture(chain_id);
    let expected = report_fixture(&fixture);

    let mut client = connect(&url).unwrap();
    clear_chain(&mut client, chain_id).unwrap();
    let persisted = persist_fixture(&mut client, &fixture, PersistMode::Commit).unwrap();
    assert_eq!(persisted.outcome, "COMMITTED");
    drop(client);

    let mut restarted = connect(&url).unwrap();
    let recovered = recover(&mut restarted, chain_id).unwrap();
    assert_eq!(recovered.outcome, "RECOVERED");
    assert_eq!(recovered.state_sha256, expected.state_sha256);
    assert_eq!(
        recovered.canonical_block_hashes,
        expected.canonical_block_hashes
    );
    assert_eq!(recovered.material_state, expected.material_state);
}

#[test]
fn injected_pre_commit_crash_preserves_previous_checkpoint() {
    let Some(url) = database_url() else {
        return;
    };
    let chain_id = 42_002;
    let old = pre_reorg_fixture(chain_id);
    let replacement = reorg_fixture(chain_id);

    let mut client = connect(&url).unwrap();
    clear_chain(&mut client, chain_id).unwrap();
    persist_fixture(&mut client, &old, PersistMode::Commit).unwrap();
    let before = recover(&mut client, chain_id).unwrap();

    let aborted =
        persist_fixture(&mut client, &replacement, PersistMode::AbortBeforeCommit).unwrap();
    assert_eq!(aborted.outcome, "ABORTED_BEFORE_COMMIT");
    drop(client);

    let mut restarted = connect(&url).unwrap();
    let after = recover(&mut restarted, chain_id).unwrap();
    assert_eq!(after.checkpoint_tip, before.checkpoint_tip);
    assert_eq!(after.checkpoint_height, before.checkpoint_height);
    assert_eq!(after.state_sha256, before.state_sha256);
    assert_eq!(after.material_state, before.material_state);
}

#[test]
fn durable_commit_is_idempotent_after_restart() {
    let Some(url) = database_url() else {
        return;
    };
    let chain_id = 42_003;
    let fixture = reorg_fixture(chain_id);

    let mut client = connect(&url).unwrap();
    clear_chain(&mut client, chain_id).unwrap();
    persist_fixture(&mut client, &fixture, PersistMode::Commit).unwrap();
    drop(client);

    let mut restarted = connect(&url).unwrap();
    let first = recover(&mut restarted, chain_id).unwrap();
    persist_fixture(&mut restarted, &fixture, PersistMode::Commit).unwrap();
    let second = recover(&mut restarted, chain_id).unwrap();

    assert_eq!(first.checkpoint_tip, second.checkpoint_tip);
    assert_eq!(first.state_sha256, second.state_sha256);
    assert_eq!(first.canonical_event_count, second.canonical_event_count);

    let chain_id_db = i64::try_from(chain_id).unwrap();
    let block_count: i64 = restarted
        .query_one(
            "SELECT COUNT(*) FROM chain_evidence_blocks WHERE chain_id = $1",
            &[&chain_id_db],
        )
        .unwrap()
        .get(0);
    let event_count: i64 = restarted
        .query_one(
            "SELECT COUNT(*) FROM chain_evidence_events WHERE chain_id = $1",
            &[&chain_id_db],
        )
        .unwrap()
        .get(0);
    assert_eq!(block_count, 5);
    assert_eq!(event_count, 5);
}

#[test]
fn persisted_reorg_invalidates_orphaned_blocks_and_state() {
    let Some(url) = database_url() else {
        return;
    };
    let chain_id = 42_004;
    let old = pre_reorg_fixture(chain_id);
    let replacement = reorg_fixture(chain_id);

    let mut client = connect(&url).unwrap();
    clear_chain(&mut client, chain_id).unwrap();
    persist_fixture(&mut client, &old, PersistMode::Commit).unwrap();
    persist_fixture(&mut client, &replacement, PersistMode::Commit).unwrap();
    let recovered = recover(&mut client, chain_id).unwrap();

    assert_eq!(
        recovered.canonical_block_hashes,
        vec!["0xg", "0xb1", "0xb2"]
    );
    assert!(!recovered.material_state.contains_key("orphan-only"));
    assert_eq!(
        recovered.material_state.get("route"),
        Some(&"replacement".to_owned())
    );

    let chain_id_db = i64::try_from(chain_id).unwrap();
    for (hash, expected) in [
        ("0xa1", false),
        ("0xa2", false),
        ("0xb1", true),
        ("0xb2", true),
    ] {
        let canonical: bool = client
            .query_one(
                "SELECT canonical FROM chain_evidence_blocks WHERE chain_id = $1 AND block_hash = $2",
                &[&chain_id_db, &hash],
            )
            .unwrap()
            .get(0);
        assert_eq!(canonical, expected, "unexpected canonical flag for {hash}");
    }
}

#[test]
fn corrupt_checkpoint_fails_closed() {
    let Some(url) = database_url() else {
        return;
    };
    let chain_id = 42_005;
    let fixture = reorg_fixture(chain_id);

    let mut client = connect(&url).unwrap();
    clear_chain(&mut client, chain_id).unwrap();
    persist_fixture(&mut client, &fixture, PersistMode::Commit).unwrap();
    let chain_id_db = i64::try_from(chain_id).unwrap();
    client
        .execute(
            "UPDATE chain_evidence_checkpoints SET canonical_tip = '0xcorrupt' WHERE chain_id = $1",
            &[&chain_id_db],
        )
        .unwrap();

    let error = recover(&mut client, chain_id).unwrap_err();
    assert_eq!(error.code(), "STALE_OR_CORRUPT_CHECKPOINT");
}

#[test]
fn known_bad_nontransactional_checkpoint_update_is_detected() {
    let Some(url) = database_url() else {
        return;
    };
    let chain_id = 42_006;
    let old = pre_reorg_fixture(chain_id);
    let replacement = reorg_fixture(chain_id);
    let replacement_report = report_fixture(&replacement);

    let mut client = connect(&url).unwrap();
    clear_chain(&mut client, chain_id).unwrap();
    persist_fixture(&mut client, &old, PersistMode::Commit).unwrap();

    // Known-bad control: advance only the checkpoint, without atomically persisting
    // the corresponding replacement branch/canonical flags.
    let chain_id_db = i64::try_from(chain_id).unwrap();
    client
        .execute(
            "UPDATE chain_evidence_checkpoints \
             SET canonical_tip = $2, canonical_height = 2, state_sha256 = $3, report_sha256 = $4 \
             WHERE chain_id = $1",
            &[
                &chain_id_db,
                &"0xb2",
                &replacement_report.state_sha256,
                &replacement_report.report_sha256,
            ],
        )
        .unwrap();

    let error = recover(&mut client, chain_id).unwrap_err();
    assert_eq!(error.code(), "STALE_OR_CORRUPT_CHECKPOINT");
}
