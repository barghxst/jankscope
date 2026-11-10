use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TraceEvent {
    pub name: String,
    pub cat: String,
    pub ph: String,
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
        assert!(event.ph == "X" && event.dur == Some(10.0))
    }

    #[test]
    fn instant_has_no_dur() {
        let json = r#"{"cat":"devtools.timeline","name":"Mark","ph":"I","pid":46050,"tid":46067,"ts":1740485496}"#;
        let event: TraceEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.ph, "I");
        assert_eq!(event.dur, None);
    }
}
