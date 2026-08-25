use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const RECOVERED_BLOODPRG_ROUTINE_COUNT: usize = 337;
const RECOVERED_XDB_ROUTINE_COUNT: usize = 183;
const RECOVERED_MANU3_ROUTINE_COUNT: usize = 12;
const RECOVERED_NATIVE_ROUTINE_COUNT: usize =
    RECOVERED_BLOODPRG_ROUTINE_COUNT + RECOVERED_XDB_ROUTINE_COUNT;
const CURRENT_PORTED_ROUTINE_COUNT: usize = 228;
const CURRENT_ELIMINATED_ROUTINE_COUNT: usize = 5;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tab_separated_rows(path: &Path) -> Vec<BTreeMap<String, String>> {
    let input = std::fs::read_to_string(path).unwrap();
    let mut lines = input.lines();
    let headers: Vec<&str> = lines.next().unwrap().split('\t').collect();
    lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            headers
                .iter()
                .copied()
                .zip(line.split('\t').map(str::to_owned))
                .map(|(key, value)| (key.to_owned(), value))
                .collect()
        })
        .collect()
}

#[test]
fn coverage_ledger_only_accepts_documented_authoritative_routines() {
    let root = workspace_root();
    let bloodprg = tab_separated_rows(&root.join("re/source/bloodprg/candidates/manifest.tsv"));
    let xdb = tab_separated_rows(&root.join("re/source/xdb/candidates/manifest.tsv"));
    assert_eq!(bloodprg.len(), RECOVERED_BLOODPRG_ROUTINE_COUNT);
    assert_eq!(xdb.len(), RECOVERED_XDB_ROUTINE_COUNT);

    let mut recovered = BTreeMap::new();
    for row in bloodprg {
        recovered.insert(
            ("bloodprg".to_owned(), row["entry"].clone()),
            (row["source"].clone(), row["function"].clone()),
        );
    }
    for row in xdb {
        let (component, entry) = row["entry"].split_once(':').unwrap();
        recovered.insert(
            (component.to_owned(), entry.to_owned()),
            (row["source"].clone(), row["function"].clone()),
        );
    }
    assert_eq!(recovered.len(), RECOVERED_NATIVE_ROUTINE_COUNT);

    let ported = tab_separated_rows(&root.join("re/rust-port/ported.tsv"));
    let mut seen = BTreeSet::new();
    for row in &ported {
        let key = (row["component"].clone(), row["entry"].clone());
        assert!(seen.insert(key.clone()), "duplicate Rust port row: {key:?}");
        let expected = recovered
            .get(&key)
            .unwrap_or_else(|| panic!("Rust port row is not in recovered manifests: {key:?}"));
        assert_eq!(
            (&row["source"], &row["function"]),
            (&expected.0, &expected.1)
        );

        let rust_path = root.join(&row["rust_path"]);
        let rust_source = std::fs::read_to_string(&rust_path).unwrap();
        let function_name = row["rust_symbol"].rsplit("::").next().unwrap();
        assert!(
            rust_source.contains(&format!("fn {function_name}")),
            "{} does not define {}",
            rust_path.display(),
            row["rust_symbol"]
        );
        assert!(
            !row["documentation"].trim().is_empty() && rust_source.contains("///"),
            "{} lacks port documentation",
            rust_path.display()
        );

        let (evidence_path, vector_count) = row["evidence"].rsplit_once(':').unwrap();
        assert!(root.join(evidence_path).is_file());
        assert!(vector_count.parse::<usize>().unwrap() > usize::MIN);
    }
    assert_eq!(ported.len(), CURRENT_PORTED_ROUTINE_COUNT);

    let eliminated = tab_separated_rows(&root.join("re/rust-port/eliminated.tsv"));
    for row in &eliminated {
        let key = (row["component"].clone(), row["entry"].clone());
        assert!(
            seen.insert(key.clone()),
            "duplicate Rust mapping row: {key:?}"
        );
        let expected = recovered
            .get(&key)
            .unwrap_or_else(|| panic!("eliminated row is not in recovered manifests: {key:?}"));
        assert_eq!(
            (&row["source"], &row["function"]),
            (&expected.0, &expected.1)
        );
        assert_eq!(
            row["disposition"], "eliminated_flat_memory_adapter",
            "{key:?} has an unsupported elimination disposition"
        );
        assert!(!row["rationale"].trim().is_empty());

        let rust_path = root.join(&row["rust_path"]);
        let rust_source = std::fs::read_to_string(&rust_path).unwrap();
        let function_name = row["rust_symbol"].rsplit("::").next().unwrap();
        assert!(
            rust_source.contains(&format!("fn {function_name}")),
            "{} does not define {}",
            rust_path.display(),
            row["rust_symbol"]
        );

        let (evidence_path, vector_count) = row["evidence"].rsplit_once(':').unwrap();
        assert!(root.join(evidence_path).is_file());
        assert!(vector_count.parse::<usize>().unwrap() > usize::MIN);
    }
    assert_eq!(eliminated.len(), CURRENT_ELIMINATED_ROUTINE_COUNT);
    assert_eq!(
        seen.iter()
            .filter(|(component, _entry)| component == "xdb_manu3")
            .count(),
        RECOVERED_MANU3_ROUTINE_COUNT
    );
}
