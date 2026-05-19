use crate::error::Error;

#[macro_use]
mod parsing;
pub mod client;
pub mod device;
pub mod error;
pub mod notifications;

#[cfg(test)]
mod test;

pub type Result<T> = std::result::Result<T, Error>;
