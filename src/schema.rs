use {FromValueError, ValidationError};
use either::Either;
use json_pointer::JsonPointer;
use regex::Regex;
use serde_json::{Number, Map, Value};
use std::collections::BTreeMap;
use std::ops::Deref;
use url::Url;

/// A JSON Schema. See the crate's documentation for more information.
#[derive(Clone, Debug, PartialEq)]
pub struct JsonSchema {
    description: Option<String>,
    id: Url,
    title: Option<String>,
    validator: Validator,
}

impl JsonSchema {
    /// Creates a JSON Schema from a JSON value.
    pub fn from_value(base_uri: Url, json: &Value) -> Result<JsonSchema, FromValueError> {
        #[cfg(feature = "metaschema")]
        METASCHEMA.validate(json)
            .map_err(FromValueError::MetaschemaFailedToValidate)?;

        JsonSchema::from_value_unvalidated(base_uri, json, 0)
    }

    fn parse_parts(id: Url, json: &Value, depth: usize) -> Result<(Validator, Url, Option<String>, Option<String>), FromValueError> {
        match *json {
            Value::Bool(true) => Ok((Validator::Anything, id, None, None)),
            Value::Bool(false) => Ok((Validator::Nothing, id, None, None)),
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
                        return Err(FromValueError::InvalidKeywordType(json.clone(), "$schema".to_string(), val.clone()));
                    }
                }
    
                // Get `$id`. We're a little stricter than the RFC; a Schema with
                // an `$id` whose fragment is non-empty will be rejected.
                let id = if let Some(val) = obj.get("$id") {
                    if let Value::String(ref id) = *val {
                        let id = Url::parse(id).map_err(|e| {
                            FromValueError::InvalidId(json.clone(), id.to_owned(), e)
                        })?;
                        // TODO Validate `$id`.
                        id
                    } else {
                        return Err(FromValueError::InvalidKeywordType(json.clone(), "$id".to_string(), val.clone()));
                    }
                } else {
                    id
                };

                // Get the `title`, if it exists.
                let title = if let Some(val) = obj.get("title") {
                    if let Value::String(ref title) = *val {
                        Some(title.to_owned())
                    } else {
                        return Err(FromValueError::InvalidKeywordType(json.clone(), "title".to_string(), val.clone()));
                    }
                } else {
                    None
                };

                // Get the `description`, if it exists.
                let description = if let Some(val) = obj.get("description") {
                    if let Value::String(ref description) = *val {
                        Some(description.to_owned())
                    } else {
                        return Err(FromValueError::InvalidKeywordType(json.clone(), "description".to_string(), val.clone()));
                    }
                } else {
                    None
                };
    
                // Check if this schema is a `$ref`.
                // N.B. Infinitely recursive schema are undefined behavior by the
                // spec, but it might be nice to allow them.
                if let Some(val) = obj.get("$ref") {
                    return if let Value::String(ref r) = *val {
                        let r = id.join(r).map_err(|e| {
                            FromValueError::InvalidKeywordValue(json.clone(), "$ref".to_string(), val.clone())
                        })?;
                        Ok((Validator::Reference(r.to_owned()), id, title, description))
                    } else {
                        Err(FromValueError::InvalidKeywordType(json.clone(), "$ref".to_string(), val.clone()))
                    };
                }
    
