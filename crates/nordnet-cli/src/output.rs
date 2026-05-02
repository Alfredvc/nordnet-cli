//! JSON-to-stdout output module + `--fields` filter.
//!
//! Every CLI subcommand emits its result by serializing a Rust value to
//! JSON and writing it to stdout. The `--fields` flag (a comma-separated
//! list of top-level keys) restricts the output to a subset of the
//! object's fields; arrays of objects are filtered element-wise. Values
//! that are not JSON objects (or arrays of objects) pass through
//! unchanged when the filter is empty, and produce an [`OutputError`]
//! when filtering is requested but inapplicable.

use serde::Serialize;
use serde_json::{Map, Value};
use std::io::{self, Write};
use thiserror::Error;

/// Output errors (typically caller misuse).
#[derive(Debug, Error)]
pub enum OutputError {
    #[error("could not serialize value to JSON: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("write to stdout failed: {0}")]
    Write(#[from] io::Error),
    #[error("--fields requested ({0:?}) but value is not a JSON object or array of objects")]
    FilterInapplicable(Vec<String>),
}

/// Write `value` as pretty JSON to stdout. If `fields` is non-empty,
/// restrict each output object to those top-level keys (preserving the
/// requested order). Trailing newline is appended.
pub fn emit<T: Serialize>(value: &T, fields: &[String]) -> Result<(), OutputError> {
    emit_to(&mut io::stdout(), value, fields)
}

/// As [`emit`] but to an arbitrary writer — used by tests.
pub fn emit_to<W: Write, T: Serialize>(
    writer: &mut W,
    value: &T,
    fields: &[String],
) -> Result<(), OutputError> {
    let json = serde_json::to_value(value)?;
    let filtered = if fields.is_empty() {
        json
    } else {
        apply_fields(json, fields)?
    };
    let pretty = serde_json::to_string_pretty(&filtered)?;
    writer.write_all(pretty.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

fn apply_fields(value: Value, fields: &[String]) -> Result<Value, OutputError> {
    match value {
        Value::Object(map) => Ok(Value::Object(filter_object(map, fields))),
        Value::Array(items) => {
            let filtered: Vec<Value> = items
                .into_iter()
                .map(|v| match v {
                    Value::Object(m) => Value::Object(filter_object(m, fields)),
                    other => other,
                })
                .collect();
            Ok(Value::Array(filtered))
        }
        // Scalars / null with --fields makes no sense: surface as error
        // so the caller learns they're using the flag wrong.
        _ => Err(OutputError::FilterInapplicable(fields.to_vec())),
    }
}

fn filter_object(map: Map<String, Value>, fields: &[String]) -> Map<String, Value> {
    let mut out = Map::with_capacity(fields.len());
    for f in fields {
        if let Some(v) = map.get(f) {
            out.insert(f.clone(), v.clone());
        }
    }
    out
}

/// Parse a comma-separated `--fields` argument string into a vector,
/// stripping whitespace around each entry and dropping empties.
pub fn parse_fields(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct Row {
        id: i64,
        name: String,
        secret: String,
    }

    #[test]
    fn no_fields_emits_full_object() {
        let r = Row {
            id: 1,
            name: "n".into(),
            secret: "s".into(),
        };
        let mut buf = Vec::new();
        emit_to(&mut buf, &r, &[]).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\"id\": 1"));
        assert!(s.contains("\"name\": \"n\""));
        assert!(s.contains("\"secret\": \"s\""));
    }

    #[test]
    fn fields_filter_restricts_object() {
        let r = Row {
            id: 1,
            name: "n".into(),
            secret: "s".into(),
        };
        let mut buf = Vec::new();
        emit_to(&mut buf, &r, &["id".into(), "name".into()]).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\"id\""));
        assert!(s.contains("\"name\""));
        assert!(!s.contains("secret"));
    }

    #[test]
    fn fields_filter_applies_per_array_element() {
        let rows = vec![
            Row {
                id: 1,
                name: "a".into(),
                secret: "x".into(),
            },
            Row {
                id: 2,
                name: "b".into(),
                secret: "y".into(),
            },
        ];
        let mut buf = Vec::new();
        emit_to(&mut buf, &rows, &["name".into()]).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(!s.contains("secret"));
        assert!(s.contains("\"a\""));
        assert!(s.contains("\"b\""));
    }

    #[test]
    fn fields_on_scalar_errors() {
        let mut buf = Vec::new();
        let r = emit_to(&mut buf, &42i64, &["x".into()]);
        assert!(matches!(r, Err(OutputError::FilterInapplicable(_))));
    }

    #[test]
    fn parse_fields_handles_whitespace_and_empties() {
        assert_eq!(parse_fields(""), Vec::<String>::new());
        assert_eq!(
            parse_fields(" id , name ,, value"),
            vec!["id".to_string(), "name".to_string(), "value".to_string()]
        );
    }
}
