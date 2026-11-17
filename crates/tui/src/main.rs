use std::{error::Error, fs, path::Path};

use crossterm::event::{self, Event, KeyCode};
use jankscope_core::{frames, trace};
use ratatui::widgets::{Block, List, ListState};

fn main() -> Result<(), Box<dyn Error>> {
    let mut files = Vec::new();

    for entry in fs::read_dir("fixtures/recorded")? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();

        if entry.file_type()?.is_file() && name.ends_with(".trace.json") {
            files.push(name);
        }
    }

    files.sort();

    let mut trace_selection = ListState::default().with_selected(Some(0));
    let mut frame_selection = ListState::default();
    let mut opened: Option<Vec<String>> = None;

    ratatui::run(|terminal| {
        loop {
            let (rows, selection, title) = match &opened {
                Some(rows) => (rows, &mut frame_selection, " Frames / Esc: back "),
                None => (&files, &mut trace_selection, " Traces / Enter: open "),
            };

            terminal.draw(|screen| {
                let list = List::new(rows.iter().map(String::as_str))
                    .block(Block::bordered().title(title))
                    .highlight_symbol("> ");

                screen.render_stateful_widget(list, screen.area(), selection);
            })?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => selection.select_previous(),
                    KeyCode::Down => selection.select_next(),
                    KeyCode::Enter if opened.is_none() => {
                        if let Some(name) = selection.selected().and_then(|i| files.get(i)) {
                            opened = Some(load_frame_rows(name)?);
                            frame_selection = ListState::default().with_selected(Some(0));
                        }
                    }
                    KeyCode::Esc => opened = None,
                    KeyCode::Char('q') => return Ok(()),
                    _ => {}
                }
            }
        }
    })
}

fn load_frame_rows(name: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let path = Path::new("fixtures/recorded").join(name);
    let json = fs::read_to_string(path)?;
    let trace = trace::parse(&json)?;
    let renderer = trace.main_renderer()?;
    let frames = frames::frames(&trace, &renderer);
    let start = frames.first().map_or(0.0, |frame| frame.begin_ts);

    let mut rows = Vec::new();
    for frame in frames {
        let draw = match frame.draw_ts {
            Some(ts) => format!("{:.1} ms", (ts - start) / 1000.0),
            None => String::from("-"),
        };

        rows.push(format!("{:>5}  {:<10?}  {draw}", frame.seq, frame.status));
    }

    Ok(rows)
}
