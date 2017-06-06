extern crate json_schema;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate url;

use json_schema::{Context, JsonSchema};
use serde_json::{from_reader, Value};
use std::fs::{File, read_dir};
use url::Url;

#[derive(Clone, Debug, Deserialize)]
struct Test {
    description: String,
    schema: Value,
    tests: Vec<TestCase>,
}

#[derive(Clone, Debug, Deserialize)]
struct TestCase {
    description: String,
    data: Value,
    valid: bool,
}

const TEST_SUITE_DIR: &str = "tests/JSON-Schema-Test-Suite/tests/draft6";

#[test]
fn test_suite() {
    let all_tests = read_dir(TEST_SUITE_DIR)
        .expect("Couldn't find test suite -- did you run 'git submodule init; git submodule update'?")
        .filter_map(|r| r.ok())
        .filter(|f| f.file_type()
            .map(|t| t.is_file())
            .unwrap_or(false))
        .map(|f| f.path());
    let mut ctx = Context::new();
    for path in all_tests {
        let path = path.canonicalize()
            .expect("Couldn't canonicalize path");
        let base_uri = Url::from_file_path(&path)
            .expect("Couldn't create URI for test");
        let file = File::open(path)
            .expect("Couldn't open test file");
        let tests: Vec<Test> = from_reader(file)
            .expect("Couldn't read test cases");
        for (i, test) in tests.into_iter().enumerate() {
            let uri = base_uri.join(&format!("#/{}/schema", i))
                .expect("Couldn't create schema URI");
            test_one(&mut ctx, uri, test);
        }
    }
}

fn test_one(ctx: &mut Context, uri: Url, test: Test) {
    let schema = ctx.make_schema(uri, &test.schema)
        .expect("Invalid schema");
    for case in test.tests {
        test_case(&schema, case)
    }
}

fn test_case(schema: &JsonSchema, case: TestCase) {
    let result = schema.validate(&case.data);
    if case.valid != result.is_ok() {
        let got = if let Err(err) = result {
            format!("failed with {:?}", err)
        } else {
            "succeeded".to_string()
        };
        let should = if case.valid {
            "succeeded"
        } else {
            "failed"
        };
        panic!("Test '{}' {}, should have {}.",
            case.description,
            got,
            should);
    }
}
