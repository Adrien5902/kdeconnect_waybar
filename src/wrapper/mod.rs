#[macro_use]
mod parsing;
pub mod client;
pub mod device;
pub mod error;
pub mod notifications;

pub use self::client::*;
pub use self::device::*;
pub use self::error::*;
pub use self::notifications::*;

#[cfg(test)]
mod test;

pub type Result<T> = std::result::Result<T, Error>;
