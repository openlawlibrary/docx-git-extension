//! Smudge filter module contains logic for reconstuction of
//! the docx file according to data provided in the pointer file.

use std::io::{Read, Write};
use git2::{Repository, Tree, ObjectType};
use std::fs::{self, File};
use std::path::Path;
use tempfile::tempdir;
use zip::{write::FileOptions, ZipWriter};
use std::convert::TryInto;
use crate::filters::FileInfo;
use crate::utils::utils::calculate_sha256;

/// Reads commit oid that is written to a custom reference and finds a 
/// git tree that is referenced by the commit oid. Reconstructs the original
/// docx file from the git tree.
pub fn create_docx_from_commit(
    repo: &Repository,
    refname: &str,
    expected_hash: &str,
    file_info_list: &[FileInfo],
) -> Result<(), Box<dyn std::error::Error>> {
    // log_messsage(&format!("Creating DOCX from ref '{}'", repub fname));

    let reference = repo
        .find_reference(refname)
        .map_err(|e| format!("Failed to find ref '{}': {}", refname, e))?;
    let object = reference.peel(ObjectType::Any)?;

    let tree = match object.kind() {
        Some(ObjectType::Commit) => {
            let commit = object
                .into_commit()
                .map_err(|_| "Expected commit object")?;
            // log_messsage(&format!(
            //     "Resolved {} to commit {}",
            //     refname,
            //     commit.id()
            // ));
            commit.tree()?
        }
        Some(ObjectType::Tree) => {
            match object.into_tree() {
                Ok(tree) => {
                    // log_messsage(&format!("Resolved {} to tree {}", repub fname, tree.id()));
                    tree
                }
                Err(obj) => {
                    return Err(format!("Expected Tree but got {:?}", obj.kind()).into());
                }
            }
        }
        _ => {
            return Err(format!(
                "Unsupported object type at {}: {:?}",
                refname,
                object.kind()
            )
            .into());
        }
    };

    let tmpdir = tempdir()?;
    let tmp_path = tmpdir.path();

    extract_tree(repo, &tree, tmp_path)?;

    let docx_name = refname
        .rsplit('/')
        .next()
        .unwrap_or("output")
        .to_owned() + ".docx";
    let docx_path = tmp_path.join(&docx_name);

    // log_messsage(&format!("docx_name {}", docx_name));

    rezip_preserving_metadata(tmp_path, file_info_list, &docx_path)?;

    let rezipped_sha = calculate_sha256(&docx_path)?;
    let mut buffer = Vec::new();

    if expected_hash == rezipped_sha {
        // log_messsage(&format!("Hash matched: {}", rezipped_sha));
        File::open(&docx_path)?.read_to_end(&mut buffer)?;
    } else {
        // log_messsage(&format!(
        //     "Hash mismatch. Expected: {}, Got: {}",
        //     expected_hash, rezipped_sha
        // ));
    }
    
    std::io::stdout().write_all(&buffer)?;

    Ok(())
}

/// Extracts xml files from a git tree to a specified path.
pub fn extract_tree(repo: &Repository, tree: &Tree, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(path)?;

    // log_messsage(&format!("Extracting tree to {}", path.display()));

    for entry in tree.iter() {
        let name = entry.name().unwrap_or("<invalid>");
        let full_path = path.join(name);
        let obj = entry.to_object(repo)?;

        match obj.kind() {
            Some(ObjectType::Tree) => {
                let subtree = obj.as_tree().expect("Expected tree");
                extract_tree(repo, subtree, &full_path)?;
            }
            Some(ObjectType::Blob) => {
                let blob = obj.as_blob().expect("Expected blob");
                fs::write(&full_path, blob.content())?;
                // log_messsage(&format!("Extracted file: {}", full_path.display()));
            }
            _ => {
                // log_messsage(&format!("Skipping non-blob/tree object: {}", name));
            }
        }
    }

    Ok(())
}

/// Casts date time string to u16 tuple.
pub fn parse_zip_datetime(date_time_str: &str) -> Result<(u16, u16, u16, u16, u16, u16), Box<dyn std::error::Error>> {
    let parts: Vec<u16> = date_time_str
        .trim_matches(|c| c == '(' || c == ')')
        .split(", ")
        .map(|s| s.trim().parse::<u16>())
        .collect::<Result<_, _>>()?;

    if parts.len() != 6 {
        return Err("Invalid datetime format, expected 6 values".into());
    }

    Ok((
        parts[0], // year
        parts[1], // month
        parts[2], // day
        parts[3], // hour
        parts[4], // minute
        parts[5], // second
    ))
}

/// Recreates docx with original metadata from pointer file.
pub fn rezip_preserving_metadata(
    src_folder: &Path,
    file_info_list: &[FileInfo],
    output_docx_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // log_messsage(&format!("Creating ZIP at {}", output_docx_path.display()));

    // Sort for deterministic ordering by filename
    let mut sorted_files = file_info_list.to_vec();
    sorted_files.sort_by_key(|f| f.filename.clone());

    let file = File::create(output_docx_path)?;
    let mut zip = ZipWriter::new(file);

    for file_info in sorted_files.iter() {
        // Convert datetime tuple (u16,u16,u16,u16,u16,u16) into zip::DateTime
        let date_time = match zip::DateTime::from_date_and_time(
            file_info.datetime.0.try_into().unwrap_or(1980),
            file_info.datetime.1.try_into().unwrap_or(1),
            file_info.datetime.2.try_into().unwrap_or(1),
            file_info.datetime.3.try_into().unwrap_or(0),
            file_info.datetime.4.try_into().unwrap_or(0),
            file_info.datetime.5.try_into().unwrap_or(0),
        ) {
            Ok(dt) => dt,
            Err(e) => {
                // log_messsage(&format!("Invalid datetime for '{}': {:?}", file_info.filename, e));
                continue;
            }
        };

        // Map compress_type u16 to CompressionMethod
        // let compress_type = match file_info.compress_type {
        //     0 => CompressionMethod::Stored,
        //     8 => CompressionMethod::Deflated,
        //     other => {
        //         // log_messsage(&format!("Unsupported compression type {} for '{}'", other, file_info.filename));
        //         continue;
        //     }
        // };

        let permissions = file_info.unix_permissions;

        let file_path = src_folder.join(&file_info.filename);
        if !file_path.is_file() {
            // log_messsage(&format!("Missing file for ZIP: {}", file_path.display()));
            continue;
        }

        let mut contents = Vec::new();
        File::open(&file_path)?.read_to_end(&mut contents)?;

        let options = FileOptions::default()
            // .compression_method(compress_type)
            .last_modified_time(date_time)
            .unix_permissions(permissions);

        zip.start_file(&file_info.filename, options)?;
        zip.write_all(&contents)?;
        // log_messsage(&format!("Added to ZIP: {} as {}", file_path.display(), file_info.filename));
    }

    zip.finish()?;
    Ok(())
}