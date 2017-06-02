use errors::ValidationError;
use serde_json::{Map, Number, Value};
use super::{Condition, Context};
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
    pub fn to_json_object(&self) -> Map<String, Value> {
        unimplemented!()
    }

    pub fn validate(&self, ctx: &Context, json: &Value) -> Result<(), ValidationError> {
        match *self {
            Validator::Anything => Ok(()),
            Validator::Conditions(ref c) => c.iter().map(|c| {
                c.validate(json)
            }).collect::<Result<Vec<_>, _>>().map(|_| ()),
            Validator::Nothing => Err(ValidationError::NoValuesPass(json.clone())),
            Validator::Reference(ref r) => if let Some(schema) = ctx.get(r.to_owned()) {
                unimplemented!()
            } else {
                Err(ValidationError::BadReference(r.clone()))
            },
        }
    }
}
