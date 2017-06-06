use errors::FromValueError;
use serde_json::Value;
use std::collections::BTreeMap;
use super::{JsonSchema, JsonSchemaInner, METASCHEMA_URI};
use url::Url;

/// The context a JSON Schema is created and run in.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Context {
    schemas: BTreeMap<Url, JsonSchemaInner>,
}

impl Context {
    /// Creates a new Context.
    pub fn new() -> Context {
        // Create the context.
        let ctx = Context { schemas: BTreeMap::new() };
        
        // Add the metaschema to the context.
        ctx.make_schema(*METASCHEMA_URI, *METASCHEMA_VALUE)
            .expect("Couldn't build the metaschema?");

        // Return the context.
        ctx
    }

    /// Creates a JsonSchema from a JSON value.
    pub fn make_schema<'a>(&'a mut self, base_uri: Url, json: &Value) -> Result<JsonSchema<'a>, FromValueError> {
        let uri = self.parse(base_uri, json, 0)?;
        Ok(self.get(&uri).unwrap())
    }

    /// Gets a JsonSchema from the Context.
    pub fn get<'a>(&'a self, uri: &Url) -> Option<JsonSchema<'a>> {
        if *uri == *METASCHEMA_URI {
            unimplemented!()
        } else {
            self.schemas.get(uri).map(|inner| {
                JsonSchema {
                    ctx: self,
                    id: uri.clone(),
                    inner: inner,
                }
            })
        }
    }

    /// Stores a JsonSchema into the context.
    pub(crate) fn put(&mut self, uri: Url, schema: JsonSchemaInner) {
        self.schemas.insert(uri, schema);
    }
}
