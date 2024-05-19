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

use std::{fmt::Display, str::FromStr};

use eyre::{Context, OptionExt};

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
    pub fn patch_version(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor,
            patch: self.patch + 1,
        }
    }

    pub fn minor_version(&self) -> Self {
        Self {
            major: self.major,
            minor: self.minor + 1,
            patch: 0,
        }
    }

    pub fn major_version(&self) -> Self {
        Self {
            major: self.major + 1,
            minor: 0,
            patch: 0,
        }
    }
}
