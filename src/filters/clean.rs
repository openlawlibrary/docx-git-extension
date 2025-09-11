//! Clean filter module contains logic for unzipping the docx,
//! saving it to a git tree and creating a pointer file that
//! contains all the necessary metadata for docx reconstruction.
use git2::{Repository, Oid, TreeBuilder, FileMode, Error};
use zip::read::ZipArchive;
use zip::result::ZipResult;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use std::os::unix::fs::PermissionsExt;
use tempfile::NamedTempFile;
use zip::{write::FileOptions, ZipWriter};
use std::convert::TryInto;
use crate::filters::FileInfo;
use crate::utils::utils::calculate_sha256;
use log::{info, warn};

/// Calculates deterministic hash of the docx file and writes it to pointer file.
pub fn write_deterministic_hash(
    src_folder: &Path,
    file_info_list: &mut [FileInfo],
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting calculate_deterministic_hash");

    // Sort by filename for deterministic ordering
    // file_info_list.sort_by_key(|f| f.filename.clone());

    let tmp_docx = NamedTempFile::new()?.keep()?;
    let output_docx_path = PathBuf::from(&tmp_docx.1);

    let tmp_file = File::create(&output_docx_path)?;
    let mut zip_writer = ZipWriter::new(tmp_file);

    for file_info in file_info_list.iter() {
        let file_path = src_folder.join(&file_info.filename);
        if !file_path.is_file() {
            warn!("Missing file for ZIP: {}", file_path.display());
            continue;
        }

        let options = FileOptions::default()
            .last_modified_time(
                zip::DateTime::from_date_and_time(
                    file_info.datetime.0.try_into().unwrap(),
                    file_info.datetime.1.try_into().unwrap(),
                    file_info.datetime.2.try_into().unwrap(),
                    file_info.datetime.3.try_into().unwrap(),
                    file_info.datetime.4.try_into().unwrap(),
                    file_info.datetime.5.try_into().unwrap(),
                )
                .map_err(|_| "Invalid date/time")?,
            )
            .unix_permissions(file_info.unix_permissions);

        zip_writer.start_file(&file_info.filename, options)?;

        let mut f = File::open(&file_path)?;
        std::io::copy(&mut f, &mut zip_writer)?;

        info!("Added file to ZIP: {}", &file_info.filename);
    }

    zip_writer.finish()?;

    let docx_hash = calculate_sha256(&output_docx_path)?;
    info!("Calculated SHA256 hash: {}", docx_hash);
    println!("HASH:{}", docx_hash);

    Ok(())
}

/// Unzips docx file and stores its xml components in a git tree.
/// Retruns git tree oid.
pub fn save_docx_as_git_tree(
    repo: &Repository,
    docx_bytes: &[u8],
    mut file_info_list: &mut [FileInfo],
) -> Result<Oid, Box<dyn std::error::Error>> {
    // Create temporary directory
    let tmp_dir = tempdir()?;
    let tmp_path = tmp_dir.path();
    let perm = fs::Permissions::from_mode(0o755);
    fs::set_permissions(tmp_path, perm)?;
    let docx_path = tmp_path.join("file.docx");

    // Write the .docx bytes to file
    fs::write(&docx_path, docx_bytes)?;

    // Unzip .docx file
    let file = File::open(&docx_path)?;
    let mut zip = ZipArchive::new(file)?;
    let unzip_path = tmp_path.join("unzipped");
    fs::create_dir(&unzip_path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&unzip_path, fs::Permissions::from_mode(0o755))?;
    }

    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let out_path = unzip_path.join(file.name());

        if file.name().ends_with('/') {
            fs::create_dir_all(&out_path)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&out_path, fs::Permissions::from_mode(0o755))?;
            }
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(parent, fs::Permissions::from_mode(0o755))?;
                }
            }
            let mut out_file = File::create(&out_path)?;
            std::io::copy(&mut file, &mut out_file)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                // If unix_mode info is available, prefer that; otherwise 0644
                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&out_path, fs::Permissions::from_mode(mode))?;
                } else {
                    fs::set_permissions(&out_path, fs::Permissions::from_mode(0o644))?;
                }
            }
        }
    }

    // Build the tree
    let mut builder = repo.treebuilder(None)?;
    add_directory_to_tree(repo, &unzip_path, &mut builder)?;

    // Call deterministic hash function
    write_deterministic_hash(&unzip_path, &mut file_info_list)?;

    // Write tree and return OID
    let tree_oid = builder.write()?;
    Ok(tree_oid)
}

