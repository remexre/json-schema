use errors::FromValueError;
use json_pointer::JsonPointer;
use serde_json::Value;
use std::collections::BTreeMap;
use std::ops::Deref;
use super::{Condition, Context, JsonSchemaInner, RegexWrapper, Type, Validator};
use url::Url;

impl Context {
    pub(crate) fn parse(&mut self, id: Url, json: &Value, depth: usize) -> Result<Url, FromValueError> {
        let (validator, id, title, description) = match *json {
            Value::Bool(true) => (Validator::Anything, id, None, None),
            Value::Bool(false) => (Validator::Nothing, id, None, None),
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
                    if let Value::String(ref r) = *val {
                        let r = id.join(r).map_err(|e| {
                            FromValueError::InvalidKeywordValue(json.clone(), "$ref".to_string(), val.clone())
                        })?;
                        (Validator::Reference(r.to_owned()), id, title, description)
                    } else {
                        return Err(FromValueError::InvalidKeywordType(json.clone(), "$ref".to_string(), val.clone()));
                    }
                } else {
                    let mut conditions = Vec::new();
                    for (k, v) in obj {
                        match k.as_ref() {
                            // Implemented conditions
                            "additionalProperties" => {
                                let uri = push_uri(id.clone(), "additionalProperties".to_string());
                                let schema = self.parse(uri, v, depth + 1)?;
                                conditions.push(Condition::AdditionalProperties(schema));
                            },
                            "allOf" => if let Value::Array(ref arr) = *v {
                                let schemas = arr.into_iter().enumerate().map(|(i, v)| {
                                    let uri = push_uri(push_uri(id.clone(), "anyOf".to_string()), format!("{}", i));
                                    self.parse(uri, v, depth + 1)
                                }).collect::<Result<Vec<_>, _>>()?;
                                conditions.push(Condition::AllOf(schemas));
                            } else {
                                return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                            },
                            "anyOf" => if let Value::Array(ref arr) = *v {
                                let schemas = arr.into_iter().enumerate().map(|(i, v)| {
                                    let uri = push_uri(push_uri(id.clone(), "anyOf".to_string()), format!("{}", i));
                                    self.parse(uri, v, depth + 1)
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
                                let re = s.parse().map_err(|e| FromValueError::BadPattern(json.clone(), e))?;
                                conditions.push(Condition::Pattern(RegexWrapper(re)));
                            } else {
                                return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                            },
                            "properties" => if let Value::Object(ref obj) = *v {
                                let props = obj.into_iter().map(|(k, v)| {
                                    let uri = push_uri(push_uri(id.clone(), "properties".to_string()), k.to_owned());
                                    let schema = self.parse(uri, v, depth + 1)?;
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
                    (Validator::Conditions(conditions), id, title, description)
                }
            },
            _ => return Err(FromValueError::InvalidSchemaType(json.clone())),
        };
        self.put(id.clone(), JsonSchemaInner {
            description,
            title,
            validator,
        });
        Ok(id)
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
