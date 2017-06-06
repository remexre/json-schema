use errors::FromValueError;
use json_pointer::JsonPointer;
use serde_json::Value;
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
                // N.B. Infinitely recursive schema are undefined behavior by
                // the spec, but it might be nice to allow them. This resolves
                // `$ref`s at validation-time, which also makes it possible to
                // load external schemas in dependency-insensitive order.
                if let Some(val) = obj.get("$ref") {
                    if let Value::String(ref r) = *val {
                        let r = id.join(r).map_err(|_| {
                            FromValueError::InvalidKeywordValue(json.clone(), "$ref".to_string(), val.clone())
                        })?;
                        (Validator::Reference(r.to_owned()), id, title, description)
                    } else {
                        return Err(FromValueError::InvalidKeywordType(json.clone(), "$ref".to_string(), val.clone()));
                    }
                } else {
                    let mut conditions = Vec::new();

                    // Process the items and additionalItems fields.
                    if let Some(val) = obj.get("items") {
                        let uri = push_uri(id.clone(), "items".to_string());
                        conditions.push(if let Value::Array(ref arr) = *val {
                            let items = arr.iter().enumerate().map(|(i, s)| {
                                let uri = push_uri(uri.clone(), i.to_string());
                                self.parse(uri, s, depth + 1)
                            }).collect::<Result<Vec<_>, _>>()?;
                            let additional_items = if let Some(val) = obj.get("additionalItems") {
                                let uri = push_uri(id.clone(), "additionalItems".to_string());
                                Some(self.parse(uri, val, depth + 1)?)
                            } else {
                                None
                            };
                            Condition::Items(items, additional_items)
                        } else {
                            let items = self.parse(uri, val, depth + 1)?;
                            Condition::Items(Vec::new(), Some(items))
                        })
                    }

                    // Process the properties, patternProperties, and additionalProperties fields.
                    let properties = match obj.get("properties") {
                        Some(&Value::Object(ref obj)) => Some(obj.iter().map(|(k, v)| {
                            let uri = push_uri(id.clone(), k.to_string());
                            self.parse(uri, v, depth + 1)
                                .map(|u| (k.to_owned(), u))
                        }).collect::<Result<_, _>>()?),
                        Some(val) => return Err(FromValueError::InvalidKeywordType(json.clone(), "properties".to_string(), val.clone())),
                        None => None,
                    };
                    let pattern_properties = match obj.get("patternProperties") {
                        Some(&Value::Object(ref obj)) => Some(obj.iter().map(|(k, v)| {
                            let uri = push_uri(id.clone(), k.to_string());
                            self.parse(uri, v, depth + 1).and_then(|u| {
                                match k.parse() {
                                    Ok(re) => Ok((RegexWrapper(re), u)),
                                    Err(e) => Err(FromValueError::BadPattern(json.clone(), e)),
                                }
                            })
                        }).collect::<Result<_, _>>()?),
                        Some(val) => return Err(FromValueError::InvalidKeywordType(json.clone(), "patternProperties".to_string(), val.clone())),
                        None => None,
                    };
                    let additional_properties = match obj.get("additionalProperties") {
                        Some(schema) => {
                            let uri = push_uri(id.clone(), "additionalProperties".to_string());
                            Some(self.parse(uri, schema, depth + 1)?)
                        },
                        None => None,
                    };
                    if properties.is_some() || pattern_properties.is_some() || additional_properties.is_some() {
                        conditions.push(Condition::Properties(properties.unwrap_or_default(), pattern_properties.unwrap_or_default(), additional_properties));
                    }

                    // Process the rest of the fields.
                    for (k, v) in obj {
                        match k.as_ref() {
                            // Implemented conditions
                            "allOf" => if let Value::Array(ref arr) = *v {
                                let schemas = arr.into_iter().enumerate().map(|(i, v)| {
                                    let uri = push_uri(push_uri(id.clone(), "allOf".to_string()), format!("{}", i));
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
                            "const" => conditions.push(Condition::Const(v.clone())),
                            "contains" => {
                                let uri = push_uri(id.clone(), "contains".to_string());
                                let uri = self.parse(uri, v, depth + 1)?;
                                conditions.push(Condition::Contains(uri))
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
                            "maxLength" => if let Value::Number(ref n) = *v {
                                if let Some(n) = n.as_u64() {
                                    conditions.push(Condition::MaxLength(n));
                                } else {
                                    return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                                }
                            } else {
                                return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                            },
                            "maximum" => if let Value::Number(ref n) = *v {
                                conditions.push(Condition::Maximum(n.clone()));
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
                            "minLength" => if let Value::Number(ref n) = *v {
                                if let Some(n) = n.as_u64() {
                                    conditions.push(Condition::MinLength(n));
                                } else {
                                    return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                                }
                            } else {
                                return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                            },
                            "minimum" => if let Value::Number(ref n) = *v {
                                conditions.push(Condition::Minimum(n.clone()));
                            } else {
                                return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                            },
                            "pattern" => if let Value::String(ref s) = *v {
                                let re = s.parse().map_err(|e| FromValueError::BadPattern(json.clone(), e))?;
                                conditions.push(Condition::Pattern(RegexWrapper(re)));
                            } else {
                                return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                            },
                            "required" => if let Value::Array(ref arr) = *v {
                                let mut required = Vec::new();
                                for v in arr {
                                    if let Value::String(ref s) = *v {
                                        required.push(s.to_string());
                                    } else {
                                        return Err(FromValueError::InvalidKeywordType(json.clone(), k.clone(), v.clone()));
                                    }
                                }
                                conditions.push(Condition::Required(required));
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
                            "additionalItems" | "items" => {},
                            "additionalProperties" | "patternProperties" | "properties" => {},
                            "definitions" => {}, // TODO
                            "$schema" | "$ref" | "$id" | "title" | "description" => {}, // Already checked for.
                            "default" | "examples" => {}, // We don't validate these.
                            "format" => {}, // TODO Eventually...
                            // Not implemented or not-in-spec fields
                            _ => {
                                println!("NYI field {}", k);
                                unimplemented!();
                            }
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
