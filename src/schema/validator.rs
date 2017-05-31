use {FromValueError, ValidationError};
use regex::Regex;
use serde_json::{Number, Value};
use std::cmp::Ordering;
use super::{JsonSchema, METASCHEMA};
use url::Url;

#[derive(Clone, Debug, PartialEq)]
pub enum Validator {
    /// Matches all possible values.
    Anything,

    /// Matches if all of the conditions hold.
    Conditions(Vec<Condition>),

    /// Fails to match all possible values.
    Nothing,

    /// A reference to a JsonSchema.
    Reference(Url),
}

impl Validator {
    pub fn from_value(base_uri: &Url, json: &Value) -> Result<(Validator, Url), FromValueError> {
        #[cfg(feature = "metaschema")]
        METASCHEMA.validate(json)
            .map_err(FromValueError::MetaschemaFailedToValidate)?;

        Validator::from_value_unvalidated(base_uri, json, 0)
    }

    fn from_value_unvalidated(base_uri: &Url, json: &Value, depth: usize) -> Result<(Validator, Url), FromValueError> {
        (match *json {
            Value::Bool(true) => Ok((Validator::Anything, None)),
            Value::Bool(false) => Ok((Validator::Nothing, None)),
            Value::Object(ref obj) => {
                // Validate the `$schema` field.
                if let Some(val) = obj.get("$schema") {
                    if depth > 0 {
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
    
                // Check if this schema is a `$ref`.
                // N.B. Infinitely recursive schema are undefined behavior by the
                // spec, but it might be nice to allow them.
                if let Some(val) = obj.get("$ref") {
                    return if let Value::String(ref r) = *val {
                        panic!("$ref is not implemented");
                    } else {
                        Err(FromValueError::InvalidKeywordType(json.clone(), "$ref", val.clone()))
                    };
                }
    
                // Get `$id`. We're a little stricter than the RFC; a Schema with
                // an `$id` whose fragment is non-empty will be rejected.
                let id = if let Some(val) = obj.get("$id") {
                    if let Value::String(ref id) = *val {
                        let id = Url::parse(id).map_err(|e| {
                            FromValueError::InvalidId(json.clone(), id.to_owned(), e)
                        })?;
                        // TODO Validate `$id`.
                        Some(id)
                    } else {
                        return Err(FromValueError::InvalidKeywordType(json.clone(), "$id", val.clone()));
                    }
                } else {
                    None
                };
    
                let mut conditions = Vec::new();
                for (k, v) in obj {
                    println!("{}\t{}", k, v);
                }
                Ok((Validator::Conditions(conditions), id))
            },
            _ => Err(FromValueError::InvalidSchemaType(json.clone())),
        }).map(|(v, uri)| (v, uri.unwrap_or_else(|| base_uri.clone())))
    }

    pub fn to_value(&self) -> Value {
        unimplemented!()
    }

    pub fn validate(&self, json: &Value) -> Result<(), ValidationError> {
        unimplemented!()
    }
}

#[derive(Clone, Debug)]
pub enum Condition {
    /// If the type is a number, it must be an integer and a multiple of the
    /// given number.
    ///
    /// Defined in [Section 6.1 of the Validation
    /// RFC](https://tools.ietf.org/html/draft-wright-json-schema-validation-01#section-6.1)
    MultipleOf(u64),

    /// If the given value is a number, it must not be greater than the given
    /// number.
    ///
    /// Defined in [Section 6.2 of the Validation
    /// RFC](https://tools.ietf.org/html/draft-wright-json-schema-validation-01#section-6.2)
    ///
    /// Wait for [serde-rs/json#328](https://github.com/serde-rs/json/issues/328)
    /// to implement "right"; for now it's sketch.
    Maximum(Number),

    /// If the given value is a number, it must be less than the given number.
    ///
    /// Defined in [Section 6.3 of the Validation
    /// RFC](https://tools.ietf.org/html/draft-wright-json-schema-validation-01#section-6.3)
    ///
    /// Wait for [serde-rs/json#328](https://github.com/serde-rs/json/issues/328)
    /// to implement "right"; for now it's sketch.
    ExclusiveMaximum(Number),

    /// If the given value is a number, it must not be less than the given
    /// number.
    ///
    /// Defined in [Section 6.4 of the Validation
    /// RFC](https://tools.ietf.org/html/draft-wright-json-schema-validation-01#section-6.4)
    ///
    /// Wait for [serde-rs/json#328](https://github.com/serde-rs/json/issues/328)
    /// to implement "right"; for now it's sketch.
    Minimum(Number),

    /// If the given value is a number, it must be greater than the given number.
    ///
    /// Defined in [Section 6.5 of the Validation
    /// RFC](https://tools.ietf.org/html/draft-wright-json-schema-validation-01#section-6.5)
    ///
    /// Wait for [serde-rs/json#328](https://github.com/serde-rs/json/issues/328)
    /// to implement "right"; for now it's sketch.
    ExclusiveMinimum(Number),

    /// If the given value is a string, its length must not be greater than the
    /// given value.
    ///
    /// Defined in [Section 6.6 of the Validation
    /// RFC](https://tools.ietf.org/html/draft-wright-json-schema-validation-01#section-6.6).
    MaxLength(u64),

    /// If the given value is a string, its length must not be less than the
    /// given value.
    ///
    /// Defined in [Section 6.7 of the Validation
    /// RFC](https://tools.ietf.org/html/draft-wright-json-schema-validation-01#section-6.7).
    MinLength(u64),

    /// If the given value is a string, it must match the given regex.
    ///
    /// Defined in [Section 6.8 of the Validation
    /// RFC](https://tools.ietf.org/html/draft-wright-json-schema-validation-01#section-6.8).
    Pattern(Regex),

    /// If the given value is an array, any of its child items whose indices
    /// are also valid indices in the Vec of schemas must validate against that
    /// schema. Any other values in the array must validate against the other
    /// schema, if it is present.
    ///
    /// This cooresponds to the the `items` and `additionlItems` keywords.
    ///
    /// Defined in [Sections 6.9](https://tools.ietf.org/html/draft-wright-json-schema-validation-01#section-6.9)
    /// and [6.10](https://tools.ietf.org/html/draft-wright-json-schema-validation-01#section-6.10)
    /// of the Validation RFC.
    Items(Vec<JsonSchema>, Option<JsonSchema>),
}

impl Condition {
    /// Priority is essentially the notion of failing early rather than late --
    /// the more values a condition rules out, the higher its priority. This is
    /// use to implement PartialOrd, so that sorting a list of `Condition`s
    /// will put them into an order such that front-to-back traversal is
    /// (approximately) the "fastest-failing" approach.
    ///
    /// For reference, 0 represents the highest priority and `std::usize::MAX`
    /// represents the lowest. (That is, the cheaper and most likely to fail
    /// checks should have numerically lower priorities.)
    pub fn priority(&self) -> usize {
        match *self {
            _ => unimplemented!()
        }
    }
}

impl PartialEq for Condition {
    fn eq(&self, o: &Condition) -> bool {
        match (self, o) {
            _ => unimplemented!()
        }
    }
}

impl PartialOrd for Condition {
    fn partial_cmp(&self, o: &Condition) -> Option<Ordering> {
        match self.priority().cmp(&o.priority()) {
            Ordering::Equal => None,
            o => Some(o),
        }
    }
}

#[cfg(feature = "metaschema")]
lazy_static! {
    /// The URI corresponding to the draft-06 metaschema.
    pub static ref METASCHEMA_URI: Url = {
        Url::parse("http://json-schema.org/draft-06/schema#")
            .expect("Failed to parse metaschema's URI")
    };

    pub static ref METASCHEMA_VALIDATOR: Validator = {
        // This needs to NOT use Validator::from_value, as that would cause an
        // infinite loop.
        Validator::from_value_unvalidated(&*METASCHEMA_URI, &*METASCHEMA_VALUE, 0)
            .expect("Failed to construct validator from metaschema")
            .0
    };

    /// The JSON value representing the draft-06 metaschema.
    pub static ref METASCHEMA_VALUE: Value = {
        let src = include_str!("../../metaschema.json");
        ::serde_json::from_str(src)
            .expect("Failed to parse metaschema")
    };
}
