use errors::FromValueError;
use serde_json::Value;
use std::collections::BTreeMap;
use super::{JsonSchema, JsonSchemaInner};
use url::Url;

/// The context a JSON Schema is created and run in.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Context {
    schemas: BTreeMap<Url, JsonSchemaInner>,
}

impl Context {
    /// Creates a new Context.
    pub fn new() -> Context {
        Context { schemas: BTreeMap::new() }
    }

    /// Creates a JsonSchema from a JSON value.
    pub fn make_schema<'a>(&'a mut self, base_uri: Url, json: &Value) -> Result<JsonSchema<'a>, FromValueError> {
        let url = self.parse(base_uri, json, 0)?;
        Ok(self.get(url).unwrap())
    }

    /// Gets a JsonSchema from the Context.
    pub fn get<'a>(&'a self, url: Url) -> Option<JsonSchema<'a>> {
        self.schemas.get(&url).map(|inner| {
            JsonSchema {
                ctx: self,
                id: url,
                inner: inner,
            }
        })
    }

    /// Stores a JsonSchema into the context.
    pub(crate) fn put(&mut self, url: Url, schema: JsonSchemaInner) {
        self.schemas.insert(url, schema);
    }
}
