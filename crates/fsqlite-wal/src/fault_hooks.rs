//! Targeted fault hooks for batched WAL append and sync verification.

use std::mem;
use std::sync::{LazyLock, Mutex};

use fsqlite_error::{FrankenError, Result};
use fsqlite_types::flags::SyncFlags;
use tracing::warn;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultHookArm {
    pub run_id: String,
    pub scenario_id: String,
    pub invariant_family: String,
}

impl FaultHookArm {
    #[must_use]
    pub fn new(
        run_id: impl Into<String>,
        scenario_id: impl Into<String>,
        invariant_family: impl Into<String>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            scenario_id: scenario_id.into(),
            invariant_family: invariant_family.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultInjectionRecord {
    pub trigger_seq: u64,
    pub point: &'static str,
    pub run_id: String,
    pub scenario_id: String,
    pub invariant_family: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
struct CountdownFaultArm {
    arm: FaultHookArm,
    remaining_invocations: u32,
}

#[derive(Debug, Default)]
struct WalFaultHookState {
    next_trigger_seq: u64,
    after_append: Option<FaultHookArm>,
    sync_failure: Option<FaultHookArm>,
    append_busy: Option<CountdownFaultArm>,
    crash_header_truncate: Option<FaultHookArm>,
    records: Vec<FaultInjectionRecord>,
}

static WAL_FAULT_HOOK_STATE: LazyLock<Mutex<WalFaultHookState>> =
    LazyLock::new(|| Mutex::new(WalFaultHookState::default()));

pub fn clear() {
    let mut state = WAL_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *state = WalFaultHookState::default();
}

#[must_use]
pub fn take_records() -> Vec<FaultInjectionRecord> {
    let mut state = WAL_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    mem::take(&mut state.records)
}

pub fn arm_after_append(arm: FaultHookArm) {
    let mut state = WAL_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.after_append = Some(arm);
}

pub fn arm_sync_failure(arm: FaultHookArm) {
    let mut state = WAL_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.sync_failure = Some(arm);
}

pub fn arm_append_busy_countdown(arm: FaultHookArm, fire_on_nth_invocation: u32) {
    let mut state = WAL_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.append_busy = Some(CountdownFaultArm {
        arm,
        remaining_invocations: fire_on_nth_invocation.max(1),
    });
}

pub(crate) fn maybe_inject_append_busy(
    frame_count_before: usize,
    submitted_frames: usize,
) -> Result<()> {
    let mut state = WAL_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(mut countdown) = state.append_busy.take() else {
        return Ok(());
    };
    if countdown.remaining_invocations > 1 {
        countdown.remaining_invocations -= 1;
        state.append_busy = Some(countdown);
        return Ok(());
    }

    let detail =
        format!("frame_count_before={frame_count_before} submitted_frames={submitted_frames}");
    record_trigger(
        &mut state,
        &countdown.arm,
        "wal_append_busy_countdown",
        detail,
    );
    Err(FrankenError::Busy)
}

pub(crate) fn maybe_inject_after_append(
    frame_count_before: usize,
    appended_frames: usize,
) -> Result<()> {
    let mut state = WAL_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(arm) = state.after_append.take() else {
        return Ok(());
    };

    let detail =
        format!("frame_count_before={frame_count_before} appended_frames={appended_frames}");
    record_trigger(&mut state, &arm, "wal_after_append", detail);
    Err(fault_error("wal_after_append", &arm))
}

pub(crate) fn maybe_inject_sync_failure(frame_count_before: usize, flags: SyncFlags) -> Result<()> {
    let mut state = WAL_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(arm) = state.sync_failure.take() else {
        return Ok(());
    };

    let detail = format!("frame_count_before={frame_count_before} flags={flags:?}");
    record_trigger(&mut state, &arm, "wal_sync_failure", detail);
    Err(fault_error("wal_sync_failure", &arm))
}

fn record_trigger(
    state: &mut WalFaultHookState,
    arm: &FaultHookArm,
    point: &'static str,
    detail: String,
) {
    state.next_trigger_seq = state.next_trigger_seq.saturating_add(1);
    let record = FaultInjectionRecord {
        trigger_seq: state.next_trigger_seq,
        point,
        run_id: arm.run_id.clone(),
        scenario_id: arm.scenario_id.clone(),
        invariant_family: arm.invariant_family.clone(),
        detail,
    };
    warn!(
        target: "fsqlite_wal::fault_injection",
        trigger_seq = record.trigger_seq,
        point = record.point,
        run_id = %record.run_id,
        scenario_id = %record.scenario_id,
        invariant_family = %record.invariant_family,
        detail = %record.detail,
        "fault hook fired"
    );
    state.records.push(record);
}

/// Arm the crash-between-header-and-truncate hook (F9 / H9).
///
/// When armed, `maybe_inject_crash_header_truncate()` fires once after
/// the new WAL header is written but before the file is truncated.
/// This simulates a crash that leaves new-generation salts in the header
/// but old-generation frames still on disk.
pub fn arm_crash_header_truncate(arm: FaultHookArm) {
    let mut state = WAL_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state.crash_header_truncate = Some(arm);
}

/// Check and fire the crash-between-header-and-truncate hook.
///
/// Called inside `WalFile::reset()` after the new header is written
/// and synced but before the file is truncated.
pub(crate) fn maybe_inject_crash_header_truncate(
    old_frame_count: usize,
    new_checkpoint_seq: u32,
) -> Result<()> {
    let mut state = WAL_FAULT_HOOK_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(arm) = state.crash_header_truncate.take() else {
        return Ok(());
    };

    let detail =
        format!("old_frame_count={old_frame_count} new_checkpoint_seq={new_checkpoint_seq}");
    record_trigger(&mut state, &arm, "wal_crash_header_truncate", detail);
    Err(fault_error("wal_crash_header_truncate", &arm))
}

fn fault_error(point: &str, arm: &FaultHookArm) -> FrankenError {
    FrankenError::Io(std::io::Error::other(format!(
        "fault_inject:{point} run_id={} scenario_id={} invariant_family={}",
        arm.run_id, arm.scenario_id, arm.invariant_family
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_GUARD: Mutex<()> = Mutex::new(());

    fn arm(point: &str) -> FaultHookArm {
        FaultHookArm::new(format!("test-{point}"), format!("scenario-{point}"), "unit")
    }

    #[test]
    fn test_clear_resets_all_armed_hooks_and_records() {
        let _g = TEST_GUARD.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        clear();

        arm_after_append(arm("append"));
        arm_sync_failure(arm("sync"));
        arm_append_busy_countdown(arm("busy"), 1);
        arm_crash_header_truncate(arm("crash"));
        let _ = maybe_inject_after_append(0, 1);
        assert_eq!(take_records().len(), 1);

        clear();
        assert!(maybe_inject_after_append(0, 1).is_ok());
        assert!(maybe_inject_sync_failure(0, SyncFlags::NORMAL).is_ok());
        assert!(maybe_inject_append_busy(0, 1).is_ok());
        assert!(maybe_inject_crash_header_truncate(0, 1).is_ok());
        assert!(take_records().is_empty());
    }

    #[test]
    fn test_after_append_fires_once_then_disarms() {
        let _g = TEST_GUARD.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        clear();

        arm_after_append(arm("append"));
        let err = maybe_inject_after_append(5, 3);
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("fault_inject:wal_after_append"));

        assert!(maybe_inject_after_append(8, 2).is_ok());
    }

    #[test]
    fn test_sync_failure_fires_once_then_disarms() {
        let _g = TEST_GUARD.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        clear();

        arm_sync_failure(arm("sync"));
        let err = maybe_inject_sync_failure(10, SyncFlags::FULL);
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("fault_inject:wal_sync_failure"));

        assert!(maybe_inject_sync_failure(10, SyncFlags::FULL).is_ok());
    }

    #[test]
    fn test_append_busy_countdown_fires_on_nth_invocation() {
        let _g = TEST_GUARD.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        clear();

        arm_append_busy_countdown(arm("busy"), 3);
        assert!(maybe_inject_append_busy(0, 1).is_ok(), "1st call should not fire");
        assert!(maybe_inject_append_busy(1, 1).is_ok(), "2nd call should not fire");
        let err = maybe_inject_append_busy(2, 1);
        assert!(err.is_err(), "3rd call should fire");
        assert!(matches!(err.unwrap_err(), FrankenError::Busy));

        assert!(maybe_inject_append_busy(3, 1).is_ok(), "disarmed after fire");
    }

    #[test]
    fn test_append_busy_countdown_minimum_one() {
        let _g = TEST_GUARD.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        clear();

        arm_append_busy_countdown(arm("busy"), 0);
        let err = maybe_inject_append_busy(0, 1);
        assert!(err.is_err(), "countdown of 0 should clamp to 1 and fire immediately");
    }

    #[test]
    fn test_crash_header_truncate_fires_once() {
        let _g = TEST_GUARD.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        clear();

        arm_crash_header_truncate(arm("crash"));
        let err = maybe_inject_crash_header_truncate(4, 2);
        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("fault_inject:wal_crash_header_truncate"),
        );

        assert!(maybe_inject_crash_header_truncate(4, 3).is_ok());
    }

    #[test]
    fn test_records_capture_trigger_details() {
        let _g = TEST_GUARD.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        clear();

        arm_after_append(FaultHookArm::new("run-a", "scen-a", "inv-a"));
        let _ = maybe_inject_after_append(7, 3);

        arm_sync_failure(FaultHookArm::new("run-b", "scen-b", "inv-b"));
        let _ = maybe_inject_sync_failure(12, SyncFlags::FULL);

        let records = take_records();
        assert_eq!(records.len(), 2);

        assert_eq!(records[0].point, "wal_after_append");
        assert_eq!(records[0].run_id, "run-a");
        assert!(records[0].detail.contains("frame_count_before=7"));
        assert!(records[0].detail.contains("appended_frames=3"));

        assert_eq!(records[1].point, "wal_sync_failure");
        assert_eq!(records[1].run_id, "run-b");
        assert!(records[1].detail.contains("frame_count_before=12"));

        assert_eq!(records[0].trigger_seq, 1);
        assert_eq!(records[1].trigger_seq, 2);
    }

    #[test]
    fn test_take_records_drains_and_is_empty_after() {
        let _g = TEST_GUARD.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        clear();

        arm_after_append(arm("append"));
        let _ = maybe_inject_after_append(0, 1);

        let first = take_records();
        assert_eq!(first.len(), 1);
        let second = take_records();
        assert!(second.is_empty());
    }
}
