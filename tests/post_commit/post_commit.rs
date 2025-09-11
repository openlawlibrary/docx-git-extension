// #[cfg(test)]
// mod tests {
//     use std::fs;
//     use std::path::Path;
//     use git2::Signature;
//     use docx_git_extension::post_commit::post_commit::{
//         create_commit, get_modified_docx_files, parse_ref_from_pointer,
//         read_pointer_file_from_commit, resolve_tree, update_ref,
//     };
//     use crate::common::{add_and_commit, setup_repos};

//     #[test]
//     fn test_get_modified_docx_files() {
//         let repo = setup_repos();

//         add_and_commit(&repo, "file1.docx", "hello", "first commit");
//         let modified1 = get_modified_docx_files(&repo);
//         assert_eq!(modified1.len(), 1);
//         assert!(modified1[0].ends_with("file1.docx"));

//         let path = repo.workdir().unwrap().join("file1.docx");
//         fs::remove_file(&path).unwrap();
//         let mut index = repo.index().unwrap();
//         index.remove_path(Path::new("file1.docx")).unwrap();
//         index.write().unwrap();
//         let tree_oid = index.write_tree().unwrap();
//         let tree = repo.find_tree(tree_oid).unwrap();
//         let sig = Signature::now("Test User", "test@example.com").unwrap();
//         let parent = repo.head().unwrap().peel_to_commit().unwrap();
//         repo.commit(Some("HEAD"), &sig, &sig, "delete", &tree, &[&parent]).unwrap();

//         let modified2 = get_modified_docx_files(&repo);
//         assert!(modified2.is_empty());
//     }

//     #[test]
//     fn test_read_pointer_file_from_commit_and_parse_ref() {
//         let repo = reset_fixture_repo();
//         let content = "DOCX-POINTER: refs/docx/test";
//         add_and_commit(&repo, "pointer.txt", content, "add pointer");

//         let result = read_pointer_file_from_commit(&repo, "pointer.txt");
//         assert!(result.is_some());
//         assert_eq!(result.unwrap(), content);

//         let parsed = parse_ref_from_pointer(content).unwrap();
//         assert_eq!(parsed, "refs/docx/test");
//     }

//     #[test]
//     fn test_create_commit_and_update_ref() {
//         let repo = reset_fixture_repo();
//         add_and_commit(&repo, "file.docx", "hello", "init");

//         let head = repo.head().unwrap().peel_to_commit().unwrap();
//         let tree = head.tree().unwrap();

//         let oid = create_commit(&repo, "file.docx", &tree);
//         assert!(oid.is_some());

//         let refname = "refs/docx/testref";
//         update_ref(&repo, refname, oid.unwrap());
//         let r = repo.find_reference(refname).unwrap();
//         assert_eq!(r.target(), Some(oid.unwrap()));
//     }

//     #[test]
//     fn test_resolve_tree_from_oid_file() {
//         let repo = reset_fixture_repo();
//         add_and_commit(&repo, "file.docx", "hello", "init");

//         let head = repo.head().unwrap().peel_to_commit().unwrap();
//         let tree = head.tree().unwrap();
//         let tree_oid = tree.id();

//         let path = Path::new(repo.path()).join("docx-tree-oid");
//         fs::write(&path, tree_oid.to_string()).unwrap();

//         let resolved = resolve_tree(&repo, "dummyref").unwrap();
//         assert_eq!(resolved.id(), tree_oid);
//     }
// }
