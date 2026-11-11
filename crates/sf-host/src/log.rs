//! The game's log file (spec #150, section 9): one file in the app-data
//! folder, overwritten at every launch. Each line is the seconds since
//! launch, a kind and the text. The kinds are `version` (the game's edition,
//! version and build commit, written first at launch), `settings` (the settings in
//! force), `roms` (the ROM folder and the manifest result), `pads` (pads
//! seen or lost), `parse` (a settings file that failed to parse),
//! `untranslated` (a branch the translation lacks, with frame and
//! addresses), `art` (the art built at launch, or why the game kept the
//! ROM's own graphics), `stages` (the background bank built at launch, or
//! why there is none), `themes` (the extended tune table and the music pack, or why there is
//! none) and `voice` (the announcer's voice pack, or why
//! there is no announcer). Every line is also written to stderr.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct Log {
    file: Option<File>,
    path: PathBuf,
    start: Instant,
}

impl Log {
    /// Create (or empty) the log at `path`. A log that cannot be written
    /// leaves stderr only, and says so there.
    pub fn create(path: &Path) -> Log {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let file = File::create(path)
            .map_err(|e| eprintln!("log: cannot write {}: {e}", path.display()))
            .ok();
        Log {
            file,
            path: path.to_owned(),
            start: Instant::now(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// One line of `kind`, written and flushed at once so a crash keeps it.
    pub fn line(&mut self, kind: &str, text: &str) {
        let line = format!("[{:8.3}] {kind}: {text}", self.start.elapsed().as_secs_f64());
        eprintln!("{line}");
        if let Some(file) = &mut self.file {
            let _ = writeln!(file, "{line}").and_then(|_| file.flush());
        }
    }
}