                let mut conditions = Vec::new();
                for (k, v) in obj {
                    match k.as_ref() {
                        // Implemented conditions
                        "additionalProperties" => {
                            let uri = push_uri(id.clone(), "additionalProperties".to_string());
                            let schema = JsonSchema::from_value_unvalidated(uri, v, depth + 1)?;
                            conditions.push(Condition::AdditionalProperties(schema));
                        },
                        "allOf" => if let Value::Array(ref arr) = *v {
                            let schemas = arr.into_iter().enumerate().map(|(i, v)| {
                                let uri = push_uri(push_uri(id.clone(), "anyOf".to_string()), format!("{}", i));
                                JsonSchema::from_value_unvalidated(uri, v, depth + 1)
                            }).collect::<Result<Vec<_>, _>>()?;
                            conditions.push(Condition::AllOf(schemas));
                        } else {
                            return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                        },
                        "anyOf" => if let Value::Array(ref arr) = *v {
                            let schemas = arr.into_iter().enumerate().map(|(i, v)| {
                                let uri = push_uri(push_uri(id.clone(), "anyOf".to_string()), format!("{}", i));
                                JsonSchema::from_value_unvalidated(uri, v, depth + 1)
                            }).collect::<Result<Vec<_>, _>>()?;
                            conditions.push(Condition::AnyOf(schemas));
                        } else {
                            return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                        },
                        "exclusiveMaximum" => if let Value::Number(ref n) = *v {
                            conditions.push(Condition::ExclusiveMaximum(n.clone()));
                        } else {
                            return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                        },
                        "exclusiveMinimum" => if let Value::Number(ref n) = *v {
                            conditions.push(Condition::ExclusiveMinimum(n.clone()));
                        } else {
                            return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                        },
                        "maximum" => if let Value::Number(ref n) = *v {
                            conditions.push(Condition::Maximum(n.clone()));
                        } else {
                            return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                        },
                        "minimum" => if let Value::Number(ref n) = *v {
                            conditions.push(Condition::Minimum(n.clone()));
                        } else {
                            return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                        },
                        "minItems" => if let Value::Number(ref n) = *v {
                            if let Some(n) = n.as_u64() {
                                conditions.push(Condition::MinItems(n));
                            } else {
                                return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                            }
                        } else {
                            return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                        },
                        "pattern" => if let Value::String(ref s) = *v {
                            let re = s.parse().map_err(FromValueError::BadPattern)?;
                            conditions.push(Condition::Pattern(RegexWrapper(re)));
                        } else {
                            return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                        },
                        "properties" => if let Value::Object(ref obj) = *v {
                            let props = obj.into_iter().map(|(k, v)| {
                                let uri = push_uri(push_uri(id.clone(), "properties".to_string()), k.to_owned());
                                let schema = JsonSchema::from_value_unvalidated(uri, v, depth + 1)?;
                                Ok((k.to_owned(), schema))
                            }).collect::<Result<BTreeMap<_, _>, _>>()?;
                            conditions.push(Condition::Properties(props));
                        } else {
                            return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                        },
                        "type" => match *v {
                            Value::Array(ref arr) => {
                                let types = arr.into_iter().map(|vv| {
                                    if let Value::String(ref ty) = *vv {
                                        Type::from_string(ty).ok_or_else(|| {
                                            FromValueError::InvalidKeywordValue(json.clone(), k.clone(), v.clone())
                                        })
                                    } else {
                                        Err(FromValueError::InvalidKeywordValue(json.clone(), k.clone(), v.clone()))
                                    }
                                }).collect::<Result<Vec<_>, _>>()?;
                                conditions.push(Condition::Type(types))
                            },
                            Value::String(ref ty) => {
                                let ty = Type::from_string(ty).ok_or_else(|| {
                                    FromValueError::InvalidKeywordValue(json.clone(), k.clone(), v.clone())
                                })?;
                                conditions.push(Condition::Type(vec![ty]))
                            },
                            _ => {
                                return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                            }
                        },
                        // Intentionally ignored fields
                        "definitions" => {}, // TODO
                        "$schema" | "$ref" | "$id" | "title" | "description" => {}, // Already checked for.
                        "default" | "examples" => {}, // We don't validate these.
                        "format" => {}, // TODO Eventually...
                        // Not implemented or not-in-spec fields
                        _ => println!("DEBUG: Ignoring field {}", k),
                    }
                }
                conditions.sort_by_key(|c| c.priority());
                Ok((Validator::Conditions(conditions), id, title, description))
            },
            _ => Err(FromValueError::InvalidSchemaType(json.clone())),
        }
    }

    fn from_value_unvalidated(id: Url, json: &Value, depth: usize) -> Result<JsonSchema, FromValueError> {
        JsonSchema::parse_parts(id, json, depth)
            .map(|(validator, id, title, description)| JsonSchema { description, id, title, validator })
    }

    /// Creates a JSON value from a JSON Schema. This can be used to serialize
    /// the JsonSchema in lieu of a Serialize impl.
    pub fn to_value(&self) -> Value {
        let map = self.validator.to_json_object();
        // TODO Add the other JSON Schema properties.
        Value::Object(map)
    }

    /// Validates a JSON value using this schema.
    pub fn validate(&self, json: &Value) -> Result<(), ValidationError> {
        self.validator.validate(json)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Validator {
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
    fn to_json_object(&self) -> Map<String, Value> {
        unimplemented!()
    }

    fn validate(&self, json: &Value) -> Result<(), ValidationError> {
        match *self {
            Validator::Anything => Ok(()),
            Validator::Conditions(ref c) => c.iter().map(|c| {
                c.validate(json)
            }).collect::<Result<Vec<_>, _>>().map(|_| ()),
            Validator::Nothing => Err(ValidationError::NoValuesPass(json.clone())),
            Validator::Reference(ref _r) => {
                unimplemented!()
            },
        }
    }
}

/// A single constraint put on a value by a schema.
#[derive(Clone, Debug, PartialEq)]
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
    Maximum(Number),

    /// If the given value is a number, it must be less than the given number.
    ///
    /// Defined in [Section 6.3 of the Validation
    /// RFC](https://tools.ietf.org/html/draft-wright-json-schema-validation-01#section-6.3)
    ExclusiveMaximum(Number),

    /// If the given value is a number, it must not be less than the given
    /// number.
    ///
    /// Defined in [Section 6.4 of the Validation
    /// RFC](https://tools.ietf.org/html/draft-wright-json-schema-validation-01#section-6.4)
    Minimum(Number),

    /// If the given value is a number, it must be greater than the given number.
    ///
    /// Defined in [Section 6.5 of the Validation
    /// RFC](https://tools.ietf.org/html/draft-wright-json-schema-validation-01#section-6.5)
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
    Pattern(RegexWrapper),

    /// If the given value is an array, any of its child items whose indices
    /// are also valid indices in the Vec of schemas must validate against that
    /// schema. Any other values in the array must validate against the other
    /// schema, if it is present.
    ///
    /// This cooresponds to the the `items` and `additionalItems` keywords.
    ///
    /// Defined in [Sections 6.9](https://tools.ietf.org/html/draft-wright-json-schema-validation-01#section-6.9)
    /// and [6.10](https://tools.ietf.org/html/draft-wright-json-schema-validation-01#section-6.10)
    /// of the Validation RFC.
    Items(Vec<JsonSchema>, Option<JsonSchema>),

    /// If the given value is an array, it must not have more items than the
    /// given number.
    ///
    /// Defined in [Section 6.11 of the Validation
    /// RFC](https://tools.ietf.org/html/draft-wright-json-schema-validation-01#section-6.11).
    MaxItems(u64),

    /// If the given value is an array, it must not have fewer items than the
    /// given number.
    ///
    /// Defined in [Section 6.12 of the Validation
    /// RFC](https://tools.ietf.org/html/draft-wright-json-schema-validation-01#section-6.12).
    MinItems(u64),

    #[doc(hidden)] // TODO
    UniqueItems(bool),
    #[doc(hidden)] // TODO
    Contains(JsonSchema),
    #[doc(hidden)] // TODO
    MaxProperties(u64),
    #[doc(hidden)] // TODO
    MinProperties(u64),
    #[doc(hidden)] // TODO
    Required(Vec<String>),
    #[doc(hidden)] // TODO
    Properties(BTreeMap<String, JsonSchema>),
    #[doc(hidden)] // TODO
    PatternProperties(BTreeMap<RegexWrapper, JsonSchema>),
    #[doc(hidden)] // TODO
    AdditionalProperties(JsonSchema),
    #[doc(hidden)] // TODO
    Dependencies(BTreeMap<String, Either<String, JsonSchema>>),
    #[doc(hidden)] // TODO
    PropertyNames(JsonSchema),
    #[doc(hidden)] // TODO
    Enum(Vec<Value>),
    #[doc(hidden)] // TODO
    Const(Value),
    #[doc(hidden)] // TODO
    Type(Vec<Type>),
    #[doc(hidden)] // TODO
    AllOf(Vec<JsonSchema>),
    #[doc(hidden)] // TODO
    AnyOf(Vec<JsonSchema>),
    #[doc(hidden)] // TODO
    OneOf(Vec<JsonSchema>),
    #[doc(hidden)] // TODO
    Not(JsonSchema),
}

