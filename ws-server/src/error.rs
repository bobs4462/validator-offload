use std::{
    borrow::Cow,
    fmt::{self, Display},
};

use serde::Serialize;

/// Subscription related error type
#[derive(Debug, Serialize)]
pub struct SubError<'a> {
    code: i64,
    message: Cow<'a, str>,
}

/// Error code that is returned to client, on request error
pub enum SubErrorKind {
    /// Subscription request couldn't be deserialized properly
    ParseError = -32700,
    /// Subscription request contained parameter, which wasn't expected
    InvalidParams = -32602,
}

impl<'a, T: serde::de::Error> From<T> for SubError<'a> {
    fn from(e: T) -> Self {
        Self {
            code: SubErrorKind::ParseError as i64,
            message: e.to_string().into(),
        }
    }
}

impl<'a> std::error::Error for SubError<'a> {}

impl<'a> Display for SubError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl<'a> SubError<'a> {
    /// Error constructor
    pub fn new(message: Cow<'a, str>, kind: SubErrorKind) -> Self {
        Self {
            code: kind as i64,
            message,
        }
    }
}
