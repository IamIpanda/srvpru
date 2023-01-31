#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Something wrong when io")]
    IO(std::io::Error),
    #[error("Custom error")]
    Custom,
    #[error("Try to serialize a component over its design size")]
    Oversize,
    #[error("Try to deserialize a seq without limit.")]
    Unlimited,
    #[error("Some error happened when unwrap the writer.")]
    UnwrapWriter,
    #[error("Try to deserialize to a wrong type.")]
    WrongType,
    #[error("Try to deserialize an unknown type message.")]
    UnknownType,
    #[error("Try to change full message to wrong status.")]
    WrongStatus,
}

impl serde::ser::Error for Error {
    fn custom<T>(_: T) -> Self where T: std::fmt::Display {
        Error::Custom
    }
}

impl serde::de::Error for Error {
    fn custom<T>(_ :T) -> Self where T:std::fmt::Display {
        Error::Custom
    }
}

pub fn map_std_io_result<T>(result: std::io::Result<T>) -> Result<(), Error> {
    match result {
        Ok(_) => Ok(()),
        Err(err) => Err(Error::IO(err)),
    }
}

pub fn map_std_io_error(err: std::io::Error) -> Error {
    return Error::IO(err);
}
