mod common;

use jankscope_core::trace::{EventId, Phase};

const EDGE_CASES: &str = "parse-edge-cases";

#[test]
fn edge_cases_event_count() {
    let trace = common::load(EDGE_CASES);
    assert_eq!(trace.events.len(), 41);
}

#[test]
fn edge_cases_phases() {
    let trace = common::load(EDGE_CASES);
    let phases = trace.count_by_phase();
    assert_eq!(phases.len(), 10);
    assert_eq!(phases[&Phase::Metadata], 5);
    assert_eq!(phases[&Phase::Complete], 4);
    assert_eq!(phases[&Phase::Begin], 4);
    assert!(!phases.contains_key(&Phase::End));
}

#[test]
fn edge_cases_ids() {
    let trace = common::load(EDGE_CASES);
    assert!(
        trace
            .events
            .iter()
            .any(|e| matches!(e.id, Some(EventId::Num(_))))
    );
    assert!(
        trace
            .events
            .iter()
            .any(|e| matches!(e.id, Some(EventId::Str(_))))
    );
}

#[test]
fn edge_cases_summary() {
    let trace = common::load(EDGE_CASES);
    assert!(trace.thread_names().contains(&(46120, 1, "CrRendererMain")));
    assert!((trace.duration_ms() - 2949.5).abs() < 0.1);
    assert_eq!(trace.top_names(1), vec![("RunTask", 8)]);
}
