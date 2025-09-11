use git2::Repository;
use std::fs;
mod common;
use std::error::Error;
use docx_git_extension::utils::utils::calculate_sha256;

const BRANCH_NAME: &str = "test-branch";

#[test]
fn test_docx_workflow_success() -> Result<(), Box<dyn Error>> {
    let _workspace = common::TestWorkspace::new();
    let _ = common::setup_repos();
    let local_repo = Repository::open(&common::LOCAL_REPO_PATH)?;
    let remote_repo= Repository::open(&common::REMOTE_REPO_PATH)?;
    _ = common::setup_repo_configuration(&local_repo);
    
    let local_repo_path = &local_repo.path().parent().unwrap();
    let remote_repo_path = &remote_repo.path().parent().unwrap();
    
    let pointer_path =     common::get_absolute_path(common::POINTER_DOCX_PATH);
    let docx_path: std::path::PathBuf =     common::get_absolute_path(common::DOCX_FILE_PATH);
    let pointer_content_path = common::get_absolute_path(common::POINTER_CONTENT_PATH);
    let remote_docx_path = remote_repo_path.join("test_doc.docx");
    let local_docx_path = local_repo_path.join("test_doc.docx");

    let status = common::run_git_command(local_repo_path, &["checkout", "-b", BRANCH_NAME])?;
    assert!(status.success(), "git checkout -b failed");

    _ = fs::copy(&docx_path, &local_docx_path);

    let status = common::run_git_command(local_repo_path, &["add", "test_doc.docx"])?;
    assert!(status.success(), "git add failed");
    let staged_file = local_repo.find_blob(local_repo.index().unwrap().get_path("test_doc.docx".as_ref(), 0).unwrap().id).unwrap();
    let pointer_content = fs::read(&pointer_content_path).unwrap();
    assert_eq!(common::normalize_line_endings(staged_file.content()), common::normalize_line_endings(&pointer_content));
    // Assert cat-file content
    // Assert temp structure exists in docx-git-tree

    let status = common::run_git_command(local_repo_path, &["commit", "-m", "add doc"])?;
    assert!(status.success(), "git commit failed");
    assert!(local_repo.find_reference("refs/docx/test_doc").is_ok());
    // Assert ref exists

    let status = common::run_git_command(local_repo_path, &["push", "--set-upstream", "origin", "test-branch"])?;
    assert!(status.success(), "git push failed");

    let status = common::run_git_command(remote_repo_path, &["checkout", "test-branch"])?;
    assert!(status.success(), "git checkout in remote failed");

    assert_eq!(calculate_sha256(&docx_path)?, calculate_sha256(&local_docx_path)?);
    assert_eq!(calculate_sha256(&pointer_path)?, calculate_sha256(&remote_docx_path)?);
    // Push with set-upstream flag doesn't push the refs as defined in config:
    assert!(!remote_repo.find_reference("refs/docx/test_doc").is_ok());
    
    // Regular push pushes refs, assert it exists on remote:
    let status = common::run_git_command(local_repo_path, &["push"])?;
    assert!(status.success(), "git push failed");
    assert!(remote_repo.find_reference("refs/docx/test_doc").is_ok());

    // Assert docx on remote and content matches
    // Assert custom ref exists on remote

    // Trigger smudge filter, assert file contents:
    let status = common::run_git_command(&local_repo_path, &["checkout", "main"])?;
    assert!(status.success(), "git checkout failed");
    let status = common::run_git_command(&local_repo_path, &["checkout", BRANCH_NAME])?;
    assert_ne!(calculate_sha256(&docx_path)?, calculate_sha256(&local_docx_path)?);


    // TODO: git checkout - ; git checkout -
    // Assert docx hash same as the original
    // Assert ref exists
    
    // TODO: clean clone
    // Assert ref exists
    // Assert docx hash same as the original

    // TODO: invalid inputs?
    Ok(())
}