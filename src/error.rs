use std::fmt::{self, Display};

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SubError {
    code: i64,
    message: String,
}

pub enum SubErrorKind {
    ParseError = -32700,
    InvalidParams = -32602,
}

impl<T: serde::de::Error> From<T> for SubError {
    fn from(e: T) -> Self {
        Self {
            code: SubErrorKind::ParseError as i64,
            message: e.to_string(),
        }
    }
}

impl std::error::Error for SubError {}

impl Display for SubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl SubError {
    pub fn new(message: String, kind: SubErrorKind) -> Self {
        Self {
            code: kind as i64,
            message,
        }
    }
}
