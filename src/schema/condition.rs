use either::Either;
use errors::ValidationError;
use regex::Regex;
use serde_json::{Number, Value};
use std::collections::BTreeMap;
use std::cmp::Ordering;
use std::ops::Deref;
use super::Context;
use url::Url;

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
    Items(Vec<Url>, Option<Url>),

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
    Contains(Url),
    #[doc(hidden)] // TODO
    MaxProperties(u64),
    #[doc(hidden)] // TODO
    MinProperties(u64),
    #[doc(hidden)] // TODO
    Required(Vec<String>),
    #[doc(hidden)] // TODO
    Properties(BTreeMap<String, Url>, BTreeMap<RegexWrapper, Url>, Option<Url>),
    #[doc(hidden)] // TODO
    Dependencies(BTreeMap<String, Either<String, Url>>),
    #[doc(hidden)] // TODO
    PropertyNames(Url),
    #[doc(hidden)] // TODO
    Enum(Vec<Value>),
    #[doc(hidden)] // TODO
    Const(Value),
    #[doc(hidden)] // TODO
    Type(Vec<Type>),
    #[doc(hidden)] // TODO
    AllOf(Vec<Url>),
    #[doc(hidden)] // TODO
    AnyOf(Vec<Url>),
    #[doc(hidden)] // TODO
    OneOf(Vec<Url>),
    #[doc(hidden)] // TODO
    Not(Url),
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
    pub fn priority(&self) -> usize {
        match *self {
            Condition::Type(..) => 0,
            Condition::ExclusiveMaximum(..) => 10,
            Condition::ExclusiveMinimum(..) => 10,
            Condition::MaxLength(..) => 10,
            Condition::Maximum(..) => 10,
            Condition::MinLength(..) => 10,
            Condition::Minimum(..) => 10,
            Condition::Required(..) => 10,
            Condition::Properties(..) => 20,
            Condition::AllOf(..) => 100,
            Condition::AnyOf(..) => 100,
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
    pub fn validate(&self, context: &Context, json: &Value) -> Result<(), ValidationError> {
        let ok = match *self {
            Condition::AllOf(ref urls) => {
                for url in urls {
                    let schema = context.get(url)
                        .ok_or_else(|| ValidationError::BadReference(url.clone()))?;
                    schema.validate(json)?
                }
                true
            },
            Condition::AnyOf(ref urls) => {
                for url in urls {
                    let schema = context.get(url)
                        .ok_or_else(|| ValidationError::BadReference(url.clone()))?;
                    if schema.validate(json).is_ok() {
                        return Ok(());
                    }
                }
                false
            },
            Condition::Const(ref v) => json == v,
            Condition::Contains(ref uri) => if let Value::Array(ref arr) = *json {
                let schema = context.get(uri)
                    .ok_or_else(|| ValidationError::BadReference(uri.clone()))?;
                arr.iter().any(|v| schema.validate(v).is_ok())
            } else {
                true
            },
            Condition::ExclusiveMinimum(ref m) => if let Value::Number(ref n) = *json {
                n > m
            } else {
                true
            },
            Condition::Items(ref items, ref additional) => if let Value::Array(ref arr) = *json {
                for (i, json) in arr.iter().enumerate() {
                    if let Some(url) = items.get(i).or(additional.as_ref()) {
                        let schema = context.get(url)
                            .ok_or_else(|| ValidationError::BadReference(url.clone()))?;
                        schema.validate(json)?
                    }
                }
                true
            } else {
                true
            },
            Condition::MaxLength(n) => if let Value::String(ref s) = *json {
                (s.chars().count() as u64) <= n
            } else {
                true
            },
            Condition::Maximum(ref m) => if let Value::Number(ref n) = *json {
                n <= m
            } else {
                true
            },
            Condition::MinLength(n) => if let Value::String(ref s) = *json {
                (s.chars().count() as u64) >= n
            } else {
                true
            },
            Condition::Minimum(ref n) => if let Value::Number(ref num) = *json {
                num >= n
            } else {
                true
            },
            Condition::Pattern(RegexWrapper(ref re)) => if let Value::String(ref s) = *json {
                re.is_match(s)
            } else {
                true
            },
            Condition::Properties(ref props, ref patterns, ref additional) => if let Value::Object(ref obj) = *json {
                for (k, json) in obj {
                    let mut is_additional = true;
                    if let Some(url) = props.get(k) {
                        is_additional = false;
                        let schema = context.get(url)
                            .ok_or_else(|| ValidationError::BadReference(url.clone()))?;
                        schema.validate(json)?
                    }
                    for (_, url) in patterns.iter().filter(|&(re, _)| re.is_match(k)) {
                        is_additional = false;
                        let schema = context.get(url)
                            .ok_or_else(|| ValidationError::BadReference(url.clone()))?;
                        schema.validate(json)?
                    }
                    if is_additional {
                        if let Some(url) = additional.as_ref() {
                            let schema = context.get(url)
                                .ok_or_else(|| ValidationError::BadReference(url.clone()))?;
                            schema.validate(json)?
                        }
                    }
                }
                true
            } else {
                true
            },
            Condition::Required(ref props) => if let Value::Object(ref obj) = *json {
                !props.iter().any(|p| obj.get(p).is_none())
            } else {
                true
            },
            Condition::Type(ref types) => types.iter().any(|t| t.type_of(json)),
            _ => panic!("Condition {:?} not implemented", self),
        };
        if ok {
            Ok(())
        } else {
            Err(ValidationError::ConditionFailed(self.clone()))
        }
    }
}

/// The type of a JSON value.
///
/// Under this definition of type, a value may have more than one type. For
/// example, `4` has both the type `Integer` and the type `Number`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    /// Tries to convert the string to a Type, returning None if it does not
    /// match.
    pub fn from_string(s: &str) -> Option<Type> {
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

#[derive(Clone, Debug)]
pub struct RegexWrapper(pub Regex);

impl Deref for RegexWrapper {
    type Target = Regex;
    fn deref(&self) -> &Regex { &self.0 }
}

impl Eq for RegexWrapper {}

impl Ord for RegexWrapper {
    fn cmp(&self, other: &RegexWrapper) -> Ordering {
        unimplemented!()
    }
}

impl PartialEq for RegexWrapper {
    fn eq(&self, other: &RegexWrapper) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialOrd for RegexWrapper {
    fn partial_cmp(&self, other: &RegexWrapper) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
