use crate::config::Config;
use color_eyre::eyre::{Report, Result};
use serde::{Deserialize, Deserializer};
use std::{
    fmt::{Debug, Write},
    str::FromStr,
};

pub mod field;
pub mod notification;
pub use field::*;
pub use notification::*;

/// A kind of [`Format`]
/// used in [`Config::format`], [`Config::tooltip_format`], and more, see [`Config`]
///
/// A string of text with special fields being replaced e.g. `"Battery: {Battery::ChargePercent}%"`
///
/// [`GlobalFormat`] fields are separated categories delimited by two colons (`::`),
/// see [`FieldCategory`] for all the different categories available
pub type GlobalFormat = Format<FieldCategory>;

pub trait FieldFormat: Sized {
    fn parse(s: &str) -> Result<Self>;
}

#[derive(Debug)]
/// ℹ️ Can contain Nerd-Font icons
///
/// Formats are strings of text that can contain special fields surrounded by braces (`{` and `}`),
/// fields may are organized in categories and are then access with this syntax `{Category::Field}`
///
/// See [`GlobalFormat`] and [`NotificationFormat`] for more info
pub struct Format<T: FieldFormat> {
    chunks: Vec<Chunk<T>>,
}

impl<T: FieldFormat> Default for Format<T> {
    fn default() -> Self {
        Self { chunks: Vec::new() }
    }
}

#[derive(Debug)]
pub enum Chunk<T: FieldFormat> {
    Field(T),
    Str(String),
}

const OPENING_CHAR: char = '{';
const CLOSING_CHAR: char = '}';
const PATH_SEPARATOR: &str = "::";

impl<T: FieldFormat> Format<T> {
    pub fn parse(format: &str) -> Result<Self> {
        let mut current_buffer = String::new();
        let mut chunks = Vec::new();

        for c in format.chars() {
            match c {
                OPENING_CHAR => {
                    if !current_buffer.is_empty() {
                        chunks.push(Chunk::Str(current_buffer));
                        current_buffer = String::new();
                    }
                }
                CLOSING_CHAR => {
                    let field = T::parse(&current_buffer)?;
                    chunks.push(Chunk::Field(field));
                    current_buffer = String::new();
                }
                other => current_buffer.push(other),
            }
        }

        if !current_buffer.is_empty() {
            chunks.push(Chunk::Str(current_buffer));
        }

        Ok(Format { chunks })
    }
}

impl Format<FieldCategory> {
    pub fn format(
        &self,
        f: &mut String,
        config: &Config,
        cache: &DeviceCategoryDataCache,
    ) -> Result<()> {
        for chunk in &self.chunks {
            chunk.format(f, config, cache)?;
        }
        Ok(())
    }

    pub fn to_string(&self, config: &Config, cache: &DeviceCategoryDataCache) -> Result<String> {
        let mut s = String::new();
        self.format(&mut s, config, cache)?;
        Ok(s)
    }
}

impl Chunk<FieldCategory> {
    pub fn format<'a>(
        &'a self,
        f: &mut String,
        config: &'a Config,
        cache: &'a DeviceCategoryDataCache,
    ) -> Result<()> {
        match self {
            Chunk::Str(s) => f.write_str(s)?,
            Chunk::Field(field) => field.format_from_device(f, config, cache)?,
        }
        Ok(())
    }
}

impl<'de, T: FieldFormat> Deserialize<'de> for Format<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        Format::parse(&s).map_err(serde::de::Error::custom)
    }
}

impl<T: FieldFormat> FromStr for Format<T> {
    type Err = Report;
    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        Format::parse(s)
    }
}
