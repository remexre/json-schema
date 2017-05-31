mod validator;

use {FromValueError, ValidationError};
use self::validator::Validator;
use serde_json::Value;

/// A JSON Schema. See the crate's documentation for more information.
#[derive(Clone, Debug, PartialEq)]
pub struct JsonSchema {
    validator: Validator,
}

impl JsonSchema {
    /// Creates a JSON Schema from a JSON value.
    pub fn from_value(json: &Value) -> Result<JsonSchema, FromValueError> {
        let validator = Validator::from_value(json)?;
        Ok(JsonSchema { validator })
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
