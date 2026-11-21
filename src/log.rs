//! Diagnostic log channel: appends to %TEMP%\wmode-rs.log. Usable from both
//! the launcher process (during ApplyPatchSuspended) and the injected game
//! process, both resolve the same per-user TEMP directory.

pub fn log(msg: &str) {
    use std::io::Write;
    let path = std::env::temp_dir().join("wmode-rs.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{msg}");
    }
}
