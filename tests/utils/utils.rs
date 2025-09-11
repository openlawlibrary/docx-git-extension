use docx_git_extension::utils::utils::{calculate_sha256, repo_from_cwd};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::env;
    use tempfile::tempdir;
    use git2::Repository;

    #[test]
    fn test_calculate_sha256() {
        // Create a temporary file
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        let mut file = File::create(&file_path).unwrap();
        write!(file, "hello world").unwrap();

        // Calculate hash
        let hash = calculate_sha256(&file_path).unwrap();

        // Expected SHA256 for "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_repo_from_cwd_success() {
        // Create a temporary directory and init a repo
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Change CWD into the repo
        let old_cwd = env::current_dir().unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let result = repo_from_cwd();
        assert!(result.is_ok());

        // Restore old cwd
        env::set_current_dir(old_cwd).unwrap();

        // Just to prevent compiler warning
        drop(repo);
    }

    #[test]
    fn test_repo_from_cwd_fail() {
        // Create a temporary non-repo directory
        let dir = tempdir().unwrap();

        // Change CWD into the dir
        let old_cwd = env::current_dir().unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let result = repo_from_cwd();
        assert!(result.is_err());

        // Restore old cwd
        env::set_current_dir(old_cwd).unwrap();
    }
}