//! Every lint of the pack reads the repository through the view it is handed, so what a unit
//! reads is what its trace says, and a saved finding replays only while its answers are unchanged.

#[test]
fn the_pack_reads_through_the_view_only() {
    mockspace_lint_rules::assert_lints_read_through_the_view(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
}
