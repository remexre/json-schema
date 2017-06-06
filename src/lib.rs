//! A crate for parsing and using JSON Schemas, as specified in
//! [draft-wright-json-schema-01](https://tools.ietf.org/html/draft-wright-json-schema-01) and
//! [draft-wright-json-schema-validation-01](https://tools.ietf.org/html/draft-wright-json-schema-validation-01).
//! 
//! [![Build Status](https://travis-ci.org/remexre/json-schema.svg?branch=master)](https://travis-ci.org/remexre/json-schema)
//! [![crates.io](https://img.shields.io/crates/v/json-schema.svg)](https://crates.io/crates/json-schema)
//! [![Documentation](https://docs.rs/json-schema/badge.svg)](https://docs.rs/json-schema)
//! 
//! **TODO Document**

#![deny(missing_docs)]

extern crate either;
extern crate json_pointer;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate serde_json;
extern crate url;

mod errors;
mod schema;

pub use errors::{FromValueError, ValidationError};
pub use schema::{Condition, Context, JsonSchema, Type};
