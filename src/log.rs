//! Diagnostic log: appends to %TEMP%\wmode-rs.log, from both the launcher
//! and the injected game process.

pub fn log(msg: &str) {
    use std::io::Write;
    let path = std::env::temp_dir().join("wmode-rs.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{msg}");
    }
}
