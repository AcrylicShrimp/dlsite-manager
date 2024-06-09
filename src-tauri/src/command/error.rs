use anyhow::Error;
use serde::{Serialize, Serializer};

pub struct CommandError {
    error: Error,
}

impl From<Error> for CommandError {
    fn from(value: Error) -> Self {
        Self { error: value }
    }
}

impl Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{:?}", self.error))
    }
}

pub type CommandResult<T> = Result<T, CommandError>;
