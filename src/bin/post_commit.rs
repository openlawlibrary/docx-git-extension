use git2::Repository;
use docx_git_extension::post_commit::post_commit::{create_commit, get_modified_docx_files, parse_ref_from_pointer, read_pointer_file_from_commit, resolve_tree, update_ref};

fn main() {
    let repo = Repository::discover(".").expect("Not a git repository");

    let (modified_files, _deleted_files) = get_modified_docx_files(&repo);

    if modified_files.is_empty() {
        eprintln!("No .docx files added or modified in last commit.");
    }

    for path in modified_files {
        eprintln!("📄 Processing {}...", path);
        let pointer = match read_pointer_file_from_commit(&repo, &path) {
            Some(p) => p,
            None => continue,
        };

        let refname = match parse_ref_from_pointer(&pointer) {
            Some(r) => r,
            None => {
                eprintln!("⚠️ No DOCX-POINTER ref found in {}", path);
                continue;
            }
        };

        match resolve_tree(&repo, &refname) {
            Ok(tree) => {
                if let Some(commit_oid) = create_commit(&repo, &path, &tree) {
                    update_ref(&repo, &refname, commit_oid);
                }
            },
            Err(e) => eprintln!("❌ Error resolving tree for {}: {}", refname, e),
        }
    }
}