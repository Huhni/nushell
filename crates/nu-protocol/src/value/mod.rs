mod range;
mod row;
mod stream;
mod unit;

use chrono::{DateTime, FixedOffset};
use chrono_humanize::HumanTime;
pub use range::*;
pub use row::*;
use serde::{Deserialize, Serialize};
pub use stream::*;
pub use unit::*;

use std::{cmp::Ordering, fmt::Debug};

use crate::ast::{CellPath, PathMember};
use crate::{span, BlockId, Span, Type};

use crate::ShellError;

/// Core structured values that pass through the pipeline in engine-q
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    Bool {
        val: bool,
        span: Span,
    },
    Int {
        val: i64,
        span: Span,
    },
    Filesize {
        val: i64,
        span: Span,
    },
    Duration {
        val: i64,
        span: Span,
    },
    Date {
        val: DateTime<FixedOffset>,
        span: Span,
    },
    Range {
        val: Box<Range>,
        span: Span,
    },
    Float {
        val: f64,
        span: Span,
    },
    String {
        val: String,
        span: Span,
    },
    Record {
        cols: Vec<String>,
        vals: Vec<Value>,
        span: Span,
    },
    Stream {
        stream: ValueStream,
        span: Span,
    },
    List {
        vals: Vec<Value>,
        span: Span,
    },
    Block {
        val: BlockId,
        span: Span,
    },
    Nothing {
        span: Span,
    },
    Error {
        error: ShellError,
    },
    Binary {
        val: Vec<u8>,
        span: Span,
    },
    CellPath {
        val: CellPath,
        span: Span,
    },
}

impl Value {
    pub fn as_string(&self) -> Result<String, ShellError> {
        match self {
            Value::String { val, .. } => Ok(val.to_string()),
            _ => Err(ShellError::CantConvert("string".into(), self.span()?)),
        }
    }

    /// Get the span for the current value
    pub fn span(&self) -> Result<Span, ShellError> {
        match self {
            Value::Error { error } => Err(error.clone()),
            Value::Bool { span, .. } => Ok(*span),
            Value::Int { span, .. } => Ok(*span),
            Value::Float { span, .. } => Ok(*span),
            Value::Filesize { span, .. } => Ok(*span),
            Value::Duration { span, .. } => Ok(*span),
            Value::Date { span, .. } => Ok(*span),
            Value::Range { span, .. } => Ok(*span),
            Value::String { span, .. } => Ok(*span),
            Value::Record { span, .. } => Ok(*span),
            Value::List { span, .. } => Ok(*span),
            Value::Block { span, .. } => Ok(*span),
            Value::Stream { span, .. } => Ok(*span),
            Value::Nothing { span, .. } => Ok(*span),
            Value::Binary { span, .. } => Ok(*span),
            Value::CellPath { span, .. } => Ok(*span),
        }
    }

    /// Update the value with a new span
    pub fn with_span(mut self, new_span: Span) -> Value {
        match &mut self {
            Value::Bool { span, .. } => *span = new_span,
            Value::Int { span, .. } => *span = new_span,
            Value::Float { span, .. } => *span = new_span,
            Value::Filesize { span, .. } => *span = new_span,
            Value::Duration { span, .. } => *span = new_span,
            Value::Date { span, .. } => *span = new_span,
            Value::Range { span, .. } => *span = new_span,
            Value::String { span, .. } => *span = new_span,
            Value::Record { span, .. } => *span = new_span,
            Value::Stream { span, .. } => *span = new_span,
            Value::List { span, .. } => *span = new_span,
            Value::Block { span, .. } => *span = new_span,
            Value::Nothing { span, .. } => *span = new_span,
            Value::Error { .. } => {}
            Value::Binary { span, .. } => *span = new_span,
            Value::CellPath { span, .. } => *span = new_span,
        }

        self
    }

