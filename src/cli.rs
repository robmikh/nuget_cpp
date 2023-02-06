use std::{fmt::Display, str::FromStr};

use clap::Parser;

/// A tool that assists in packaging C++/WinRT components for NuGet.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Sets the current directory when running the tool
    #[arg(short, long)]
    pub dir: Option<String>,

    /// Restores project, builds all platforms, and packs [overrides everything]
    #[arg(short, long)]
    pub all: bool,

    /// Calls nuget restore
    #[arg(short, long)]
    pub restore: bool,

    /// Builds the solution for Release
    #[arg(short, long)]
    pub build: Vec<Platform>,

    /// Packs the resulting files. Uses the nuget\ directory
    #[arg(short, long)]
    pub pack: bool,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Platform {
    x64,
    ARM,
    x86,
    ARM64,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ParsePlatformError(&'static str);

impl FromStr for Platform {
    type Err = ParsePlatformError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "x64" => Ok(Platform::x64),
            "arm" => Ok(Platform::ARM),
            "x86" => Ok(Platform::x86),
            "arm64" => Ok(Platform::ARM64),
            _ => Err(ParsePlatformError(
                "Invalid resolution value! Expecting: x86, x64, ARM or ARM64.",
            )),
        }
    }
}

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Platform::x64 => "x64",
            Platform::ARM => "ARM",
            Platform::x86 => "x86",
            Platform::ARM64 => "ARM64",
        };
        write!(f, "{}", string)
    }
}

impl Display for ParsePlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for ParsePlatformError {}
