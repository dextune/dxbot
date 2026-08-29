//! AT-CONTRACT-001: golden-file contract test.
//!
//! The snapshot generator is the source of truth. On the first run the golden file is
//! written; on every subsequent run the freshly generated snapshot must match it exactly
//! (exact diff). The stored golden file must never be hand-edited.

use application_contract::golden::GoldenContract;
use std::fs;
use std::path::Path;

#[test]
fn golden_contract_snapshot_is_stable() {
    let snapshot = GoldenContract::generate_snapshot();
    assert!(
        !snapshot.is_empty(),
        "generated snapshot must never be empty"
    );

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("golden");
    let golden_path = dir.join("contract-snapshot-v1.json");

    if !golden_path.exists() {
        fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("create golden dir: {e}"));
        fs::write(&golden_path, snapshot.as_bytes())
            .unwrap_or_else(|e| panic!("write golden file {}: {e}", golden_path.display()));
    }

    let expected =
        fs::read_to_string(&golden_path).unwrap_or_else(|e| panic!("read golden file: {e}"));

    // Semantic verification is separate so drift is reported clearly.
    GoldenContract::verify_snapshot(&expected)
        .unwrap_or_else(|e| panic!("verify_snapshot returned an error: {e:?}"));

    // Exact diff against the stored golden file.
    assert_eq!(
        expected,
        snapshot,
        "golden contract snapshot drifted from generator output; \
         regenerate {} from GoldenContract::generate_snapshot()",
        golden_path.display()
    );
}