    /// Get the type of the current Value
    pub fn get_type(&self) -> Type {
        match self {
            Value::Bool { .. } => Type::Bool,
            Value::Int { .. } => Type::Int,
            Value::Float { .. } => Type::Float,
            Value::Filesize { .. } => Type::Filesize,
            Value::Duration { .. } => Type::Duration,
            Value::Date { .. } => Type::Date,
            Value::Range { .. } => Type::Range,
            Value::String { .. } => Type::String,
            Value::Record { cols, vals, .. } => {
                Type::Record(cols.clone(), vals.iter().map(|x| x.get_type()).collect())
            }
            Value::List { .. } => Type::List(Box::new(Type::Unknown)), // FIXME
            Value::Nothing { .. } => Type::Nothing,
            Value::Block { .. } => Type::Block,
            Value::Stream { .. } => Type::ValueStream,
            Value::Error { .. } => Type::Error,
            Value::Binary { .. } => Type::Binary,
            Value::CellPath { .. } => Type::CellPath,
        }
    }

    /// Convert Value into string. Note that Streams will be consumed.
    pub fn into_string(self) -> String {
        match self {
            Value::Bool { val, .. } => val.to_string(),
            Value::Int { val, .. } => val.to_string(),
            Value::Float { val, .. } => val.to_string(),
            Value::Filesize { val, .. } => format_filesize(val),
            Value::Duration { val, .. } => format_duration(val),
            Value::Date { val, .. } => HumanTime::from(val).to_string(),
            Value::Range { val, .. } => match val.into_range_iter() {
                Ok(iter) => {
                    format!(
                        "range: [{}]",
                        iter.map(|x| x.into_string())
                            .collect::<Vec<String>>()
                            .join(", ")
                    )
                }
                Err(error) => format!("{:?}", error),
            },
            Value::String { val, .. } => val,
            Value::Stream { stream, .. } => stream.into_string(),
            Value::List { vals: val, .. } => format!(
                "[{}]",
                val.into_iter()
                    .map(|x| x.into_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Record { cols, vals, .. } => format!(
                "{{{}}}",
                cols.iter()
                    .zip(vals.iter())
                    .map(|(x, y)| format!("{}: {}", x, y.clone().into_string()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Value::Block { val, .. } => format!("<Block {}>", val),
            Value::Nothing { .. } => String::new(),
            Value::Error { error } => format!("{:?}", error),
            Value::Binary { val, .. } => format!("{:?}", val),
            Value::CellPath { val, .. } => val.into_string(),
        }
    }

    pub fn collect_string(self) -> String {
        match self {
            Value::Bool { val, .. } => val.to_string(),
            Value::Int { val, .. } => val.to_string(),
            Value::Float { val, .. } => val.to_string(),
            Value::Filesize { val, .. } => format!("{} bytes", val),
            Value::Duration { val, .. } => format!("{} ns", val),
            Value::Date { val, .. } => format!("{:?}", val),
            Value::Range { val, .. } => match val.into_range_iter() {
                Ok(iter) => iter
                    .map(|x| x.into_string())
                    .collect::<Vec<String>>()
                    .join(", "),
                Err(error) => {
                    format!("{:?}", error)
                }
            },
            Value::String { val, .. } => val,
            Value::Stream { stream, .. } => stream.collect_string(),
            Value::List { vals: val, .. } => val
                .into_iter()
                .map(|x| x.collect_string())
                .collect::<Vec<_>>()
                .join("\n"),
            Value::Record { vals, .. } => vals
                .into_iter()
                .map(|y| y.collect_string())
                .collect::<Vec<_>>()
                .join("\n"),
            Value::Block { val, .. } => format!("<Block {}>", val),
            Value::Nothing { .. } => String::new(),
            Value::Error { error } => format!("{:?}", error),
            Value::Binary { val, .. } => format!("{:?}", val),
            Value::CellPath { .. } => self.into_string(),
        }
    }

    /// Create a new `Nothing` value
    pub fn nothing() -> Value {
        Value::Nothing {
            span: Span::unknown(),
        }
    }

    /// Follow a given column path into the value: for example accessing nth elements in a stream or list
    pub fn follow_cell_path(self, cell_path: &[PathMember]) -> Result<Value, ShellError> {
        let mut current = self;
        for member in cell_path {
            // FIXME: this uses a few extra clones for simplicity, but there may be a way
            // to traverse the path without them
            match member {
                PathMember::Int {
                    val: count,
                    span: origin_span,
                } => {
                    // Treat a numeric path member as `nth <val>`
                    match &mut current {
                        Value::List { vals: val, .. } => {
                            if let Some(item) = val.get(*count) {
                                current = item.clone();
                            } else {
                                return Err(ShellError::AccessBeyondEnd(val.len(), *origin_span));
                            }
                        }
                        Value::Stream { stream, .. } => {
                            if let Some(item) = stream.nth(*count) {
                                current = item;
                            } else {
                                return Err(ShellError::AccessBeyondEndOfStream(*origin_span));
                            }
                        }
                        x => {
                            return Err(ShellError::IncompatiblePathAccess(
                                format!("{}", x.get_type()),
                                *origin_span,
                            ))
                        }
                    }
                }
                PathMember::String {
                    val: column_name,
                    span: origin_span,
                } => match &mut current {
                    Value::Record { cols, vals, span } => {
                        let span = *span;
                        let mut found = false;
                        for col in cols.iter().zip(vals.iter()) {
                            if col.0 == column_name {
                                current = col.1.clone();
                                found = true;
                                break;
                            }
                        }

                        if !found {
                            return Err(ShellError::CantFindColumn(*origin_span, span));
                        }
                    }
                    Value::List { vals, span } => {
                        let mut output = vec![];
                        for val in vals {
                            output.push(val.clone().follow_cell_path(&[PathMember::String {
                                val: column_name.clone(),
                                span: *origin_span,
                            }])?);
                            // if let Value::Record { cols, vals, .. } = val {
                            //     for col in cols.iter().enumerate() {
                            //         if col.1 == column_name {
                            //             output.push(vals[col.0].clone());
                            //         }
                            //     }
                            // }
                        }

                        current = Value::List {
                            vals: output,
                            span: *span,
                        };
                    }
                    Value::Stream { stream, span } => {
                        let mut output = vec![];
                        for val in stream {
                            output.push(val.clone().follow_cell_path(&[PathMember::String {
                                val: column_name.clone(),
                                span: *origin_span,
                            }])?);
                            // if let Value::Record { cols, vals, .. } = val {
                            //     for col in cols.iter().enumerate() {
                            //         if col.1 == column_name {
                            //             output.push(vals[col.0].clone());
                            //         }
                            //     }
                            // }
                        }

                        current = Value::List {
                            vals: output,
                            span: *span,
                        };
                    }
                    x => {
                        return Err(ShellError::IncompatiblePathAccess(
                            format!("{}", x.get_type()),
                            *origin_span,
                        ))
                    }
                },
            }
        }

        Ok(current)
    }

    pub fn string(s: &str, span: Span) -> Value {
        Value::String {
            val: s.into(),
            span,
        }
    }

    pub fn is_true(&self) -> bool {
        matches!(self, Value::Bool { val: true, .. })
    }

    pub fn columns(&self) -> Vec<String> {
        match self {
            Value::Record { cols, .. } => cols.clone(),
            _ => vec![],
        }
    }

    pub fn map<F>(self, span: Span, mut f: F) -> Result<Value, ShellError>
    where
        Self: Sized,
        F: FnMut(Self) -> Value + 'static,
    {
        match self {
            Value::List { vals, .. } => Ok(Value::Stream {
                stream: vals.into_iter().map(f).into_value_stream(),
                span,
            }),
            Value::Stream { stream, .. } => Ok(Value::Stream {
                stream: stream.map(f).into_value_stream(),
                span,
            }),
            Value::Range { val, .. } => Ok(Value::Stream {
                stream: val.into_range_iter()?.map(f).into_value_stream(),
                span,
            }),
            v => {
                let output = f(v);
                match output {
                    Value::Error { error } => Err(error),
                    v => Ok(v),
                }
            }
        }
    }

    pub fn flat_map<U, F>(self, span: Span, mut f: F) -> Value
    where
        Self: Sized,
        U: IntoIterator<Item = Value>,
        <U as IntoIterator>::IntoIter: 'static,
        F: FnMut(Self) -> U + 'static,
    {
        match self {
            Value::List { vals, .. } => Value::Stream {
                stream: vals.into_iter().map(f).flatten().into_value_stream(),
                span,
            },
            Value::Stream { stream, .. } => Value::Stream {
                stream: stream.map(f).flatten().into_value_stream(),
                span,
            },
            Value::Range { val, .. } => match val.into_range_iter() {
                Ok(iter) => Value::Stream {
                    stream: iter.map(f).flatten().into_value_stream(),
                    span,
                },
                Err(error) => Value::Error { error },
            },
            v => Value::Stream {
                stream: f(v).into_iter().into_value_stream(),
                span,
            },
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Compare two floating point numbers. The decision interval for equality is dynamically
        // scaled as the value being compared increases in magnitude.
        fn compare_floats(val: f64, other: f64) -> Option<Ordering> {
            let prec = f64::EPSILON.max(val.abs() * f64::EPSILON);

            if (other - val).abs() < prec {
                return Some(Ordering::Equal);
            }

            val.partial_cmp(&other)
        }

        match (self, other) {
            (Value::Bool { val: lhs, .. }, Value::Bool { val: rhs, .. }) => lhs.partial_cmp(rhs),
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => lhs.partial_cmp(rhs),
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                compare_floats(*lhs, *rhs)
            }
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => {
                lhs.partial_cmp(rhs)
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                compare_floats(*lhs as f64, *rhs)
            }
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                compare_floats(*lhs, *rhs as f64)
            }
            (Value::Duration { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                lhs.partial_cmp(rhs)
            }
            (Value::Filesize { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                lhs.partial_cmp(rhs)
            }
            (Value::Block { val: b1, .. }, Value::Block { val: b2, .. }) if b1 == b2 => {
                Some(Ordering::Equal)
            }
            (Value::List { vals: lhs, .. }, Value::List { vals: rhs, .. }) => lhs.partial_cmp(rhs),
            (
                Value::Record {
                    vals: lhs,
                    cols: lhs_headers,
                    ..
                },
                Value::Record {
                    vals: rhs,
                    cols: rhs_headers,
                    ..
                },
            ) if lhs_headers == rhs_headers && lhs == rhs => Some(Ordering::Equal),
            (Value::Stream { stream: lhs, .. }, Value::Stream { stream: rhs, .. }) => {
                lhs.clone().partial_cmp(rhs.clone())
            }
            (Value::Stream { stream: lhs, .. }, Value::String { val: rhs, .. }) => {
                lhs.clone().collect_string().partial_cmp(rhs)
            }
            (Value::String { val: lhs, .. }, Value::Stream { stream: rhs, .. }) => {
                lhs.partial_cmp(&rhs.clone().collect_string())
            }
            // NOTE: This may look a bit strange, but a `Stream` is still just a `List`, it just
            // happens to be in an iterator form instead of a concrete form. The contained values
            // can be compared.
            (Value::Stream { stream: lhs, .. }, Value::List { vals: rhs, .. }) => {
                lhs.clone().collect::<Vec<Value>>().partial_cmp(rhs)
            }
            (Value::List { vals: lhs, .. }, Value::Stream { stream: rhs, .. }) => {
                lhs.partial_cmp(&rhs.clone().collect::<Vec<Value>>())
            }
            (Value::Binary { val: lhs, .. }, Value::Binary { val: rhs, .. }) => {
                lhs.partial_cmp(rhs)
            }
            (_, _) => None,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other).map_or(false, Ordering::is_eq)
    }
}

impl Value {
    pub fn add(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Int {
                val: lhs + rhs,
                span,
            }),
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Float {
                val: *lhs as f64 + *rhs,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Float {
                val: *lhs + *rhs as f64,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Float {
                val: lhs + rhs,
                span,
            }),
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => Ok(Value::String {
                val: lhs.to_string() + rhs,
                span,
            }),
            (Value::Duration { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                Ok(Value::Duration {
                    val: *lhs + *rhs,
                    span,
                })
            }
            (Value::Filesize { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                Ok(Value::Filesize {
                    val: *lhs + *rhs,
                    span,
                })
            }

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
    pub fn sub(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Int {
                val: lhs - rhs,
                span,
            }),
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Float {
                val: *lhs as f64 - *rhs,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Float {
                val: *lhs - *rhs as f64,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Float {
                val: lhs - rhs,
                span,
            }),
            (Value::Duration { val: lhs, .. }, Value::Duration { val: rhs, .. }) => {
                Ok(Value::Duration {
                    val: *lhs - *rhs,
                    span,
                })
            }
            (Value::Filesize { val: lhs, .. }, Value::Filesize { val: rhs, .. }) => {
                Ok(Value::Filesize {
                    val: *lhs - *rhs,
                    span,
                })
            }

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
    pub fn mul(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Int {
                val: lhs * rhs,
                span,
            }),
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Float {
                val: *lhs as f64 * *rhs,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => Ok(Value::Float {
                val: *lhs * *rhs as f64,
                span,
            }),
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => Ok(Value::Float {
                val: lhs * rhs,
                span,
            }),

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
    pub fn div(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match (self, rhs) {
            (Value::Int { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    if lhs % rhs == 0 {
                        Ok(Value::Int {
                            val: lhs / rhs,
                            span,
                        })
                    } else {
                        Ok(Value::Float {
                            val: (*lhs as f64) / (*rhs as f64),
                            span,
                        })
                    }
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Int { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::Float {
                        val: *lhs as f64 / *rhs,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Float { val: lhs, .. }, Value::Int { val: rhs, .. }) => {
                if *rhs != 0 {
                    Ok(Value::Float {
                        val: *lhs / *rhs as f64,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }
            (Value::Float { val: lhs, .. }, Value::Float { val: rhs, .. }) => {
                if *rhs != 0.0 {
                    Ok(Value::Float {
                        val: lhs / rhs,
                        span,
                    })
                } else {
                    Err(ShellError::DivisionByZero(op))
                }
            }

            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
    pub fn lt(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match self.partial_cmp(rhs) {
            Some(ordering) => Ok(Value::Bool {
                val: matches!(ordering, Ordering::Less),
                span,
            }),
            None => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
    pub fn lte(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match self.partial_cmp(rhs) {
            Some(ordering) => Ok(Value::Bool {
                val: matches!(ordering, Ordering::Less | Ordering::Equal),
                span,
            }),
            None => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
    pub fn gt(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match self.partial_cmp(rhs) {
            Some(ordering) => Ok(Value::Bool {
                val: matches!(ordering, Ordering::Greater),
                span,
            }),
            None => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
    pub fn gte(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match self.partial_cmp(rhs) {
            Some(ordering) => Ok(Value::Bool {
                val: matches!(ordering, Ordering::Greater | Ordering::Equal),
                span,
            }),
            None => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
    pub fn eq(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match self.partial_cmp(rhs) {
            Some(ordering) => Ok(Value::Bool {
                val: matches!(ordering, Ordering::Equal),
                span,
            }),
            None => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
    pub fn ne(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match self.partial_cmp(rhs) {
            Some(ordering) => Ok(Value::Bool {
                val: !matches!(ordering, Ordering::Equal),
                span,
            }),
            None => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }

    pub fn r#in(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match (self, rhs) {
            (lhs, Value::Range { val: rhs, .. }) => Ok(Value::Bool {
                val: rhs.contains(lhs),
                span,
            }),
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => Ok(Value::Bool {
                val: rhs.contains(lhs),
                span,
            }),
            (lhs, Value::List { vals: rhs, .. }) => Ok(Value::Bool {
                val: rhs.contains(lhs),
                span,
            }),
            (Value::String { val: lhs, .. }, Value::Record { cols: rhs, .. }) => Ok(Value::Bool {
                val: rhs.contains(lhs),
                span,
            }),
            (lhs, Value::Stream { stream: rhs, .. }) => Ok(Value::Bool {
                val: rhs.clone().any(|x| lhs == &x),
                span,
            }),
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }

    pub fn not_in(&self, op: Span, rhs: &Value) -> Result<Value, ShellError> {
        let span = span(&[self.span()?, rhs.span()?]);

        match (self, rhs) {
            (lhs, Value::Range { val: rhs, .. }) => Ok(Value::Bool {
                val: !rhs.contains(lhs),
                span,
            }),
            (Value::String { val: lhs, .. }, Value::String { val: rhs, .. }) => Ok(Value::Bool {
                val: !rhs.contains(lhs),
                span,
            }),
            (lhs, Value::List { vals: rhs, .. }) => Ok(Value::Bool {
                val: !rhs.contains(lhs),
                span,
            }),
            (Value::String { val: lhs, .. }, Value::Record { cols: rhs, .. }) => Ok(Value::Bool {
                val: !rhs.contains(lhs),
                span,
            }),
            (lhs, Value::Stream { stream: rhs, .. }) => Ok(Value::Bool {
                val: rhs.clone().all(|x| lhs != &x),
                span,
            }),
            _ => Err(ShellError::OperatorMismatch {
                op_span: op,
                lhs_ty: self.get_type(),
                lhs_span: self.span()?,
                rhs_ty: rhs.get_type(),
                rhs_span: rhs.span()?,
            }),
        }
    }
}

/// Format a duration in nanoseconds into a string
pub fn format_duration(duration: i64) -> String {
    let (sign, duration) = if duration >= 0 {
        (1, duration)
    } else {
        (-1, -duration)
    };
    let (micros, nanos): (i64, i64) = (duration / 1000, duration % 1000);
    let (millis, micros): (i64, i64) = (micros / 1000, micros % 1000);
    let (secs, millis): (i64, i64) = (millis / 1000, millis % 1000);
    let (mins, secs): (i64, i64) = (secs / 60, secs % 60);
    let (hours, mins): (i64, i64) = (mins / 60, mins % 60);
    let (days, hours): (i64, i64) = (hours / 24, hours % 24);

    let mut output_prep = vec![];

    if days != 0 {
        output_prep.push(format!("{}day", days));
    }

    if hours != 0 {
        output_prep.push(format!("{}hr", hours));
    }

    if mins != 0 {
        output_prep.push(format!("{}min", mins));
    }
    // output 0sec for zero duration
    if duration == 0 || secs != 0 {
        output_prep.push(format!("{}sec", secs));
    }

    if millis != 0 {
        output_prep.push(format!("{}ms", millis));
    }

    if micros != 0 {
        output_prep.push(format!("{}us", micros));
    }

    if nanos != 0 {
        output_prep.push(format!("{}ns", nanos));
    }

    format!(
        "{}{}",
        if sign == -1 { "-" } else { "" },
        output_prep.join(" ")
    )
}

fn format_filesize(num_bytes: i64) -> String {
    let byte = byte_unit::Byte::from_bytes(num_bytes as u128);

    if byte.get_bytes() == 0u128 {
        return "—".to_string();
    }

    let byte = byte.get_appropriate_unit(false);

    match byte.get_unit() {
        byte_unit::ByteUnit::B => format!("{} B ", byte.get_value()),
        _ => byte.format(1),
    }
}