impl Condition {
    /// Priority is essentially the notion of failing early rather than late --
    /// the more values a condition rules out, the higher its priority. This is
    /// so that sorting a list of `Condition`s with this as a key function will
    /// put them into an order such that front-to-back traversal is
    /// (approximately) the "fastest-failing" approach.
    ///
    /// For reference, 0 represents the highest priority and `std::usize::MAX`
    /// represents the lowest. (That is, the cheaper and most likely to fail
    /// checks should have numerically lower priorities.)
    fn priority(&self) -> usize {
        match *self {
            Condition::Type(_) => 0,
            Condition::AdditionalProperties(_) => 10,
            Condition::Properties(_) => 10,
            _ => {
                println!("No priority set for {:?}, will default to 1000", self);
                1000
            },
        }
    }

    /// Returns key-value pairs cooresponding to this condition.
    ///
    /// The ability to return multiple pairs is required by the Items condition.
    /// TODO It might also be required for Properties.
    fn to_pair(&self) -> (String, Value) {
        unimplemented!()
    }

    /// Validates the value with the condition.
    fn validate(&self, json: &Value) -> Result<(), ValidationError> {
        match *self {
            Condition::ExclusiveMinimum(ref m) => if let Value::Number(ref n) = *json {
                if n > m {
                    Ok(())
                } else {
                    Err(ValidationError::ConditionFailed(self.clone()))
                }
            } else {
                Ok(())
            },
            Condition::Pattern(RegexWrapper(ref re)) => if let Value::String(ref s) = *json {
                if re.is_match(s) {
                    Ok(())
                } else {
                    Err(ValidationError::ConditionFailed(self.clone()))
                }
            } else {
                Ok(())
            },
            Condition::Properties(ref props) => if let Value::Object(ref obj) = *json {
                for (k, s) in props {
                    if let Some(v) = obj.get(k) {
                        s.validate(v)?
                    }
                }
                Ok(())
            } else {
                Ok(())
            },
            Condition::Type(ref types) => if types.iter().any(|t| t.type_of(json)) {
                Ok(())
            } else {
                Err(ValidationError::ConditionFailed(self.clone()))
            },
            _ => panic!("Condition {:?} not implemented", self),
        }
    }
}

