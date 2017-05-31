use FromValueError;
use serde_json::Value;
use super::Validator;

type Result<T> = ::std::result::Result<T, FromValueError>;

/// Parses a JSON schema.
/// 
/// `root` should be true if this is a root schema, false for subschemas.
pub fn schema(json: &Value, root: bool) -> Result<Validator> {
    match *json {
        Value::Bool(true) => Ok(Validator::Anything),
        Value::Bool(false) => Ok(Validator::Nothing),
        Value::Object(ref obj) => {
            // Validate the `$schema` field.
            if let Some(val) = obj.get("$schema") {
                if !root {
                    return Err(FromValueError::SubschemaUsesSchemaKeyword(json.clone()));
                }
                if let Value::String(ref schema) = *val {
	                if schema != "http://json-schema.org/draft-06/schema#" {
                        return Err(FromValueError::UnknownSchemaVersion(json.clone(), schema.to_owned()));
                    }
                } else {
                    return Err(FromValueError::InvalidKeywordType(json.clone(), "$schema", val.clone()));
                }
            }

            // TODO
            unimplemented!()
        },
        _ => Err(FromValueError::InvalidSchemaType(json.clone())),
    }
}

