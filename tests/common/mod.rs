use git2::{Repository, Signature, Oid};
use std::fs::{self, File};
use std::io::Write;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::os::unix::fs::PermissionsExt;
use std::error::Error;
use assert_cmd::cargo::cargo_bin;
use std::process::{Command, ExitStatus};

pub const TEST_WORKSPACE: &str = "tests/test_workspace";
pub const REMOTE_REPO_PATH: &str = "tests/test_workspace/repos/origin";
pub const LOCAL_REPO_PATH: &str = "tests/test_workspace/repos/local";
pub const DOCX_FILE_PATH: &str = "tests/fixtures/static_files/docx/test_doc.docx";
pub const POINTER_DOCX_PATH: &str = "tests/fixtures/static_files/pointer/test_doc.docx";
pub const POINTER_CONTENT_PATH: &str = "tests/fixtures/static_files/pointer/test_doc_pointer.txt";
pub const BINARY_NAME: &str = env!("CARGO_PKG_NAME");
pub const FILTER_CONDITION: &str = "*.docx filter=docx";
pub const POST_COMMIT_TEMPLATE_PATH: &str = "tests/fixtures/static_files/post-commit";

pub struct TestWorkspace;

impl TestWorkspace {
    pub fn new() -> Self {
        let _ = setup_repos();
        Self
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        clear_repos();
    }
}

pub fn get_absolute_path(relative_path: &str) -> PathBuf {
    let absolute_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    absolute_path
}

pub fn normalize_line_endings(bytes: &[u8]) -> Vec<u8> {
    bytes.iter().copied()
        .filter(|&b| b != b'\r')
        .skip_while(|&b| b == b'\n') // optional leading
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .skip_while(|&b| b == b'\n') // optional trailing
        .collect::<Vec<_>>()
}
pub fn run_git_command(repo_path: &Path, args: &[&str]) -> std::io::Result<ExitStatus> {
    let status = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .status()?;

    Ok(status)
}

/// Reset the fixture repo before each test.
/// Deletes `tests/fixtures/git_repo` and initializes a fresh repo with config.
pub fn setup_repos() -> Result<(), git2::Error> {
    let local_repo_path = get_absolute_path(LOCAL_REPO_PATH);
    let remote_repo_path = get_absolute_path(REMOTE_REPO_PATH);

    fs::create_dir_all(&local_repo_path).unwrap();
    fs::create_dir_all(&remote_repo_path).unwrap();

    // Init local (non-bare) and remote (bare)
    let local_repo = Repository::init(&local_repo_path)?;
    let remote_repo = Repository::init(&remote_repo_path)?;

    // Set default branch on remote to "main"
    remote_repo.reference_symbolic(
        "HEAD",
        "refs/heads/main",
        true,
        "Set default branch to main",
    )?;

    // Configure local repo
    let mut config = local_repo.config()?;
    config.set_str("user.name", "Test User")?;
    config.set_str("user.email", "test@example.com")?;
    config.set_str("remote.origin.url", &remote_repo_path.to_str().unwrap())?;

    // Create initial empty commit on "main" in local repo
    let sig = Signature::now("Test User", "test@example.com")?;
    let tree_id = {
        let mut index = local_repo.index()?;
        index.write_tree()?
    };
    let tree = local_repo.find_tree(tree_id)?;
    let commit_id = local_repo.commit(
        Some("HEAD"), // points HEAD to "main"
        &sig,
        &sig,
        "Initial commit",
        &tree,
        &[],
    )?;

    // Ensure branch is "main" and HEAD points there
    let commit = local_repo.find_commit(commit_id)?;
    local_repo.branch("main", &commit, true)?;
    local_repo.set_head("refs/heads/main")?;

    Ok(())
}


pub fn setup_repo_configuration(repo: &Repository) -> Result<(), Box<dyn Error>> {
    // Find binary path:
    let binary_path_buffer = cargo_bin(BINARY_NAME);
    let binary_path = binary_path_buffer.to_str().unwrap();
    
    // Add entries to git config:
    let mut config = repo.config()?;
    let mut cmd = format!("{} clean %f", binary_path);
    _ = config.set_str("filter.docx.clean", &cmd);
    cmd = format!("{} smudge", binary_path);
    _ = config.set_str("filter.docx.smudge", &cmd);
    _ = config.set_str("filter.docx.required", "true");

    // Register extension in .gitattributes:
    let repo_path = repo.path().parent().unwrap();
    let gitattributes = repo_path.join(".gitattributes");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)   // append instead of overwrite
        .open(&gitattributes)
        .unwrap();

    writeln!(file, "{}", FILTER_CONDITION).unwrap();

    // Copy post-commit template:
    let post_commit_template = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(POST_COMMIT_TEMPLATE_PATH);
    let post_commit_hook = repo_path.join(".git/hooks/post-commit");
    _ = fs::copy(&post_commit_template, &post_commit_hook);
    cmd = format!("{} post-commit", binary_path);
    // Add binary invocation to post-commit hook:
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)   // append instead of overwrite
        .open(&post_commit_hook)
        .unwrap();
    writeln!(file, "{}", cmd).unwrap();
    make_executable(post_commit_hook.as_path())?;

    // Copy pre-push hook:
    // let pre_push_hook_source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(PRE_PUSH_HOOK_PATH);
    // let pre_push_hook = repo_path.join(".git/hooks/pre-push");
    // _ = fs::copy(&pre_push_hook_source, &pre_push_hook);
    // make_executable(pre_push_hook.as_path())?;

    // Include custom refs in push/fetch:
    add_custom_refs_to_config(&repo)?;

    Ok(())
}

fn add_custom_refs_to_config(repo: &Repository) -> Result<(), Box<dyn Error>> {
    let mut config = repo.config()?;

    // Add fetch refspec for docx
    config.set_multivar(
        "remote.origin.fetch",
        "^$",
        "+refs/docx/*:refs/docx/*",
    )?;

    // Add push refspec for heads
    config.set_multivar(
        "remote.origin.push",
        "^$",
        "refs/heads/*:refs/heads/*",
    )?;

    // Add push refspec for docx
    config.set_multivar(
        "remote.origin.push",
        "^$",
        "refs/docx/*:refs/docx/*",
    )?;

    Ok(())
}

pub fn clear_repos() {
    let test_workspace_path = get_absolute_path(TEST_WORKSPACE);
    if test_workspace_path.exists() {
        fs::remove_dir_all(&test_workspace_path).unwrap();
    }
}

/// Add a file and commit it.
pub fn add_and_commit(repo: &Repository, path: &str, content: &str, msg: &str) -> Oid {
    let repo_path = repo.workdir().unwrap().join(path);
    // TODO: Check parent?
    fs::create_dir_all(repo_path.parent().unwrap()).unwrap();
    let mut f = File::create(&repo_path).unwrap();
    f.write_all(content.as_bytes()).unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new(path)).unwrap();
    index.write().unwrap();

    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("Test User", "test@example.com").unwrap();

    if let Ok(head) = repo.head() {
        let parent = head.peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &[&parent]).unwrap()
    } else {
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &[]).unwrap()
    }
}

fn make_executable(path: &Path) -> std::io::Result<()> {
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(perms.mode() | 0o111);
    fs::set_permissions(path, perms)?;
    Ok(())
}

/// Helper: create a minimal fake .docx in memory
pub fn create_test_docx(git_repo_root: PathBuf) -> PathBuf {
    
    let path = git_repo_root.join("test_document.docx");
    path
}
