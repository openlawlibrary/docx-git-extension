//! Filters module implements clean and smudge filter.
use std::env;
use std::io::{self, BufRead, Read, Write};
use std::process;
use std::path::{Path, PathBuf};
use std::fs;
use crate::utils::utils::repo_from_cwd;
use crate::filters::smudge::{create_docx_from_commit, parse_zip_datetime};
use crate::filters::clean::{save_docx_as_git_tree, get_file_info_from_docx, get_eocd_record, write_eocd};
use log::{info, warn, error};

pub mod clean;
pub mod smudge;

/// A structure that contains metadata of xml file wihin a docx.
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// docx file name
    pub filename: String,
    /// Timestamp represented as a tuple of u16 integers
    pub datetime: (u16, u16, u16, u16, u16, u16),
    /// File permissions necessary for deterministic zip
    pub unix_permissions: u32,
}

/// Clean filter entry point. Clean filter functionality is triggered during file staging -
/// contents of staged file are passed to filter as an input stream via stdin. Original input 
/// is transformed to a pointer file content that contains necessary metadata for smudge filter 
/// (reference name, docx file hash and xml files metadata). Contents of the pointer file are 
/// written to stdout and then to the staged file.
pub fn clean_filter(docx_path_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        writeln!(io::stderr(), "Usage: clean_filter <path_to_docx>")?;
        process::exit(1);
    }

    let docx_path = Path::new(docx_path_str);
    match docx_path.to_str() {
        Some(path_str) => info!("{}", &format!("docx_path: {}", path_str)),
        None => error!("docx_path contains invalid UTF-8"),
    }
    let base_name = docx_path.with_extension(""); // Remove extension
    let base_name_str = base_name
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| -> Box<dyn std::error::Error> { "Invalid base name".into() })?;

    let mut stdin = io::stdin().lock();
    let mut docx_bytes = Vec::new();
    stdin.read_to_end(&mut docx_bytes)?;

    let refname = format!("refs/docx/{}", base_name_str);
    println!("DOCX-POINTER:{}", refname);
    // TODO: log pointer

    let repo = repo_from_cwd()?;
    let mut docx_metadata = get_file_info_from_docx(&docx_path)?;

    let tree_oid = save_docx_as_git_tree(&repo, &docx_bytes, &mut docx_metadata)?;

    if let Some(repo_path) = repo.path().to_str() {
        let tree_oid_file = PathBuf::from(repo_path).join("docx-tree-oid");
        if let Err(e) = fs::write(&tree_oid_file, format!("{}\n", tree_oid)) {
            warn!("Warning: failed to write tree_oid_file: {}", e);
        } else {
            info!("Wrote tree_oid to {}", tree_oid_file.display());
        }
    }

    for meta in &docx_metadata {
        let dt = &meta.datetime;
        println!(
            "METADATA:{}|({}, {}, {}, {}, {}, {})|{}",
            meta.filename,
            dt.0, dt.1, dt.2, dt.3, dt.4, dt.5,
            meta.unix_permissions
        );
        info!("Output METADATA line for {}", meta.filename);
    }
    let eocd_record = get_eocd_record(docx_path_str);
    let _ = write_eocd(&eocd_record?);
    Ok(())
}

/// Smudge filter entry point. Smudge filter functionality is triggered during file checkout -
/// contents of the file are passed to filter as an input stream via stdin. Original input is
/// a pointer file containing metadata necessary for docx reconstruction. Once the docx is 
/// reconstructed, its contents are written to stdout and then to the file that is checked out.
pub fn smudge_filter() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let all_lines: Vec<String> = stdin.lock().lines().collect::<Result<_, _>>()?;
    let mut line_iter = all_lines.into_iter();

    let pointer_line = match line_iter.next() {
        Some(line) => line,
        None => {
            // TODO: decide if exit should be here?
            error!("Missing DOCX-POINTER");
            return Ok(());
        }
    };

    if !pointer_line.starts_with("DOCX-POINTER:") {
        error!("Missing DOCX-POINTER");
        // TODO: decide if exit should be here?
        // println!("{}", pointer_line);
        for line in line_iter {
            error!("{}", line);
        }
        return Ok(());
    }

    let refname = pointer_line.splitn(2, ':').nth(1).unwrap().trim().to_string();

    let hash_line = match line_iter.next() {
        Some(line) => line,
        // TODO: decide if exit should be here?
        None => {
            error!("Missing HASH");
            return Ok(());
        }
    };

    if !hash_line.starts_with("HASH:") {
        error!("Missing HASH");
        // TODO: decide if exit should be here?
        // println!("{}", hash_line);
        for line in line_iter {
            error!("{}", line);
        }
        return Ok(());
    }

    let expected_hash = hash_line.splitn(2, ':').nth(1).unwrap().trim().to_string();

    let mut file_info_list = Vec::new();
    for line in line_iter {
        let trimmed = line.trim();
        if !trimmed.starts_with("METADATA:") {
            // TODO: decide if exit should be here?
            error!("Unexpected input line (missing METADATA)");
            // println!("{}", hash_line);
            // println!("{}", line);
            continue;
        }

        let metadata: Vec<&str> = trimmed.splitn(2, ':').nth(1).unwrap().split('|').collect();
        if metadata.len() != 3 {
            // TODO: decide if exit should be here?
            error!("Invalid METADATA format {}", metadata.len());
            continue;
        }

        let datetime = match parse_zip_datetime(metadata[1]) {
            Ok(dt) => dt,
            Err(e) => {
            // TODO: decide if exit should be here?
                error!("Invalid datetime for '{}': {}", metadata[0], e);
                continue;
            }
        };

        let file_info = FileInfo {
            filename: metadata[0].to_string(),
            datetime,
            unix_permissions: metadata[2].parse().unwrap_or(0),
        };

        file_info_list.push(file_info);
    }

    info!(
        "Parsed {} metadata entries",
        file_info_list.len()
    );

    match repo_from_cwd() {
        Ok(repo) => {
            if let Err(e) = create_docx_from_commit(&repo, &refname, &expected_hash, &file_info_list) {
                error!("{}", &format!("Error in create_docx_from_commit: {e:?}"));
            }
        }
        Err(e) => {
            error!("{}", &format!("Error opening repo: {e:?}"));
        }
    }

    Ok(())
}