// Copyright 2024 David Smith <david@narigama.dev>
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy
// of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations
// under the License.

use clap::Parser;
use toml_edit::DocumentMut;

use eyre::{Context, OptionExt};

pub mod model;

#[derive(Debug, Parser)]
pub enum Command {
    /// bump the patch version (e.g. 1.1.1 => 1.1.2)
    Patch,
    /// bump the minor version (e.g. 1.1.1 => 1.2.0)
    Minor,
    /// bump the major version (e.g. 1.1.1 => 2.0.0)
    Major,
}

#[derive(Debug, Parser)]
pub enum Args {
    Semver {
        /// don't actually do anything, just print what would happen
        #[clap(long)]
        dry_run: bool,

        /// commit the new Cargo.toml/lock and tag it
        #[clap(long)]
        git: bool,

        #[clap(subcommand)]
        command: Command,
    },
}

/// parse Cargo.toml and get it's `package.version`
fn get_cargo_version(path: &std::path::Path) -> eyre::Result<model::Version> {
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
fn set_cargo_version(path: &std::path::Path, version_new: &model::Version) -> eyre::Result<()> {
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
    command.args("status --porcelain".split_ascii_whitespace());

    let output = command.output()?;
    Ok(String::from_utf8(output.stdout)?.trim().is_empty())
}

/// commit Cargo.toml and Cargo.lock and tag the version
fn commit_with_tag(version: &model::Version) -> eyre::Result<()> {
    // add files
    let mut command = std::process::Command::new("git");
    command.args(
        "add Cargo.toml Cargo.lock"
            .to_string()
            .split_ascii_whitespace(),
    );
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

    match args {
        Args::Semver {
            dry_run,
            git,
            command,
        } => {
            if !dry_run && !is_working_dir_clean()? {
                println!(
                    "Working directory doesn't appear to be clean. Commit your changes first."
                );
                return Ok(());
            }

            let version_new = match &command {
                Command::Patch => version.patch_version(),
                Command::Minor => version.minor_version(),
                Command::Major => version.major_version(),
            };

            println!("Promoting from {version} to {version_new}");
            if git {
                println!("--git was included, will commit and tag this version")
            }

            match dry_run {
                true => println!("But this was a --dry-run. Not actually doing anything..."),
                false => {
                    set_cargo_version(&path, &version_new)?;
                    println!("Wrote changes to {:?}", &path);

                    if git {
                        commit_with_tag(&version_new)?;
                    }
                }
            }
        }
    }

    Ok(())
}
