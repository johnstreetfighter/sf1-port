//! Disk persistence for native input replays.
use crate::log::Log;
use sf_engine::replay::{MatchReplay, MAX_REPLAY_BYTES, REPLAY_LIMIT};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

fn replay_dir(app_dir: &Path) -> PathBuf {
    app_dir.join("replays")
}

fn replay_name(id: u64) -> String {
    format!("replay-{id}.sf1replay")
}

fn entries(dir: &Path) -> io::Result<Vec<(u64, PathBuf)>> {
    let mut files = Vec::new();
    match fs::read_dir(dir) {
        Ok(dir) => {
            for entry in dir {
                let entry = entry?;
                let path = entry.path();
                if !entry.file_type()?.is_file()
                    || path.extension().and_then(|ext| ext.to_str()) != Some("sf1replay")
                {
                    continue;
                }
                if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                    let id = name
                        .strip_prefix("replay-")
                        .and_then(|name| name.strip_suffix(".sf1replay"))
                        .and_then(|id| id.parse().ok());
                    if let Some(id) = id.filter(|&id| name == replay_name(id)) {
                        files.push((id, path));
                    }
                }
            }
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }
    files.sort_unstable_by_key(|(id, _)| std::cmp::Reverse(*id));
    Ok(files)
}

pub fn load(app_dir: &Path, log: &mut Log) -> Vec<MatchReplay> {
    let files = match entries(&replay_dir(app_dir)) {
        Ok(files) => files,
        Err(error) => {
            log.line("replays", &error.to_string());
            return Vec::new();
        }
    };
    files
        .into_iter()
        .filter_map(|(id, path)| {
            let replay = read_replay(&path).and_then(|replay| {
                (replay.id == id)
                    .then_some(replay)
                    .ok_or_else(|| "replay ID differs from its filename".to_owned())
            });
            match replay {
                Ok(replay) => Some(replay),
                Err(error) => {
                    log.line("replays", &format!("{}: {error}", path.display()));
                    None
                }
            }
        })
        .take(REPLAY_LIMIT)
        .collect()
}

fn read_replay(path: &Path) -> Result<MatchReplay, String> {
    // Bound the read itself, even if a file is changed after we open it.
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| {
            file.take(MAX_REPLAY_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
        })
        .map_err(|error| error.to_string())?;
    if bytes.len() > MAX_REPLAY_BYTES {
        return Err("replay exceeds format limits".to_owned());
    }
    MatchReplay::from_bytes(&bytes)
}

fn next_id(dir: &Path, requested: u64) -> io::Result<u64> {
    match entries(dir)?.first() {
        Some(&(id, _)) if id >= requested => id
            .checked_add(1)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "replay IDs exhausted")),
        _ => Ok(requested),
    }
}

pub fn save(app_dir: &Path, replay: &MatchReplay) -> io::Result<()> {
    let dir = replay_dir(app_dir);
    fs::create_dir_all(&dir)?;
    // Serialize ID assignment, publication, and retention across game windows.
    // The OS releases the lock even if a writer exits before saving.
    let lock = File::options()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join(".save.lock"))?;
    lock.lock()?;
    let id = next_id(&dir, replay.id)?;
    let mut replay = replay.clone();
    replay.id = id;
    let bytes = replay.to_bytes();
    MatchReplay::from_bytes(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let (temp_path, mut file) = loop {
        let nonce = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = dir.join(format!("replay-{}-{nonce}.tmp", std::process::id()));
        match OpenOptions::new().create_new(true).write(true).open(&path) {
            Ok(file) => break (path, file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    let written = file.write_all(&bytes).and_then(|()| file.sync_all());
    drop(file);
    let published = written.and_then(|()| fs::rename(&temp_path, dir.join(replay_name(id))));
    if let Err(error) = published {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }
    for (_, path) in entries(&dir)?.into_iter().skip(REPLAY_LIMIT) {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sf_engine::{
        config::MatchConfig,
        era::StylePick,
        front::fight::{FightDips, FightKind},
        roster::{StagePick, TournamentFighter},
    };

    #[test]
    fn concurrent_saves_keep_distinct_valid_files_and_the_latest_twenty() {
        let nonce = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("sf1-port-replays-{}-{nonce}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        let mut log = Log::create(&dir.join("test.log"));
        let replay = MatchReplay {
            id: 1,
            title: "RYU VS KEN".into(),
            kind: FightKind::Local,
            online: false,
            config: MatchConfig::default(),
            dips: FightDips::default(),
            stage: StagePick::Bank(0),
            flag: None,
            seed: 9,
            data_hash: 42,
            p1_fighter: TournamentFighter::Ryu.into(),
            p2_fighter: TournamentFighter::Ken.into(),
            p1_style: StylePick::default(),
            p2_style: StylePick::default(),
            frames: vec![(0x0402, 0x0020)],
            commands: Vec::new(),
        };
        std::thread::scope(|scope| {
            let a = scope.spawn(|| save(&dir, &replay));
            let b = scope.spawn(|| save(&dir, &replay));
            a.join().unwrap().unwrap();
            b.join().unwrap().unwrap();
        });
        let stored = load(&dir, &mut log);
        assert_eq!(stored.iter().map(|r| r.id).collect::<Vec<_>>(), [2, 1]);
        for _ in 0..REPLAY_LIMIT {
            save(&dir, &replay).unwrap();
        }
        let stored = load(&dir, &mut log);
        assert_eq!(stored.len(), REPLAY_LIMIT);
        assert_eq!((stored[0].id, stored[REPLAY_LIMIT - 1].id), (22, 3));
        assert!(stored.iter().all(|r| r.frames == replay.frames));
        assert_eq!(
            fs::read_dir(replay_dir(&dir)).unwrap().count(),
            REPLAY_LIMIT + 1 // Includes the save lock.
        );
        drop(log);
        fs::remove_dir_all(dir).unwrap();
    }
}
