#[cfg(test)]
mod tests {
    use git2::{TreeWalkMode, TreeWalkResult};
    use std::fs::{self, File};
    use std::io::Write;
    use docx_git_extension::filters::clean::{
        add_directory_to_tree, get_file_info_from_docx, save_docx_as_git_tree,
        write_deterministic_hash,
    };
    use docx_git_extension::filters::FileInfo;
    use crate::common::{create_test_docx, fixture_repo_path, reset_fixture_repo};
    #[test]
    fn test_get_file_info_from_docx() {
        let path = create_test_docx();
        let infos = get_file_info_from_docx(&path).unwrap();
        assert!(!infos.is_empty());
        assert!(infos.iter().any(|f| f.filename == "word/document.xml"));
        assert!(infos.iter().any(|f| f.filename == "[Content_Types].xml"));
    }

    #[test]
    fn test_write_deterministic_hash() {
        let mut file_infos: Vec<FileInfo>;
        {
            let path = create_test_docx();
            file_infos = get_file_info_from_docx(&path).unwrap();
        }

        let repo_dir = fixture_repo_path().join("workdir");
        for fi in &file_infos {
            let path = repo_dir.join(&fi.filename);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            let mut f = File::create(&path).unwrap();
            writeln!(f, "dummy").unwrap();
        }

        let res = write_deterministic_hash(&repo_dir, &mut file_infos);
        assert!(res.is_ok());
    }

    #[test]
    fn test_add_directory_to_tree() {
        let repo = reset_fixture_repo();
        let tmp_dir = fixture_repo_path().join("files");
        fs::create_dir_all(&tmp_dir).unwrap();
        let file_path = tmp_dir.join("file.txt");
        fs::write(&file_path, "hello").unwrap();

        let mut builder = repo.treebuilder(None).unwrap();
        let res = add_directory_to_tree(&repo, &tmp_dir, &mut builder);
        assert!(res.is_ok());

        let tree_oid = builder.write().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();

        let mut entries = Vec::new();
        tree.walk(TreeWalkMode::PreOrder, |_, entry| {
            if let Some(name) = entry.name() {
                entries.push(name.to_string());
            }
            TreeWalkResult::Ok
        })
        .unwrap();

        assert!(entries.contains(&"file.txt".to_string()));
    }

    #[test]
    fn test_save_docx_as_git_tree() {
        let repo = reset_fixture_repo();
        let path = create_test_docx();
        let mut file_infos = get_file_info_from_docx(&path).unwrap();

        let oid = save_docx_as_git_tree(&repo, &docx_bytes, &mut file_infos).unwrap();
        let tree = repo.find_tree(oid).unwrap();

        let mut names = Vec::new();
        tree.walk(TreeWalkMode::PreOrder, |_, entry| {
            if let Some(name) = entry.name() {
                names.push(name.to_string());
            }
            TreeWalkResult::Ok
        })
        .unwrap();

        assert!(names.contains(&"word".to_string()));
    }
}
