use fe_analyzer::namespace::items::ModuleId;
use fe_analyzer::Db;
use fe_common::assert_strings_eq;
use fe_common::diagnostics::print_diagnostics;
use fe_common::files::{FileStore, SourceFileId};
use fe_common::utils::ron::{to_ron_string_pretty, Diff};
use fe_parser::ast as fe;
use regex::Regex;
use rstest::rstest;
use wasm_bindgen_test::wasm_bindgen_test;

fn lower_file(src: &str, id: SourceFileId, files: &FileStore) -> fe::Module {
    let fe_module = parse_file(src, id, files);
    let (db, module_id) = analyze(fe_module, id, files);
    fe_lowering::lower(&db, module_id)
}

fn analyze(module: fe::Module, id: SourceFileId, files: &FileStore) -> (Db, ModuleId) {
    let db = Db::default();
    match fe_analyzer::analyze(&db, module) {
        Ok(id) => (db, id),
        Err(diagnostics) => {
            print_diagnostics(&diagnostics, id, files);
            panic!("analysis failed");
        }
    }
}

fn parse_file(src: &str, id: SourceFileId, files: &FileStore) -> fe::Module {
    match fe_parser::parse_file(src) {
        Ok((module, diags)) if diags.is_empty() => module,
        Ok((_, diags)) | Err(diags) => {
            print_diagnostics(&diags, id, files);
            panic!("failed to parse file");
        }
    }
}

fn replace_spans(input: String) -> String {
    let span_re = Regex::new(r"\n *span: Span\(\n\s*start: \d+,\n\s*end: \d+.\n\s*\),").unwrap();
    span_re.replace_all(&input, "").to_string()
}

#[rstest(
    fixture,
    case("aug_assign"),
    case("base_tuple"),
    case("list_expressions"),
    case("return_unit"),
    case("unit_implicit"),
    case("init"),
    case("custom_empty_type"),
    case("nested_tuple"),
    case("map_tuple"),
    case("type_alias_tuple")
//    case("array_tuple") // TODO: analysis fails on "arrays can only hold primitive types"
)]
#[wasm_bindgen_test]
fn test_lowering(fixture: &str) {
    let mut files = FileStore::new();

    let path = format!("lowering/{}.fe", fixture);
    let src = test_files::fixture(&path);
    let src_id = files.add_file(src, &path);

    let path = format!("lowering/{}_lowered.fe", fixture);
    let expected_lowered = test_files::fixture(&path);
    let el_id = files.add_file(&path, expected_lowered);

    let expected_lowered_ast = parse_file(expected_lowered, el_id, &files);
    let actual_lowered_ast = lower_file(src, src_id, &files);

    assert_strings_eq!(
        replace_spans(to_ron_string_pretty(&expected_lowered_ast).unwrap()),
        replace_spans(to_ron_string_pretty(&actual_lowered_ast).unwrap()),
    );

    // TODO: the analyzer rejects lowered nested tuples, because
    //  nested structs aren't supported yet. we should move the
    //  not-yet-implemented error to the compiler.
    //
    // fe_analyzer::analyze(&actual_lowered_ast, src_id)
    //     .expect("analysis of the lowered module failed");
}
