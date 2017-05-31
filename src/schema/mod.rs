mod validator;

use {FromValueError, ValidationError};
pub use self::validator::{METASCHEMA_URI, METASCHEMA_VALUE};
use self::validator::{METASCHEMA_VALIDATOR, Validator};
use serde_json::Value;
use url::Url;

/// A JSON Schema. See the crate's documentation for more information.
#[derive(Clone, Debug, PartialEq)]
pub struct JsonSchema {
    id: Url,
    validator: Validator,
}

impl JsonSchema {
    /// Creates a JSON Schema from a JSON value.
    pub fn from_value(base_uri: &Url, json: &Value) -> Result<JsonSchema, FromValueError> {
        Validator::from_value(base_uri, json)
            .map(|(i, v)| JsonSchema::from_validator(v, i))
    }

    /// Creates a JSON Schema from a Validator.
    fn from_validator(id: Url, validator: Validator) -> JsonSchema {
        JsonSchema { id, validator }
    }

    /// Creates a JSON value from a JSON Schema.
    pub fn to_value(&self) -> Value {
        self.validator.to_value()
    }

    /// Validates a JSON value using this schema.
    pub fn validate(&self, json: &Value) -> Result<(), ValidationError> {
        self.validator.validate(json)
    }
}

#[cfg(feature = "metaschema")]
lazy_static! {
    /// A JsonSchema representing the draft-06 metaschema.
    pub static ref METASCHEMA: JsonSchema = {
        // This is constructed from the METASCHEMA_VALIDATOR instead of from
        // METASCHEMA_VALUE so that there's not an infinite loop trying to
        // validate it against itself -- METASCHEMA_VALIDATOR is defined so as
        // to not be validated.
        JsonSchema::from_validator(METASCHEMA_URI.clone(), METASCHEMA_VALIDATOR.clone())
    };
}
