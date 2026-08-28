use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const RECOVERED_BLOODPRG_ROUTINE_COUNT: usize = 337;
const RECOVERED_XDB_ROUTINE_COUNT: usize = 183;
const RECOVERED_MANU3_ROUTINE_COUNT: usize = 12;
const RECOVERED_NATIVE_ROUTINE_COUNT: usize =
    RECOVERED_BLOODPRG_ROUTINE_COUNT + RECOVERED_XDB_ROUTINE_COUNT;
const CURRENT_PORTED_ROUTINE_COUNT: usize = 470;
const CURRENT_ELIMINATED_ROUTINE_COUNT: usize = 50;
const RECOVERED_BLOODPRG_SEMANTIC_ALIAS_COUNT: usize = 71;
const CURRENT_VERIFIED_BLOODPRG_ALIAS_COUNT: usize = 63;

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
        assert!(
            matches!(
                row["disposition"].as_str(),
                "eliminated_flat_memory_adapter"
                    | "eliminated_host_adapter"
                    | "eliminated_authored_no_operation"
            ),
            "{key:?} has unsupported elimination disposition {}",
            row["disposition"]
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

#[test]
fn shared_global_alias_ledger_matches_recovered_bloodprg_headers() {
    let root = workspace_root();
    let recovered = recovered_bloodprg_aliases(&root);
    assert_eq!(recovered.len(), RECOVERED_BLOODPRG_SEMANTIC_ALIAS_COUNT);

    let rows = tab_separated_rows(&root.join("re/rust-port/shared-global-aliases.tsv"));
    let mut ledger = BTreeMap::new();
    let mut verified = usize::MIN;
    for row in rows {
        let key = (row["segment"].clone(), row["offset"].clone());
        let symbols = row["symbols"]
            .split(',')
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        assert!(
            ledger.insert(key.clone(), symbols).is_none(),
            "duplicate shared-global alias row: {key:?}"
        );
        match row["status"].as_str() {
            "pending_review" => {
                assert_eq!(row["canonical_owner"], "-");
                assert_eq!(row["rust_path"], "-");
                assert_eq!(row["evidence"], "-");
            }
            "verified_shared"
            | "verified_lifetime_split"
            | "verified_typed_view"
            | "verified_eliminated_adapter" => {
                verified += 1;
                assert!(!matches!(row["canonical_owner"].as_str(), "" | "-"));
                assert!(root.join(&row["rust_path"]).is_file());
                assert!(!matches!(row["evidence"].as_str(), "" | "-"));
            }
            status => panic!("unsupported shared-global review status {status:?}"),
        }
    }
    assert_eq!(ledger, recovered);
    assert_eq!(verified, CURRENT_VERIFIED_BLOODPRG_ALIAS_COUNT);
}

fn recovered_bloodprg_aliases(root: &Path) -> BTreeMap<(String, String), BTreeSet<String>> {
    let include_root = root.join("re/source/bloodprg/candidates/include");
    let mut declarations = BTreeMap::<(String, String), BTreeSet<String>>::new();
    for entry in std::fs::read_dir(include_root).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("h") {
            continue;
        }
        let source = std::fs::read_to_string(path).unwrap();
        let mut statement = String::new();
        let mut preceding_address = None;
        for line in source.lines() {
            let trimmed = line.trim_start();
            if statement.is_empty() && !trimmed.starts_with("extern ") {
                preceding_address = declaration_address(trimmed);
                continue;
            }
            if !statement.is_empty() {
                statement.push(' ');
            }
            statement.push_str(trimmed);
            if !trimmed.contains(';') {
                continue;
            }
            if let Some((segment, offset)) =
                declaration_address(&statement).or_else(|| preceding_address.take())
            {
                let declaration = statement.split_once(';').unwrap().0;
                let before_array = declaration.split('[').next().unwrap();
                let symbol = before_array
                    .split_whitespace()
                    .last()
                    .unwrap()
                    .trim_start_matches('*')
                    .to_owned();
                declarations
                    .entry((segment.to_owned(), offset))
                    .or_default()
                    .insert(symbol);
            }
            statement.clear();
            preceding_address = None;
        }
    }

    declarations.retain(|_, symbols| {
        symbols
            .iter()
            .filter(|symbol| !symbol.ends_with("_gs") && !symbol.ends_with("_ds"))
            .count()
            > 1
    });
    declarations
}

fn declaration_address(statement: &str) -> Option<(&'static str, String)> {
    const SEGMENT_MARKERS: [(&str, &str); 11] = [
        ("SS=DS:0x", "DATA"),
        ("SS/DS:0x", "DATA"),
        ("DS=GS:0x", "DATA"),
        ("ES=GS:0x", "DATA"),
        ("DS:0x", "DATA"),
        ("GS:0x", "DATA"),
        ("SS:0x", "DATA"),
        ("ES:0x", "DATA"),
        ("FS:0x", "FS"),
        ("CS:0x", "CS"),
        ("game data:0x", "DATA"),
    ];
    for (marker, segment) in SEGMENT_MARKERS {
        let Some(marker_start) = statement.find(marker) else {
            continue;
        };
        let start = marker_start + marker.len();
        let digits = statement[start..]
            .chars()
            .take_while(|character| character.is_ascii_hexdigit())
            .collect::<String>();
        if !digits.is_empty() {
            return Some((segment, format!("0x{}", digits.to_ascii_uppercase())));
        }
    }
    None
}
