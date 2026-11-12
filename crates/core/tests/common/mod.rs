use jankscope_core::trace::{self, Trace};

/// Загружает `fixtures/traces/<name>.trace.json` и парсит его.
pub fn load(name: &str) -> Trace {
    let path = format!(
        "{}/../../fixtures/traces/{name}.trace.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read fixture {path}: {e}"));
    trace::parse(&json).unwrap_or_else(|e| panic!("cannot parse fixture {path}: {e}"))
}
