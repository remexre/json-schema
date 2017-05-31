mod from_value;

use {FromValueError, ValidationError};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub enum Validator {
    /// Matches all possible values.
    Anything,
    /// Fails to match all possible values.
    Nothing,
}

impl Validator {
    fn bootstrap(json: &Value) -> Result<Validator, FromValueError> {
        from_value::schema(json, true)
    }

    pub fn from_value(json: &Value) -> Result<Validator, FromValueError> {
        #[cfg(feature = "metaschema")]
        check_metaschema(json)?;

        from_value::schema(json, true)
    }

    pub fn to_value(&self) -> Value {
        unimplemented!()
    }

    pub fn validate(&self, json: &Value) -> Result<(), ValidationError> {
        unimplemented!()
    }
}

#[cfg(feature = "metaschema")]
lazy_static! {
    static ref METASCHEMA: Validator = {
        let src = include_str!("../../../metaschema.json");
        let json = ::serde_json::from_str(src)
            .expect("Failed to parse metaschema");
        Validator::bootstrap(&json)
            .expect("Failed to convert metaschema to validator")
    };
}

#[cfg(feature = "metaschema")]
fn check_metaschema(json: &Value) -> Result<(), FromValueError> {
    METASCHEMA.validate(json).map_err(FromValueError::MetaschemaFailedToValidate)
}