/// The type of a JSON value.
///
/// Under this definition of type, a value may have more than one type. For
/// example, `4` has both the type `Integer` and the type `Number`.
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub enum Type {
    /// The type of the `null` value.
    Null,
    /// The type of `true` and `false`.
    Boolean,
    /// The type of all numbers.
    Number,
    /// The type of all integers (between -2^63 and 2^64).
    Integer,
    /// The type of all strings.
    String,
    /// The type of all arrays.
    Array,
    /// The type of all objects.
    Object,
}

impl Type {
    fn from_string(s: &str) -> Option<Type> {
        match s {
            "null" => Some(Type::Null),
            "boolean" => Some(Type::Boolean),
            "number" => Some(Type::Number),
            "integer" => Some(Type::Integer),
            "string" => Some(Type::String),
            "array" => Some(Type::Array),
            "object" => Some(Type::Object),
            _ => None,
        }
    }

    /// Returns if the given JSON value is a member of the given type.
    fn type_of(&self, val: &Value) -> bool {
        match (self, val) {
            (&Type::Null, &Value::Null) => true,
            (&Type::Boolean, &Value::Bool(_)) => true,
            (&Type::Number, &Value::Number(_)) => true,
            (&Type::Integer, &Value::Number(ref n)) => n.is_u64() || n.is_i64(),
            (&Type::String, &Value::String(_)) => true,
            (&Type::Array, &Value::Array(_)) => true,
            (&Type::Object, &Value::Object(_)) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RegexWrapper(Regex);

impl Deref for RegexWrapper {
    type Target = Regex;
    fn deref(&self) -> &Regex { &self.0 }
}

impl PartialEq for RegexWrapper {
    fn eq(&self, other: &RegexWrapper) -> bool {
        self.as_str() == other.as_str()
    }
}

/// Pushes a new component to the JSON pointer in the fragment portion of a
/// URI. If the fragment is not present or not a JSON pointer, overrides it.
fn push_uri(mut uri: Url, component: String) -> Url {
    let mut ptr = uri.fragment().and_then(|f| {
        f.parse::<JsonPointer<_, _>>().ok()
    }).unwrap_or_else(|| "/".parse().unwrap());
    ptr.push(component);

    uri.set_fragment(Some(&ptr.to_string()));
    uri
}

#[cfg(feature = "metaschema")]
lazy_static! {
    /// A JsonSchema representing the draft-06 metaschema.
    pub static ref METASCHEMA: JsonSchema = {
        // This needs to NOT use Validator::from_value, as that would cause an
        // infinite loop.
        JsonSchema::from_value_unvalidated(METASCHEMA_URI.to_owned(), &*METASCHEMA_VALUE, 0).
            expect("Failed to construct validator from metaschema")
    };

    /// The URI corresponding to the draft-06 metaschema.
    pub static ref METASCHEMA_URI: Url = {
        Url::parse("http://json-schema.org/draft-06/schema#")
            .expect("Failed to parse metaschema's URI")
    };

    /// The JSON value representing the draft-06 metaschema.
    pub static ref METASCHEMA_VALUE: Value = {
        let src = include_str!("../metaschema.json");
        ::serde_json::from_str(src)
            .expect("Failed to parse metaschema")
    };
}
