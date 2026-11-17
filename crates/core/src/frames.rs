use std::collections::HashMap;

use thiserror::Error;

use crate::trace::{Trace, TraceEvent};

impl TraceEvent {
    pub fn frame_seq_id(&self) -> Option<u64> {
        self.args.get("frameSeqId")?.as_u64()
    }
}

#[derive(Debug, PartialEq)]
pub struct Renderer {
    pub pid: i64,
    pub main_tid: i64,
    pub compositor_tid: i64,
}

#[derive(Debug, Error, PartialEq)]
pub enum RendererError {
    #[error("no renderer main thread in trace")]
    NotFound,
    #[error("several renderers and no TracingStartedInBrowser: {0:?}")]
    Ambiguous(Vec<i64>),
    #[error("renderer {0} has no compositor thread")]
    NoCompositor(i64),
}

impl Trace {
    pub fn main_renderer(&self) -> Result<Renderer, RendererError> {
        let pid = match self.main_frame_pid() {
            Some(pid) => pid,
            None => self.only_renderer_pid()?,
        };
        let threads = self.thread_names();
        let main_tid =
            find_thread(&threads, pid, "CrRendererMain").ok_or(RendererError::NotFound)?;
        let compositor_tid =
            find_thread(&threads, pid, "Compositor").ok_or(RendererError::NoCompositor(pid))?;
        Ok(Renderer {
            pid,
            main_tid,
            compositor_tid,
        })
    }

    fn main_frame_pid(&self) -> Option<i64> {
        let started = self
            .events
            .iter()
            .find(|e| e.name == "TracingStartedInBrowser")?;
        let frames = started.args.get("data")?.get("frames")?.as_array()?;
        let main = frames
            .iter()
            .find(|f| f.get("isOutermostMainFrame").and_then(|v| v.as_bool()) == Some(true))?;
        main.get("processId")?.as_i64()
    }

    fn only_renderer_pid(&self) -> Result<i64, RendererError> {
        let mut pids: Vec<i64> = self
            .thread_names()
            .into_iter()
            .filter(|(_, _, name)| *name == "CrRendererMain")
            .map(|(pid, _, _)| pid)
            .collect();
        pids.dedup();
        match pids.as_slice() {
            [] => Err(RendererError::NotFound),
            [pid] => Ok(*pid),
            _ => Err(RendererError::Ambiguous(pids)),
        }
    }
}

