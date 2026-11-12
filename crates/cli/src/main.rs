use std::{env::args, error::Error, process::ExitCode};

use jankscope_core::trace;

fn main() -> ExitCode {
    let args: Vec<String> = args().collect();
    match args.as_slice() {
        [_, cmd, path] if cmd == "stats" => match stats(path) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprint!("error: {e}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("usage: jankscope stats <trace.json>");
            ExitCode::from(2)
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
