use std::str::FromStr;

use miette::bail;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum Scheme {
    Ssh,
}

impl FromStr for Scheme {
    type Err = miette::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "ssh" => Self::Ssh,
            _ => bail!("Scheme {} is not supported.", s),
        })
    }
}
