use std::process::Command;

use serde_json::{Value, json};

fn axis(args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_axis_dtl"))
        .args(args)
        .output()
        .expect("axis_dtl binary should run");

    assert!(
        output.status.success(),
        "axis_dtl exited with {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("scenario output should be valid json")
}

fn assert_hex_digest(value: &Value) {
    let digest = value.as_str().expect("digest should be a string");
    assert_eq!(digest.len(), 64);
    assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));
}

#[test]
fn routed_scenario_reports_conserved_balances() {
    let report = axis(&["routed"]);

    assert_eq!(report["scenario"], json!("routed"));
    assert_eq!(report["conservation_ok"], json!(true));
    assert_eq!(report["surface"]["venues"], json!(1));
    assert_eq!(report["surface"]["routes"], json!(1));
    assert_eq!(report["surface"]["vaults"], json!(1));
    assert_eq!(report["surface"]["margins"], json!(1));
    assert_eq!(report["journal_entries"], json!(11));
    assert_eq!(report["balances"]["payer_source"], json!(23_000_000_000u64));
    assert_eq!(
        report["balances"]["receiver_target"],
        json!(1_993_000_000u64)
    );
    assert_eq!(report["balances"]["solver_target"], json!(7_000_000u64));
    assert_eq!(report["balances"]["pool_target"], json!(48_000_000_000u64));
    assert_hex_digest(&report["state_digest"]);
}

#[test]
fn direct_scenario_uses_single_registered_route() {
    let report = axis(&["direct"]);

    assert_eq!(report["scenario"], json!("direct"));
    assert_eq!(report["order_ids"].as_array().unwrap().len(), 1);
    assert_eq!(report["transaction_ids"].as_array().unwrap().len(), 1);
    assert_eq!(
        report["balances"]["receiver_target"],
        json!(1_247_500_000u64)
    );
    assert_eq!(report["balances"]["solver_target"], json!(2_500_000u64));
    assert_eq!(report["surface"]["routes"], json!(1));
    assert_eq!(report["conservation_ok"], json!(true));
}

#[test]
fn batch_scenario_accumulates_nonce_and_route_state() {
    let report = axis(&["batch"]);
    let orders = report["order_ids"].as_array().unwrap();
    let transactions = report["transaction_ids"].as_array().unwrap();

    assert_eq!(report["scenario"], json!("batch"));
    assert_eq!(orders.len(), 2);
    assert_ne!(orders[0], orders[1]);
    assert_eq!(transactions.len(), 2);
    assert_ne!(transactions[0], transactions[1]);
    assert_eq!(report["surface"]["routes"], json!(2));
    assert_eq!(report["journal_entries"], json!(13));
    assert_eq!(report["balances"]["payer_source"], json!(23_250_000_000u64));
    assert_eq!(
        report["balances"]["receiver_target"],
        json!(1_737_772_500u64)
    );
    assert_eq!(report["conservation_ok"], json!(true));
}

#[test]
fn auction_scenario_accepts_multi_hop_route_surface() {
    let report = axis(&["auction"]);

    assert_eq!(report["scenario"], json!("auction"));
    assert_eq!(report["surface"]["routes"], json!(1));
    assert_eq!(
        report["balances"]["receiver_target"],
        json!(1_775_049_505u64)
    );
    assert_eq!(report["balances"]["solver_target"], json!(7_128_712u64));
    assert_eq!(report["conservation_ok"], json!(true));
    assert_hex_digest(&report["state_digest"]);
}

#[test]
fn snapshot_scenario_exposes_configured_surface_without_orders() {
    let report = axis(&["snapshot"]);

    assert_eq!(report["scenario"], json!("snapshot"));
    assert_eq!(report["order_ids"].as_array().unwrap().len(), 0);
    assert_eq!(report["transaction_ids"].as_array().unwrap().len(), 0);
    assert_eq!(report["surface"]["venues"], json!(1));
    assert_eq!(report["surface"]["routes"], json!(0));
    assert_eq!(report["journal_entries"], json!(9));
    assert_eq!(report["balances"]["pool_target"], json!(50_000_000_000u64));
    assert_eq!(report["conservation_ok"], json!(true));
}

#[test]
fn default_invocation_runs_routed_scenario() {
    let report = axis(&[]);

    assert_eq!(report["scenario"], json!("routed"));
    assert_eq!(report["surface"]["routes"], json!(1));
    assert_eq!(report["conservation_ok"], json!(true));
}
