use serde_json::Value;

/// An error encountered when converting from a
/// [`Value`](https://docs.rs/serde_json/1.0.2/serde_json/enum.Value.html)
/// to a [`JsonSchema`](struct.JsonSchema.html).
///
/// The first value in each variant is the JSON value that failed to convert to
/// a schema or subschema.
#[derive(Clone, Debug, PartialEq)]
pub enum FromValueError {
    /// A value had an invalid type based on the specification.
    ///
    /// The second value is the keyword, and the third was the value that was
    /// present instead.
    InvalidKeywordType(Value, &'static str, Value),

    /// A subschema was invalid, or the schema was invalid at the top level.
    ///
    /// Illegal per [Section 4.4 of the
    /// RFC](https://tools.ietf.org/html/draft-wright-json-schema-01#section-4.4).
    InvalidSchemaType(Value),

    /// The schema failed to validate against the metaschema. This is only
    /// possible with the `metaschema` feature enabled.
    MetaschemaFailedToValidate(ValidationError),

    /// A subschema used the `$schema` keyword.
    ///
    /// Illegal per [Section 7 of the
    /// RFC](tools.ietf.org/html/draft-wright-json-schema-01#section-7).
    SubschemaUsesSchemaKeyword(Value),

    /// An unknown value was specified for `$schema`.
    ///
    /// This crate only supports the draft06 `$schema` value, so this may occur
    /// overly often. File a bug if it bothers you.
    ///
    /// The second value is the value of `$schema` that was present instead of
    /// a supported version.
    UnknownSchemaVersion(Value, String),
}

/// An error encountered when attempting to validate a
/// [`Value`](https://docs.rs/serde_json/1.0.2/serde_json/enum.Value.html)
/// against a [`JsonSchema`](struct.JsonSchema.html).
#[derive(Clone, Debug, PartialEq)]
pub enum ValidationError {
}