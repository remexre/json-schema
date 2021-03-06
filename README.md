# json-schema

A crate for parsing and using JSON Schemas, as specified in
[draft-wright-json-schema-01](https://tools.ietf.org/html/draft-wright-json-schema-01)
and [draft-wright-json-schema-validation-01](https://tools.ietf.org/html/draft-wright-json-schema-validation-01).

[![Build Status](https://travis-ci.org/remexre/json-schema.svg?branch=master)](https://travis-ci.org/remexre/json-schema)
[![crates.io](https://img.shields.io/crates/v/json-schema.svg)](https://crates.io/crates/json-schema)
[![Documentation](https://docs.rs/json-schema/badge.svg)](https://docs.rs/json-schema)

**TODO Document**

This crate requires Rust 1.18.0 or later, due to use of
[`pub_restricted`](https://github.com/rust-lang/rfcs/blob/master/text/1422-pub-restricted.md).

## Known Issues

 - `JsonSchema` does not implement `Deserialize` or `Serialize`, although it does provide
   [`JsonSchema::from_value`](https://docs.rs/json-schema/*/json_schema/struct.JsonSchema.html#method.from_value) and
   [`JsonSchema::to_value`](https://docs.rs/json-schema/*/json_schema/struct.JsonSchema.html#method.to_value).
 - Does not implement the `default` or `examples` keywords.
 - Does not implement the `format` keyword.
