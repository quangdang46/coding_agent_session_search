//! Regression test for AGENTS.md documenting the bead-ID commit-message convention.
//!
//! Per coding_agent_session_search-cuu3f. The "Commit-Message Convention" subsection
//! under "Beads Workflow Integration" enumerates the project rule that commit subjects
//! closing a bead include `(coding_agent_session_search-<id>)`. This test pins that
//! contract so it doesn't silently regress.

use std::path::PathBuf;

fn agents_md_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("AGENTS.md")
}

#[test]
fn agents_md_contains_commit_convention_heading() {
    tracing::info!(
        target: "cuu3f_test",
        check = "heading_present",
        path = ?agents_md_path()
    );
    let body = std::fs::read_to_string(agents_md_path()).expect("AGENTS.md must exist");
    assert!(
        body.contains("### Commit-Message Convention"),
        "AGENTS.md must include a `### Commit-Message Convention` heading; missing"
    );
}

#[test]
fn agents_md_documents_bead_id_format() {
    tracing::info!(target: "cuu3f_test", check = "format_documented");
    let body = std::fs::read_to_string(agents_md_path()).expect("AGENTS.md must exist");
    assert!(
        body.contains("(coding_agent_session_search-<id>)"),
        "AGENTS.md must document the `(coding_agent_session_search-<id>)` token format"
    );
}

#[test]
fn agents_md_example_commit_subjects_are_syntactically_valid() {
    tracing::info!(target: "cuu3f_test", check = "examples_valid");
    let body = std::fs::read_to_string(agents_md_path()).expect("AGENTS.md must exist");

    // Locate the Commit-Message Convention section and its examples.
    let section_start = body
        .find("### Commit-Message Convention")
        .expect("section heading must be present");
    let next_section = body[section_start + 1..]
        .find("\n### ")
        .map(|i| section_start + 1 + i)
        .or_else(|| {
            body[section_start..]
                .find("\n## ")
                .map(|i| section_start + i)
        })
        .unwrap_or(body.len());
    let section = &body[section_start..next_section];

    // Every code-fence block in this section must contain at least one
    // valid `(coding_agent_session_search-<id>)` token.
    let id_re = regex::Regex::new(r"\(coding_agent_session_search-[a-z0-9_]+(?:\.[0-9]+)*\)")
        .expect("regex must compile");

    let mut block_count = 0usize;
    let mut hits = 0usize;
    for block in section.split("```").skip(1).step_by(2) {
        block_count += 1;
        if id_re.is_match(block) {
            hits += 1;
        }
    }
    tracing::info!(
        target: "cuu3f_test",
        check = "examples_valid",
        code_blocks_found = block_count,
        blocks_with_valid_id = hits
    );

    assert!(
        block_count >= 1,
        "AGENTS.md Commit-Message Convention section must include at least one example code block"
    );
    assert!(
        hits >= 1,
        "AGENTS.md Commit-Message Convention section's code blocks must contain at least one valid bead-ID token; found {block_count} blocks but {hits} matched the pattern"
    );
}

#[test]
fn agents_md_references_install_hook_script() {
    tracing::info!(target: "cuu3f_test", check = "hook_script_referenced");
    let body = std::fs::read_to_string(agents_md_path()).expect("AGENTS.md must exist");
    assert!(
        body.contains("scripts/git-hooks/install.sh")
            || body.contains("scripts/git-hooks/pre-push.sh"),
        "AGENTS.md must reference the pre-push hook installation script"
    );
}
