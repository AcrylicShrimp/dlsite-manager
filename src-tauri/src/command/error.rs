use anyhow::Error as AnyError;
use serde::{Serialize, Serializer};

pub struct CommandError {
    error: AnyError,
}

impl<T> From<T> for CommandError
where
    T: Into<AnyError>,
{
    fn from(value: T) -> Self {
        Self {
            error: value.into(),
        }
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
