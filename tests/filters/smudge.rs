#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io::Read;
    use zip::ZipArchive;
    use docx_git_extension::filters::smudge::{
        create_docx_from_commit, extract_tree, parse_zip_datetime, rezip_preserving_metadata,
    };
    use docx_git_extension::filters::FileInfo;
    use docx_git_extension::utils::utils::calculate_sha256;
    use docx_git_extension::post_commit::post_commit::{create_commit, update_ref};
    use crate::common::{add_and_commit, fixture_repo_path, reset_fixture_repo};

    #[test]
    fn test_parse_zip_datetime() {
        let dt = "(2023, 12, 25, 10, 30, 45)";
        let parsed = parse_zip_datetime(dt).unwrap();
        assert_eq!(parsed, (2023, 12, 25, 10, 30, 45));
    }

    #[test]
    fn test_extract_tree() {
        let repo = reset_fixture_repo();
        add_and_commit(&repo, "subdir/file.txt", "hello", "add file");

        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let tree = head.tree().unwrap();

        let outdir = fixture_repo_path().join("outdir");
        extract_tree(&repo, &tree, &outdir).unwrap();

        let extracted = outdir.join("subdir/file.txt");
        assert!(extracted.exists());
        let content = fs::read_to_string(extracted).unwrap();
        assert_eq!(content, "hello");
    }

    #[test]
    fn test_rezip_preserving_metadata() {
        let tmpdir = fixture_repo_path().join("ziptest");
        fs::create_dir_all(&tmpdir).unwrap();
        let file_path = tmpdir.join("doc.xml");
        fs::write(&file_path, b"<xml>hello</xml>").unwrap();

        let file_info = FileInfo {
            filename: "doc.xml".to_string(),
            datetime: (2023, 1, 1, 12, 0, 0),
            unix_permissions: 0o644,
        };

        let output_docx = tmpdir.join("out.docx");
        rezip_preserving_metadata(&tmpdir, &[file_info], &output_docx).unwrap();

        let f = File::open(&output_docx).unwrap();
        let mut zip = ZipArchive::new(f).unwrap();
        let mut extracted = String::new();
        zip.by_name("doc.xml").unwrap().read_to_string(&mut extracted).unwrap();
        assert!(extracted.contains("hello"));
    }

    #[test]
    fn test_create_docx_from_commit_roundtrip() {
        let repo = reset_fixture_repo();
        add_and_commit(&repo, "word/document.xml", "<xml>docx</xml>", "add docx part");

        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let tree = head.tree().unwrap();

        let commit_oid = create_commit(&repo, "dummy.docx", &tree).unwrap();
        let refname = "refs/docx/test";
        update_ref(&repo, refname, commit_oid);

        let file_info = FileInfo {
            filename: "word/document.xml".to_string(),
            datetime: (2023, 1, 1, 12, 0, 0),
            unix_permissions: 0o644,
        };

        let tmpdir = fixture_repo_path().join("roundtrip");
        fs::create_dir_all(&tmpdir).unwrap();
        let expected_docx = tmpdir.join("expected.docx");
        rezip_preserving_metadata(&tmpdir, &[file_info.clone()], &expected_docx).unwrap();
        let expected_hash = calculate_sha256(&expected_docx).unwrap();

        let result = create_docx_from_commit(&repo, refname, &expected_hash, &[file_info]);
        assert!(result.is_ok());
    }
}
