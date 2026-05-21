use crate::config::Config;
use color_eyre::eyre::{Report, Result};
use kdeconnect_wrapper::device::Device;
use serde::{Deserialize, Deserializer};
use std::{borrow::Cow, fmt::Debug, str::FromStr};

pub mod field;
pub mod notification;
pub use field::*;
pub use notification::*;

pub type GlobalFormat = Format<FieldCategory>;

pub trait FieldFormat: Sized {
    fn parse(s: &str) -> Result<Self>;
}

#[derive(Debug)]
pub struct Format<T: FieldFormat> {
    chunks: Vec<Chunk<T>>,
}

#[derive(Debug)]
pub enum Chunk<T: FieldFormat> {
    Field(T),
    Str(String),
}

const OPENING_CHAR: char = '{';
const CLOSING_CHAR: char = '}';
const PATH_SEPARATOR: &'static str = "::";

impl<T: FieldFormat> Format<T> {
    pub fn parse(format: &str) -> Result<Self> {
        let mut current_buffer = String::new();
        let mut chars = format.chars().peekable();
        let mut chunks = Vec::new();

        while let Some(c) = chars.next() {
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
    pub fn to_string(&self, device: &Device, config: &Config) -> Result<String> {
        let cache = DeviceCategoryDataCache::default();
        self.chunks
            .iter()
            .map(|chunk| {
                chunk
                    .to_str(device, config, &cache)
                    .map(|cow| cow.to_owned())
            })
            .collect::<Result<String>>()
    }
}

impl Chunk<FieldCategory> {
    pub fn to_str<'a>(
        &'a self,
        device: &Device,
        config: &'a Config,
        cache: &DeviceCategoryDataCache,
    ) -> Result<Cow<'a, str>> {
        match self {
            Chunk::Str(s) => Ok(Cow::Borrowed(s)),
            Chunk::Field(f) => Ok(f.get_from_device(device, config, &cache)?),
        }
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
