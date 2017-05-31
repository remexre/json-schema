extern crate json_schema;

#[cfg(feature = "metaschema")]
#[test]
fn metaschema_validates_itself() {
    use json_schema::metaschema::{METASCHEMA, METASCHEMA_VALUE};

    assert!(METASCHEMA.validate(&METASCHEMA_VALUE).is_ok());
}
