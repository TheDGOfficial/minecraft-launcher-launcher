use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use colored::Colorize;

#[inline]
#[must_use]
pub(crate) fn read_file(file: &Path) -> Option<String> {
    match fs::read_to_string(file) {
        Ok(text) => Some(text),

        Err(e) => {
            eprintln!(
                "{}{}: {e}",
                "error: can't read file: ".red(),
                file.to_string_lossy()
            );

            None
        },
    }
}

#[inline]
pub(crate) fn write_file(file: &Path, text: &str) -> bool {
    match fs::write(file, text) {
        Ok(()) => true,

        Err(e) => {
            eprintln!(
                "{}{}: {e}",
                "error: can't write to file: ".red(),
                file.to_string_lossy()
            );

            false
        },
    }
}

#[inline]
#[must_use]
pub(crate) fn get_minecraft_dir() -> Option<PathBuf> {
    home::home_dir().map_or_else(
        || {
            eprintln!("error: can't find home directory");

            None
        },
        |home_path| Some(get_minecraft_dir_from_home_path(&home_path)),
    )
}

#[inline]
#[must_use]
pub(crate) fn get_minecraft_dir_from_home_path(home_path: &Path) -> PathBuf {
    home_path.join(
        env::var("MC_GAME_FOLDER").unwrap_or_else(|_| ".minecraft".to_owned()),
    )
}

#[inline]
pub(crate) fn copy(from: &Path, to: &Path) -> bool {
    if let Err(e) = fs::copy(from, to) {
        eprintln!("{}{e}", "error when copying: ".red());

        return false;
    }

    true
}

#[inline]
pub(crate) fn is_same_file(
    file1: &Path,
    file2: &Path,
) -> Result<bool, io::Error> {
    let f1 = File::open(file1)?;
    let f2 = File::open(file2)?;

    if f1.metadata()?.len() != f2.metadata()?.len() {
        return Ok(false);
    }

    let r1 = BufReader::new(f1);
    let r2 = BufReader::new(f2);

    for (b1, b2) in r1.bytes().zip(r2.bytes()) {
        if b1? != b2? {
            return Ok(false);
        }
    }

    Ok(true)
}
