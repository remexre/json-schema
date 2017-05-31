extern crate json_schema;
extern crate serde_json;

use json_schema::JsonSchema;
use serde_json::{from_str, Value};

const METASCHEMA: &str = include_str!("../metaschema.json");

// TODO Reenable
// #[test]
fn metaschema_validates_itself() {
    let metaschema: Value = from_str(METASCHEMA)
        .expect("Failed to parse metaschema as JSON");
    
    let schema = JsonSchema::from_value(&metaschema)
        .expect("Failed to parse metaschema's schema");

    assert!(schema.validate(&metaschema).is_ok());
}
