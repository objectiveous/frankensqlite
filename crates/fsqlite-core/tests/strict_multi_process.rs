//! frankensqlite#81 — strict multi-process refusal mode.
//!
//! These tests cover the infrastructure for the strict-multi-process
//! opt-in: the `ConnectionEnv` flag, the `Connection::open_strict_multi_process`
//! convenience constructor, and the new `MultiProcessContractViolation`
//! error variant. Concrete refusal sites (F_SETLK timeout, WAL checkpoint
//! contention, freelist trunk drift past db_size) attach to this
//! infrastructure in follow-up work — the test here proves the opt-in
//! plumbing itself is in place.

use fsqlite_core::connection::ConnectionEnv;
use fsqlite_error::FrankenError;

#[test]
fn connection_env_strict_multi_process_defaults_off() {
    let env = ConnectionEnv::default();
    assert!(
        !env.strict_multi_process(),
        "default ConnectionEnv should leave strict_multi_process disabled to preserve existing best-effort behavior"
    );
}

#[test]
fn connection_env_strict_multi_process_round_trips() {
    let mut env = ConnectionEnv::default();
    env.set_strict_multi_process(true);
    assert!(env.strict_multi_process(), "after enable, flag should read true");
    env.set_strict_multi_process(false);
    assert!(!env.strict_multi_process(), "after disable, flag should read false");
}

#[test]
fn multi_process_contract_violation_carries_detail() {
    let err = FrankenError::MultiProcessContractViolation {
        detail: "freelist trunk page 42 exceeds db_size 10".to_string(),
    };
    let msg = format!("{err}");
    assert!(
        msg.contains("multi-process contract violation"),
        "error display should mention the contract violation kind: {msg}"
    );
    assert!(
        msg.contains("freelist trunk page 42"),
        "error display should propagate the detail: {msg}"
    );
}
