extern crate json_schema;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use json_schema::JsonSchema;
use serde_json::{from_reader, Value};
use std::fs::{File, read_dir};

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
        .expect("Couldn't find test suite")
        .filter_map(|r| r.ok())
        .filter(|f| f.file_type()
            .map(|t| t.is_file())
            .unwrap_or(false))
        .map(|f| f.path());
    for path in all_tests {
        let file = File::open(path).unwrap();
        let tests: Vec<Test> = from_reader(file).unwrap();
        for test in tests {
            test_one(test);
        }
    }
}

fn test_one(test: Test) {
    let schema = JsonSchema::from_value(&test.schema);
    println!("{:?}", test);
    unimplemented!();
}
