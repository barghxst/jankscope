use crate::trace::TraceEvent;

impl TraceEvent {
    pub fn frame_seq_id(&self) -> Option<u64> {
        self.args.get("frameSeqId")?.as_u64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seq_id_from_begin_frame() {
        let json = r#"{"name":"BeginFrame","ph":"I","pid":46120,"tid":9,"ts":1740947026,"cat":"x","args":{"frameSeqId":204,"layerTreeId":2}}"#;
        let event: TraceEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.frame_seq_id(), Some(204));
    }

    #[test]
    fn seq_id_missing_args() {
        let json =
            r#"{"name":"BeginFrame","ph":"I","pid":46120,"tid":9,"ts":1740947026,"cat":"x"}"#;
        let event: TraceEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.frame_seq_id(), None);
    }

    #[test]
    fn seq_id_missing_key() {
        let json = r#"{"name":"BeginFrame","ph":"I","pid":46120,"tid":9,"ts":1740947026,"cat":"x","args":{"layerTreeId":2}}"#;
        let event: TraceEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.frame_seq_id(), None);
    }

    #[test]
    fn seq_id_string_is_none() {
        let json = r#"{"name":"BeginFrame","ph":"I","pid":46120,"tid":9,"ts":1740947026,"cat":"x","args":{"frameSeqId":"204"}}"#;
        let event: TraceEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.frame_seq_id(), None);
    }
}
