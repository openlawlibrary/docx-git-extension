//! This module contains logic of post commit hook.

use git2::{Oid, Repository, Signature, Tree, Delta};
use std::fs;
use std::path::Path;
use log::{info, warn, error};

/// Post-commit hook contains logic for commiting a tree that is created during docx unzip,
/// as well as creating a custom reference that contains a commit oid.
pub fn post_commit() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::discover(".").expect("Not a git repository");

    let modified_files = get_modified_docx_files(&repo);

    if modified_files.is_empty() {
        warn!("No .docx files added or modified in last commit.");
        // TODO: return?
    }

    for path in modified_files {
        info!("📄 Processing {}...", path);
        let pointer = match read_pointer_file_from_commit(&repo, &path) {
            Some(p) => p,
            None => continue,
        };

        let refname = match parse_ref_from_pointer(&pointer) {
            Some(r) => r,
            None => {
                // TODO: extend print
                warn!("⚠️ No DOCX-POINTER ref found in {}", path);
                continue;
            }
        };

        match resolve_tree(&repo, &refname) {
            Ok(tree) => {
                if let Some(commit_oid) = create_commit(&repo, &path, &tree) {
                    update_ref(&repo, &refname, commit_oid);
                }
            },
            Err(e) => error!("❌ Error resolving tree for {}: {}", refname, e),
        }
    }

    Ok(())
}

/// Find and return changed files in two lists: - added/modified and deleted files list.
pub fn get_modified_docx_files(repo: &Repository) -> Vec<String> {
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = head.tree().unwrap();

    let diff = if head.parent_count() == 0 {
        repo.diff_tree_to_tree(None, Some(&tree), None).unwrap()
    } else {
        let parent = head.parent(0).unwrap();
        let parent_tree = parent.tree().unwrap();
        repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), None).unwrap()
    };

    let mut modified = vec![];

    diff.foreach(
        &mut |delta, _| {
            let status = delta.status();
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .unwrap();

            if let Some(path_str) = path.to_str() {
                if path_str.ends_with(".docx") {
                    match status {
                        Delta::Modified | Delta::Added => modified.push(path_str.to_string()),
                        _ => {}
                    }
                }
            }

            true
        },
        None,
        None,
        None,
    ).unwrap();

    modified
}

/// Read pointer file at specific revision.
pub fn read_pointer_file_from_commit(repo: &Repository, path: &str) -> Option<String> {
    let commit = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = commit.tree().unwrap();
    if let Ok(entry) = tree.get_path(Path::new(path)) {
        if let Ok(blob) = repo.find_blob(entry.id()) {
            return String::from_utf8(blob.content().to_vec()).ok();
        }
    }
    None
}

/// Parse reference name from pointer file.
pub fn parse_ref_from_pointer(pointer: &str) -> Option<String> {
    pointer
        .lines()
        .find_map(|line| line.strip_prefix("DOCX-POINTER:").map(|s| s.trim().to_string()))
}

/// Resolve docx tree from custom reference. 
pub fn resolve_tree<'a>(repo: &'a Repository, _refname: &str) -> Result<Tree<'a>, Box<dyn std::error::Error>> {
    let tree_oid_path = Path::new(repo.path()).join("docx-tree-oid");
    let tree_oid = fs::read_to_string(tree_oid_path)?.trim().to_string();
    let obj = repo.find_object(Oid::from_str(&tree_oid)?, None)?;
    let tree = match obj.kind() {
        // TODO: should work without Tree branch
        Some(git2::ObjectType::Tree) => obj.peel_to_tree()?,
        Some(git2::ObjectType::Commit) => obj.peel_to_commit()?.tree()?,
        _ => return Err(format!("Unexpected object type: {:?}", obj.kind()).into()),
    };
    Ok(tree)
}

/// Function that mimics git commit-tree command - creates a commit pointing to the docx tree.
/// This ensures that the docx tree is referenced and therefore not deleted by garbage collector.
/// Returns the oid of the created commit.
pub fn create_commit(repo: &Repository, path: &str, tree: &Tree) -> Option<Oid> {
    // TODO: author?
    let author = Signature::now("sbojanic", "sasa-bojanic@hotmail.com").unwrap();
    let commit_msg = format!("Auto-commit for {} tree", path);

    let parents_refs = repo.revparse_single("HEAD")
        .ok()
        .and_then(|obj| obj.into_commit().ok())
        .map(|commit| vec![commit])
        .unwrap_or_else(Vec::new);

    match repo.commit(
        None, // Update HEAD
        &author,
        &author,
        &commit_msg,
        tree,
        &parents_refs.iter().collect::<Vec<_>>(),
    ) {
        Ok(oid) => {
            info!("✅ Created commit {} (HEAD)", oid);
            Some(oid)
        },
        Err(e) => {
            error!("❌ Error creating commit for {}: {}", path, e);
            None
        },
    }
}

/// Updates custom reference - writes an oid of previously created commit to the custom reference.
pub fn update_ref(repo: &Repository, refname: &str, commit_oid: Oid) {
    if repo.find_reference(refname).is_ok() {
        if let Err(e) = repo.find_reference(refname).and_then(|mut r| r.delete()) {
            error!("❌ Failed to delete existing ref {}: {}", refname, e);
            return;
        }
    }

    match repo.reference(refname, commit_oid, true, "Updating DOCX ref") {
        Ok(_) => info!("✅ Updated ref {} to {}", refname, commit_oid),
        Err(e) => error!("❌ Failed to update ref {}: {}", refname, e),
    }
}