/// Creates a git tree from a directory.
pub fn add_directory_to_tree(
    repo: &Repository,
    base_path: &Path,
    builder: &mut TreeBuilder,
) -> Result<(), Error> {
    info!("Adding directory to tree: {}", base_path.display());
    let perm = fs::Permissions::from_mode(0o755);
    fs::set_permissions(base_path, perm).map_err(|e| git2::Error::from_str(&format!("set_permissions failed: {}", e)))?;
    let mut entries = fs::read_dir(base_path)
        .map_err(|e| Error::from_str(&format!("read_dir failed: {}", e)))?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();

        if path.is_file() {
            let content = fs::read(&path)
                .map_err(|e| Error::from_str(&format!("read file {} failed: {}", path.display(), e)))?;
            let oid = repo.blob(&content)?;
            builder.insert(name, oid, FileMode::Blob.into())?;
            info!("Added file to tree: {}", path.display());
        } else if path.is_dir() {
            let mut sub_builder = repo.treebuilder(None)?;
            add_directory_to_tree(repo, &path, &mut sub_builder)?;
            let subtree_oid = sub_builder.write()?;
            builder.insert(name, subtree_oid, FileMode::Tree.into())?;
            info!("Added directory to tree: {}", path.display());
        }
    }

    Ok(())
}

/// Extracts metadata for all xml files that are part of a docx file.
/// Returns a list of FileInfo instances.
pub fn get_file_info_from_docx<P: AsRef<Path>>(docx_path: P) -> ZipResult<Vec<FileInfo>> {
    let file = File::open(docx_path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut file_infos = Vec::new();

    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let name = file.name().to_string();
        let unix_permissions = file.unix_mode().unwrap_or(0) & 0o777;

        let dt = file
            .last_modified()
            .to_time()
            .expect("Timestamp should always be valid");

        let datetime = (
            dt.year() as u16,
            dt.month() as u16,
            dt.day() as u16,
            dt.hour() as u16,
            dt.minute() as u16,
            dt.second() as u16,
        );

        file_infos.push(FileInfo {
            filename: name,
            datetime,
            unix_permissions,
        });
    }

    Ok(file_infos)
}

use std::io::Read;
pub fn get_eocd_record(zip_path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut file = File::open(zip_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // EOCD signature: 0x06054b50 (little-endian: 50 4b 05 06)
    let signature = [0x50, 0x4b, 0x05, 0x06];

    // Search backwards from the end for the EOCD signature
    let pos = buffer
        .windows(4)
        .rposition(|window| window == signature)
        .expect("EOCD signature not found in ZIP");

    // EOCD is variable size; minimum 22 bytes + optional comment length
    let comment_len = u16::from_le_bytes([buffer[pos + 20], buffer[pos + 21]]) as usize;
    let eocd_size = 22 + comment_len;

    let eocd_record = buffer[pos..pos + eocd_size].to_vec();
    Ok(eocd_record)
}

use base64::{engine::general_purpose, Engine as _};
pub fn write_eocd(eocd_bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    // Convert EOCD bytes to Base64
    let eocd_base64 = general_purpose::STANDARD.encode(eocd_bytes);
    
    // You can add other metadata before/after
    println!("EOCD_BASE64:{}", eocd_base64);

    Ok(())
}