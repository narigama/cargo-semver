use std::{fmt::Display, str::FromStr};

use clap::Parser;
use toml_edit::DocumentMut;

use eyre::{Context, OptionExt};

#[derive(Debug, Parser)]
pub enum Command {
    Patch,
    Minor,
    Major,
}

#[derive(Debug, Parser)]
pub struct Args {
    /// don't actually do anything, just print what would happen
    #[clap(long)]
    pub dry_run: bool,

    /// commit the new Cargo.toml/lock and tag it
    #[clap(long)]
    pub git: bool,

    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl FromStr for Version {
    type Err = eyre::Error;

    fn from_str(value: &str) -> eyre::Result<Self> {
        let mut parts = value.split('.');

        let major = parts
            .next()
            .ok_or_eyre(format!("Unable to find `major` version in {value}"))?
            .parse()
            .context("unable to parse `major` version into u64")?;

        let minor = parts
            .next()
            .ok_or_eyre(format!("Unable to find `minor` version in {value}"))?
            .parse()
            .context("unable to parse `minor` version into u64")?;

        let patch = parts
            .next()
            .ok_or_eyre(format!("Unable to find `patch` version in {value}"))?
            .parse()
            .context("unable to parse `patch` version into u64")?;

        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Version {
    fn patch_version(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor,
            patch: self.patch + 1,
        }
    }

    fn minor_version(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor + 1,
            patch: 0,
        }
    }

    fn major_version(&self) -> Self {
        Self {
            major: self.major + 1,
            minor: 0,
            patch: 0,
        }
    }
}

/// parse Cargo.toml and get it's `package.version`
fn get_cargo_version(path: &std::path::Path) -> eyre::Result<Version> {
    let cargo = std::fs::read_to_string(path)
        .context(format!("Couldn't find {path:?}"))?
        .parse::<DocumentMut>()
        .context(format!("Unable to parse {path:?}"))?;

    let version = cargo
        .get("package")
        .ok_or_eyre(format!("Couldn't find `[package]` in {path:?}"))?
        .get("version")
        .ok_or_eyre(format!("Couldn't find `version` in {path:?}"))?
        .as_str()
        .ok_or_eyre("Unable to convert `version` into a string.")?
        .to_string();

    version.parse()
}

/// write the new `version` back to Cargo.toml
fn set_cargo_version(path: &std::path::Path, version_new: &Version) -> eyre::Result<()> {
    // parse the Cargo.toml, and modify the `package.version`
    let mut cargo = std::fs::read_to_string(path)
        .context(format!("Couldn't find {path:?}"))?
        .parse::<DocumentMut>()
        .context(format!("Unable to parse {path:?}"))?;

    let version = cargo
        .get_mut("package")
        .ok_or_eyre(format!("Couldn't find `[package]` in {path:?}"))?
        .get_mut("version")
        .ok_or_eyre(format!("Couldn't find `version` in {path:?}"))?;
    *version = toml_edit::value(version_new.to_string());

    // write the changes
    std::fs::write(path, cargo.to_string())?;

    // run `cargo check` to make sure the .lock is also up to date
    let mut command = std::process::Command::new("cargo");
    command.arg("check");
    command.output()?;

    Ok(())
}

/// check there aren't pending changes, make sure the working dir is clean before making changes
fn is_working_dir_clean() -> eyre::Result<bool> {
    let mut command = std::process::Command::new("git");
    command.args(["status", "--porcelain"]);

    let output = command.output()?;
    Ok(String::from_utf8(output.stdout)?.trim().is_empty())
}

/// commit Cargo.toml and Cargo.lock and tag the version 
fn commit_with_tag(version: &Version) -> eyre::Result<()> {
    // add files
    let mut command = std::process::Command::new("git");
    command.args("add Cargo.toml Cargo.lock".to_string().split_ascii_whitespace());
    command.output()?;

    // commit
    let mut command = std::process::Command::new("git");
    command.args(format!("commit -m v{version}").split_ascii_whitespace());
    command.output()?;

    // add tag
    let mut command = std::process::Command::new("git");
    command.args(format!("tag v{version}").split_ascii_whitespace());
    command.output()?;

    Ok(())
}

fn main() -> eyre::Result<()> {
    let path = std::path::PathBuf::new().join("Cargo.toml");

    let args = Args::parse();
    let version = get_cargo_version(&path)?;

    if !args.dry_run && !is_working_dir_clean()? {
        println!("Working directory doesn't appear to be clean. Commit your changes first.");
        return Ok(());
    }

    let version_new = match &args.command {
        Command::Patch => version.patch_version(),
        Command::Minor => version.minor_version(),
        Command::Major => version.major_version(),
    };

    println!("Promoting from {version} to {version_new}");
    if args.git {
        println!("--git was included, will commit and tag this version")
    }

    match args.dry_run {
        true => println!("But this was a --dry-run. Not actually doing anything..."),
        false => {
            set_cargo_version(&path, &version_new)?;
            println!("Wrote changes to {:?}", &path);

            if args.git {
                commit_with_tag(&version_new)?;
            }
        }
    }

    Ok(())
}
