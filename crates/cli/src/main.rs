use std::{env::args, error::Error, process::ExitCode};

use jankscope_core::trace;

fn main() -> ExitCode {
    let args: Vec<String> = args().collect();
    let result = match args.as_slice() {
        [_, cmd, path] if cmd == "stats" => stats(path),
        [_, cmd, path] if cmd == "frames" => show_frames(path),
        _ => {
            eprintln!("usage: jankscope <stats|frames> <trace.json>");
            return ExitCode::from(2);
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn stats(path: &str) -> Result<(), Box<dyn Error>> {
    let json = std::fs::read_to_string(path)?;
    let trace = trace::parse(&json)?;

    println!("events:     {}", trace.events.len());
    println!("duration:   {:.1} ms", trace.duration_ms());

    let mut phases: Vec<_> = trace.count_by_phase().into_iter().collect();
    phases.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let phases: Vec<String> = phases
        .iter()
        .map(|(phase, count)| format!("{phase:?} {count}"))
        .collect();
    println!("phases:      {}", phases.join(" · "));

    println!("threads:");
    for (pid, tid, name) in trace.thread_names() {
        println!("  {pid:>6}:{tid:<6}  {name}");
    }

    println!("top names:");

    for (name, count) in trace.top_names(10) {
        println!("  {name:<40}  {count:>5}");
    }

    Ok(())
}

fn show_frames(path: &str) -> Result<(), Box<dyn Error>> {
    let json = std::fs::read_to_string(path)?;
    let trace = trace::parse(&json)?;
    let renderer = trace.main_renderer()?;
    let frames = jankscope_core::frames::frames(&trace, &renderer);

    let Some(first) = frames.first() else {
        println!("No frames in trace");
        return Ok(());
    };

    println!("frame   begin (ms)   draw (ms)   status");
    for frame in &frames {
        let begin = (frame.begin_ts - first.begin_ts) / 1000.0;
        let draw = match frame.draw_ts {
            Some(ts) => format!("{:.1}", (ts - first.begin_ts) / 1000.0),
            None => String::from("-"),
        };
        println!(
            "{:>5}   {:>10.1}   {:>9}   {:?}",
            frame.seq, begin, draw, frame.status
        );
    }

    Ok(())
}
