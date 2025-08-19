//! M containing utility functions.
use std::env;
use std::io::{Read, BufReader};
use git2::{Repository, Error};
use sha2::{Sha256, Digest};
use std::fs::File;
use std::path::Path;

/// Calculates sha256 of a file at the given path.
pub fn calculate_sha256<P: AsRef<Path>>(file_path: P) -> std::io::Result<String> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let digest = hasher.finalize();
    Ok(format!("{:x}", digest))
}

/// Retuns git2 Reposiotory instance of the current work directory.
pub fn repo_from_cwd() -> Result<Repository, Error> {
    let cwd = env::current_dir().map_err(|e| Error::from_str(&e.to_string()))?;
    Repository::discover(&cwd)
}

// Converts CompressionMethod to u16.
// pub fn compression_to_u16(method: CompressionMethod) -> Result<u16, String> {
//     match method {
//         CompressionMethod::Stored    => Ok(0),
//         CompressionMethod::Deflated  => Ok(8),
//         _ => Err(format!("Unknown compression method: {:?}", method)),
//     }
// }