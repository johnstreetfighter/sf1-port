use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use sf_host::roms::{self, RomSet};

const ROMS: &str = "../../tools/reference/local/roms/sf";

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "sf1-port-rom-archive-{label}-{}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.0.join("sf.zip"));
        let _ = fs::remove_file(self.0.join("sf.7z"));
        let _ = fs::remove_dir(&self.0);
    }
}

fn write_zip(destination: &Path, folder: &Path, corrupt: bool) {
    let mut zip = zip::ZipWriter::new(File::create(destination).unwrap());
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (index, name) in roms::file_names().into_iter().enumerate() {
        let mut bytes = fs::read(folder.join(&name)).unwrap();
        if corrupt && index == 0 {
            bytes[0] ^= 1;
        }
        zip.start_file(format!("sf/{name}"), options).unwrap();
        zip.write_all(&bytes).unwrap();
    }
    zip.finish().unwrap();
}

fn same_regions(archive: &RomSet, folder: &RomSet) {
    assert_eq!(archive.files, folder.files);
    assert_eq!(archive.assets.maincpu, folder.assets.maincpu);
    assert_eq!(archive.assets.gfx1, folder.assets.gfx1);
    assert_eq!(archive.assets.gfx2, folder.assets.gfx2);
    assert_eq!(archive.assets.gfx3, folder.assets.gfx3);
    assert_eq!(archive.assets.gfx4, folder.assets.gfx4);
    assert_eq!(archive.assets.tilerom, folder.assets.tilerom);
    assert_eq!(archive.audio2, folder.audio2);
    assert_eq!(archive.audiocpu, folder.audiocpu);
}

#[test]
fn zip_and_solid_7z_load_like_the_extracted_set() {
    let folder = Path::new(ROMS);
    if !folder.is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let expected = roms::load(folder).expect("the local ROM set matches");
    let scratch = Scratch::new("valid");
    let zip = scratch.0.join("sf.zip");
    let seven_z = scratch.0.join("sf.7z");

    write_zip(&zip, folder, false);
    let mut writer = sevenz_rust2::ArchiveWriter::new(File::create(&seven_z).unwrap()).unwrap();
    let names = roms::file_names();
    writer
        .push_source_path(folder, |path| {
            path.is_dir()
                || path
                    .file_name()
                    .is_some_and(|name| names.iter().any(|required| name == required.as_str()))
        })
        .unwrap();
    writer.finish().unwrap();

    let zipped = roms::load(&zip).expect("the ZIP loads");
    let archived = roms::load(&seven_z).expect("the solid 7z loads");
    same_regions(&zipped, &expected);
    same_regions(&archived, &expected);

    let persisted = seven_z.to_str().unwrap();
    let candidates = roms::candidates(None, None, &scratch.0, Some(persisted));
    assert_eq!(roms::find(&candidates).unwrap().folder, seven_z);
}

#[test]
fn archived_roms_still_have_to_match_the_manifest() {
    let folder = Path::new(ROMS);
    if !folder.is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let scratch = Scratch::new("invalid");
    let zip = scratch.0.join("sf.zip");
    write_zip(&zip, folder, true);

    let name = &roms::file_names()[0];
    let problem = roms::load(&zip)
        .err()
        .expect("manifest rejects the modified ROM");
    assert!(
        problem.starts_with(name) && problem.contains("CRC-32"),
        "{problem}"
    );
}
