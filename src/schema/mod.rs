mod condition;
mod context;
mod parse;
mod validator;

use {FromValueError, ValidationError};
use either::Either;
use json_pointer::JsonPointer;
use regex::Regex;
use serde_json::{Number, Map, Value};
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::ops::Deref;
use url::Url;

pub use self::condition::{Condition, RegexWrapper, Type};
pub use self::context::Context;
pub use self::validator::Validator;

/// A JSON Schema. See the crate's documentation for more information and usage
/// examples.
#[derive(Clone, Debug, PartialEq)]
pub struct JsonSchema<'a> {
    ctx: &'a Context,
    id: Url,
    inner: &'a JsonSchemaInner,
}

impl<'a> JsonSchema<'a> {
    /// Creates a JSON value from a JSON Schema. This can be used to serialize
    /// the JsonSchema in lieu of a Serialize impl.
    pub fn to_value(&self) -> Value {
        self.inner.to_value()
    }

    /// Validates a JSON value using this schema.
    pub fn validate(&self, json: &Value) -> Result<(), ValidationError> {
        self.inner.validate(self.ctx, json)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct JsonSchemaInner {
    description: Option<String>,
    title: Option<String>,
    validator: Validator,
}

impl JsonSchemaInner {
    fn to_value(&self) -> Value {
        let map = self.validator.to_json_object();
        // TODO Add the other JSON Schema properties.
        Value::Object(map)
    }

    fn validate(&self, ctx: &Context, json: &Value) -> Result<(), ValidationError> {
        unimplemented!()
    }
}

#[cfg(feature = "metaschema")]
lazy_static! {
    /// A JsonSchema representing the draft-06 metaschema.
    pub static ref METASCHEMA: JsonSchema<'static> = {
        /*
        let ctx = METASCHEMA_CTX.clone();
        let uri = METASCHEMA_URI.to_owned();
        let value = &*METASCHEMA_VALUE;
        JsonSchema::parse(ctx, uri, value, 0)
            .expect("Failed to construct validator from metaschema")
        */
        unimplemented!()
    };

    /// The URI corresponding to the draft-06 metaschema.
    pub static ref METASCHEMA_URI: Url = {
        Url::parse("http://json-schema.org/draft-06/schema#")
            .expect("Failed to parse metaschema's URI")
    };

    /// The JSON value representing the draft-06 metaschema.
    pub static ref METASCHEMA_VALUE: Value = {
        let src = include_str!("../../metaschema.json");
        ::serde_json::from_str(src)
            .expect("Failed to parse metaschema")
    };
}
