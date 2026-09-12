use std::collections::HashMap;

use commander_blood_script_compiler::{
    compile_program, decompile_big_bug_bang_cod,
    decompile_structured_big_bug_bang_cod_with_symbols, parse_source_dictionary,
    parse_source_directory,
};

#[test]
fn sequel_operations_are_editable_statements() {
    let bytes = [
        0xD3, 0x10, 0, 0, 2, 0, 1, 0x20, 0, 0xD4, 0x10, 0, 0xFF, 0xFF, 0xD5, 0x10, 0, 0xD6, 0x10,
        0, 0xFF, 0xFF, 0xD7, 0xFF,
    ];
    let dictionary = HashMap::new();
    let result = decompile_big_bug_bang_cod(&bytes, &dictionary).unwrap();
    assert_eq!(result.raw_bytes, 0);
    assert_eq!(result.generic_op_statements, 0);
    for command in [
        "multiply_divide",
        "population_growth",
        "settle_descendants",
        "population_conflict",
        "ending",
    ] {
        assert!(result.source.contains(command), "{}", result.source);
    }
    assert_eq!(compile_program(&result.source, &dictionary).unwrap(), bytes);
    let edited = result.source.replace(
        "population_growth 0x0010 0xFFFF",
        "population_growth 0x0010 0x0002",
    );
    assert_ne!(edited, result.source);
    let mut expected = bytes;
    expected[12..14].copy_from_slice(&2u16.to_le_bytes());
    assert_eq!(compile_program(&edited, &dictionary).unwrap(), expected);
}

#[test]
fn sequel_numeric_zero_operand_does_not_terminate_text() {
    let bytes = [
        0xA6, 0, 0, 0, 0, 0x80, 1, 0, 0, 0, 0xFE, 0xFF, 0, 0, 0xD7, 0xFF,
    ];
    let dictionary = HashMap::new();
    let result = decompile_big_bug_bang_cod(&bytes, &dictionary).unwrap();
    assert_eq!(result.raw_bytes, 0, "{}", result.source);
    assert!(result.source.contains("state_number(0x0000)"));
    assert!(result.source.contains("inventory_choices"));
    assert_eq!(compile_program(&result.source, &dictionary).unwrap(), bytes);
}

#[test]
#[ignore = "requires original BBB COD profiles under output/big-bug-bang/imported-assets/resources"]
fn all_sequel_cod_profiles_round_trip_source() {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let directory = workspace.join("output/big-bug-bang/imported-assets/resources");
    let mut totals = [0usize; 6];
    for profile in 1..=17 {
        let bytes = std::fs::read(directory.join(format!("SCRIPT{profile}.COD"))).unwrap();
        let dic = std::fs::read(directory.join(format!("SCRIPT{profile}.DIC"))).unwrap();
        let dictionary = parse_source_dictionary(&dic);
        let deb = std::fs::read(directory.join(format!("SCRIPT{profile}.DEB"))).unwrap();
        let var = std::fs::read(directory.join(format!("SCRIPT{profile}.VAR"))).unwrap();
        let symbols = parse_source_directory(&deb);
        let result =
            decompile_structured_big_bug_bang_cod_with_symbols(&bytes, &var, &dictionary, &symbols)
                .unwrap();
        if profile == 5 {
            assert!(result.source.contains("proc _1fincro enabled"));
            assert!(result.source.contains("proc _2fincro enabled"));
        }
        assert_eq!(result.raw_bytes, 0, "SCRIPT{profile}");
        assert_eq!(result.generic_op_statements, 0, "SCRIPT{profile}");
        assert_eq!(
            compile_program(&result.source, &dictionary).unwrap(),
            bytes,
            "SCRIPT{profile}"
        );
        totals[0] += result.procedures;
        totals[1] += result.structured_guards;
        totals[2] += result.unstructured_guards;
        totals[3] += result.object_aliases;
        totals[4] += result.object_alias_uses;
        totals[5] += result.field_aliases;
        println!(
            "SCRIPT{profile}: {} procedures, {} structured guards, {} rejected guards, {} objects, {} fields",
            result.procedures,
            result.structured_guards,
            result.unstructured_guards,
            result.object_aliases,
            result.field_aliases,
        );
    }
    assert_eq!(
        totals,
        [312, 2_221, 0, 627, 7_684, 1_153],
        "BBB procedure, guard, and typed ownership coverage changed"
    );
}

#[test]
fn sequel_numeric_operand_is_not_a_choice_separator() {
    for operand in [0, 1, 0x8000, 0xFFFE, 0xFFFF] {
        let mut bytes = vec![0xA6, 0, 0, 0, 0, 0x80, 1, 0];
        bytes.extend_from_slice(&u16::to_le_bytes(operand));
        bytes.extend_from_slice(&[0, 0, 0xFF]);
        let dictionary = HashMap::new();
        let result = decompile_big_bug_bang_cod(&bytes, &dictionary).unwrap();
        assert_eq!(result.raw_bytes, 0);
        assert!(!result.source.contains("choice_list"));
        assert_eq!(compile_program(&result.source, &dictionary).unwrap(), bytes);
    }
}

#[test]
fn all_sequel_fixed_instruction_truncations_are_rejected_or_reported() {
    for (opcode, size) in [(0xD3, 9), (0xD4, 5), (0xD5, 3), (0xD6, 5)] {
        for size in 1..size {
            let mut bytes = vec![0; size];
            bytes[0] = opcode;
            if let Ok(result) = decompile_big_bug_bang_cod(&bytes, &HashMap::new()) {
                assert!(result.raw_bytes > 0);
            }
        }
    }
}