fn find_thread(threads: &[(i64, i64, &str)], pid: i64, name: &str) -> Option<i64> {
    threads
        .iter()
        .find(|(p, _, n)| *p == pid && *n == name)
        .map(|(_, tid, _)| *tid)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameStatus {
    Presented,
    Dropped,
    Idle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub seq: u64,
    pub begin_ts: f64,
    pub draw_ts: Option<f64>,
    pub status: FrameStatus,
}

pub fn frames(trace: &Trace, renderer: &Renderer) -> Vec<Frame> {
    let draws: HashMap<u64, f64> = trace
        .events_on(renderer.pid, renderer.compositor_tid)
        .filter(|event| event.name == "DrawFrame")
        .filter_map(|event| Some((event.frame_seq_id()?, event.ts)))
        .collect();

    let mut frames: Vec<Frame> = trace
        .events_on(renderer.pid, renderer.compositor_tid)
        .filter(|event| event.name == "BeginFrame")
        .filter_map(|event| {
            let seq = event.frame_seq_id()?;
            Some(Frame {
                seq,
                begin_ts: event.ts,
                draw_ts: draws.get(&seq).copied(),
                status: FrameStatus::Idle,
            })
        })
        .collect();

    frames.sort_by_key(|frame| frame.seq);

    let mut seen_presented = false;
    for frame in frames.iter_mut().rev() {
        if frame.draw_ts.is_some() {
            frame.status = FrameStatus::Presented;
            seen_presented = true;
        } else if seen_presented {
            frame.status = FrameStatus::Dropped;
        }
    }

    frames
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::parse;

    const RENDERER_10: &str = r#"
        {"name":"thread_name","ph":"M","pid":10,"tid":1,"ts":0,"cat":"__metadata","args":{"name":"CrRendererMain"}},
        {"name":"thread_name","ph":"M","pid":10,"tid":9,"ts":0,"cat":"__metadata","args":{"name":"Compositor"}}"#;
    const RENDERER_20: &str = r#"
        {"name":"thread_name","ph":"M","pid":20,"tid":1,"ts":0,"cat":"__metadata","args":{"name":"CrRendererMain"}},
        {"name":"thread_name","ph":"M","pid":20,"tid":7,"ts":0,"cat":"__metadata","args":{"name":"Compositor"}}"#;
    const STARTED_20: &str = r#"
        {"name":"TracingStartedInBrowser","ph":"I","pid":1,"tid":1,"ts":0,"cat":"devtools.timeline",
         "args":{"data":{"frames":[
            {"processId":10,"isOutermostMainFrame":false},
            {"processId":20,"isOutermostMainFrame":true}]}}}"#;

    fn trace_of(parts: &[&str]) -> Trace {
        let json = format!("[{}]", parts.join(","));
        parse(&json).unwrap()
    }

    #[test]
    fn renderer_single() {
        let trace = trace_of(&[RENDERER_10]);
        assert_eq!(
            trace.main_renderer(),
            Ok(Renderer {
                pid: 10,
                main_tid: 1,
                compositor_tid: 9
            })
        );
    }

    #[test]
    fn renderer_from_tracing_started() {
        let trace = trace_of(&[RENDERER_10, RENDERER_20, STARTED_20]);
        assert_eq!(
            trace.main_renderer(),
            Ok(Renderer {
                pid: 20,
                main_tid: 1,
                compositor_tid: 7
            })
        );
    }

    #[test]
    fn renderer_ambiguous() {
        let trace = trace_of(&[RENDERER_10, RENDERER_20]);
        assert_eq!(
            trace.main_renderer(),
            Err(RendererError::Ambiguous(vec![10, 20]))
        );
    }

    #[test]
    fn renderer_not_found() {
        let trace = trace_of(&[]);
        assert_eq!(trace.main_renderer(), Err(RendererError::NotFound));
    }

    #[test]
    fn renderer_no_compositor() {
        let trace = trace_of(&[
            r#"{"name":"thread_name","ph":"M","pid":10,"tid":1,"ts":0,"cat":"__metadata","args":{"name":"CrRendererMain"}}"#,
        ]);
        assert_eq!(trace.main_renderer(), Err(RendererError::NoCompositor(10)));
    }

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

    #[test]
    fn frames_match_draws_and_classify_missing_draws() {
        let trace = trace_of(&[
            RENDERER_10,
            r#"{"name":"BeginFrame","ph":"I","pid":10,"tid":9,"ts":300,"cat":"x","args":{"frameSeqId":3}}"#,
            r#"{"name":"DrawFrame","ph":"I","pid":20,"tid":9,"ts":110,"cat":"x","args":{"frameSeqId":1}}"#,
            r#"{"name":"BeginFrame","ph":"I","pid":10,"tid":9,"ts":100,"cat":"x","args":{"frameSeqId":1}}"#,
            r#"{"name":"DrawFrame","ph":"I","pid":10,"tid":9,"ts":220,"cat":"x","args":{"frameSeqId":2}}"#,
            r#"{"name":"BeginFrame","ph":"I","pid":10,"tid":9,"ts":200,"cat":"x","args":{"frameSeqId":2}}"#,
        ]);

        let renderer = trace.main_renderer().unwrap();
        let result = frames(&trace, &renderer);

        let actual: Vec<_> = result
            .iter()
            .map(|frame| (frame.seq, frame.begin_ts, frame.draw_ts, frame.status))
            .collect();

        assert_eq!(
            actual,
            vec![
                (1, 100.0, None, FrameStatus::Dropped),
                (2, 200.0, Some(220.0), FrameStatus::Presented),
                (3, 300.0, None, FrameStatus::Idle),
            ]
        );
    }
}
