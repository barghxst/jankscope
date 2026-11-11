use std::collections::HashMap;

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Deserialize)]
#[serde(from = "String")]
pub enum Phase {
    Complete,
    Instant,
    Begin,
    End,
    AsyncBegin,
    AsyncEnd,
    AsyncInstant,
    FlowStart,
    FlowEnd,
    Sample,
    Metadata,
    Other(String),
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum EventId {
    Num(u64),
    Str(String),
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("invalid trace JSON: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug)]
pub struct Trace {
    pub events: Vec<TraceEvent>,
}

impl Trace {
    pub fn count_by_phase(&self) -> HashMap<Phase, usize> {
        let mut counts = HashMap::new();
        for event in &self.events {
            *counts.entry(event.ph.clone()).or_insert(0) += 1;
        }
        counts
    }
    pub fn top_names(&self, n: usize) -> Vec<(&str, usize)> {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for event in &self.events {
            *counts.entry(event.name.as_str()).or_insert(0) += 1
        }

        let mut top: Vec<(&str, usize)> = counts.into_iter().collect();
        top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        top.truncate(n);
        top
    }
}

#[derive(Deserialize)]
struct Wrapped {
    #[serde(rename = "traceEvents")]
    trace_events: Vec<TraceEvent>,
}

pub fn parse(json: &str) -> Result<Trace, ParseError> {
    let events: Vec<TraceEvent> = if json.trim_start().starts_with('[') {
        serde_json::from_str(json)?
    } else {
        let wrapped: Wrapped = serde_json::from_str(json)?;
        wrapped.trace_events
    };
    Ok(Trace { events })
}

impl From<String> for Phase {
    fn from(value: String) -> Self {
        match value.as_str() {
            "X" => Phase::Complete,
            "I" => Phase::Instant,
            "B" => Phase::Begin,
            "E" => Phase::End,
            "b" => Phase::AsyncBegin,
            "e" => Phase::AsyncEnd,
            "n" => Phase::AsyncInstant,
            "s" => Phase::FlowStart,
            "f" => Phase::FlowEnd,
            "P" => Phase::Sample,
            "M" => Phase::Metadata,
            _ => Phase::Other(value),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TraceEvent {
    pub id: Option<EventId>,
    pub name: String,
    pub cat: String,
    pub ph: Phase,
    pub ts: f64,
    pub dur: Option<f64>,
    pub pid: i64,
    pub tid: i64,
    #[serde(default)]
    pub args: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    const SMALL: &str = r#"[
        {"cat":"a","name":"RunTask","ph":"X","dur":5,"pid":1,"tid":1,"ts":100},
        {"cat":"a","name":"RunTask","ph":"X","dur":5,"pid":1,"tid":1,"ts":200},
        {"cat":"a","name":"Mark","ph":"I","pid":1,"tid":1,"ts":150}
    ]"#;

    #[test]
    fn ph_is_x() {
        let json = r#"{"args":{},"cat":"disabled-by-default-devtools.timeline","dur":10,"name":"RunTask","ph":"X","pid":46050,"tdur":10,"tid":46067,"ts":1740485496,"tts":789}"#;
        let event: TraceEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.ph, Phase::Complete);
        assert_eq!(event.dur, Some(10.0));
    }

    #[test]
    fn instant_has_no_dur() {
        let json = r#"{"cat":"devtools.timeline","name":"Mark","ph":"I","pid":46050,"tid":46067,"ts":1740485496}"#;
        let event: TraceEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.ph, Phase::Instant);
        assert_eq!(event.dur, None);
    }

    #[test]
    fn other_phase() {
        let json = r#"{"cat":"devtools.timeline","name":"Mark","ph":"Z","pid":46050,"tid":46067,"ts":1740485496}"#;
        let event: TraceEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.ph, Phase::Other("Z".to_string()));
    }

    #[test]
    fn id_is_num() {
        let json = r#"{"id":0,"cat":"devtools.timeline","name":"Mark","ph":"Z","pid":46050,"tid":46067,"ts":1740485496}"#;
        let event: TraceEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.id, Some(EventId::Num(0)));
    }

    #[test]
    fn id_is_string() {
        let json = r#"{"id":"0x1","cat":"devtools.timeline","name":"Mark","ph":"Z","pid":46050,"tid":46067,"ts":1740485496}"#;
        let event: TraceEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.id, Some(EventId::Str("0x1".to_string())));
    }

    #[test]
    fn id_is_none() {
        let json = r#"{"cat":"devtools.timeline","name":"Mark","ph":"Z","pid":46050,"tid":46067,"ts":1740485496}"#;
        let event: TraceEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.id, None);
    }

    #[test]
    fn parse_wrapped() {
        let json = r#"{"traceEvents":[{"cat":"devtools.timeline","name":"Mark","ph":"I","pid":46050,"tid":46067,"ts":1740485496}]}"#;
        let trace = parse(json).unwrap();
        assert_eq!(trace.events.len(), 1);
    }

    #[test]
    fn parse_bare_array() {
        let json = r#"[{"cat":"devtools.timeline","name":"Mark","ph":"I","pid":46050,"tid":46067,"ts":1740485496},{"cat":"devtools.timeline","name":"Mark","ph":"I","pid":46050,"tid":46067,"ts":1740485500}]"#;
        let trace = parse(json).unwrap();
        assert_eq!(trace.events.len(), 2);
    }
    #[test]
    fn parse_with_metadata() {
        let json = r#"{"traceEvents":[{"cat":"devtools.timeline","name":"Mark","ph":"I","pid":46050,"tid":46067,"ts":1740485496}],"metadata":{"source":"DevTools"}}"#;
        let trace = parse(json).unwrap();
        assert_eq!(trace.events.len(), 1);
    }
    #[test]
    fn parse_broken() {
        let json = r#"{"traceEvents":}"#;
        assert!(parse(json).is_err());
    }
    #[test]
    fn counts_phases() {
        let trace = parse(SMALL).unwrap();
        let counts = trace.count_by_phase();
        assert_eq!(counts[&Phase::Complete], 2);
        assert_eq!(counts[&Phase::Instant], 1);
        assert_eq!(counts.len(), 2);
    }
    #[test]
    fn top_names_sorted() {
        let trace = parse(SMALL).unwrap();
        assert_eq!(trace.top_names(5), vec![("RunTask", 2), ("Mark", 1)]);
        assert_eq!(trace.top_names(1), vec![("RunTask", 2)]);
    }
}
