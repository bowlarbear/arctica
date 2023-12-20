use serde::{Serialize, Serializer, Deserialize};

#[derive(thiserror::Error, Debug)]
pub enum Error {
   #[error("Command: {}; Failed with Error: {};", .0, .1)]
    CommandFailed(String, String),

    #[error("UUID Not Found")]
    UUIDNotFound(),
    #[error("Home Directory Not Found")]
    HomeNotFound(),

    #[error("Network Is Not Active/Connected")]
    NetworkNotActive(),
    #[error("Network Is Active/Connected")]
    NetworkActive(),

    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    StrUtf8Error(#[from] std::str::Utf8Error),
    #[error(transparent)]
    StringUtf8Error(#[from] std::string::FromUtf8Error),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_str(&self.to_string())
    }
}
