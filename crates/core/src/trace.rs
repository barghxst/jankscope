use serde::Deserialize;

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
}
