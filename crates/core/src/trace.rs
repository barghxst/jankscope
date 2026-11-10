use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, PartialEq, Deserialize)]
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
}
