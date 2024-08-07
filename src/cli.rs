// This file is a part of Audit-Inspector
// Copyright (C) 2024 Kiersten Gross

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{
    path::{Path, PathBuf},
    env::current_dir,
};
use anyhow::{Result, Context, anyhow};
use dirs::home_dir;

pub fn parse_input_path(path: &str) -> Result<PathBuf> {
    // Allow ~ to be an alias for HOME.
    if path.starts_with('~') {
        match home_dir() {
            Some(v) => {
                let home = path.replace("~", v.to_str().with_context(|| "Could not convert the home directory into a string slice.")?);
                let home_path = Path::new(&home);
                if home_path.exists() {
                    return Ok(home_path.to_path_buf());
                } else {
                    return Err(anyhow!("The path {} could not be found.", home_path.display()))
                }
            }
            None => {
                return Err(anyhow!("The HOME environment variable could not be found."));
            }
        }
    }
    // Check if the file exists
    else {
        let provided_path = Path::new(path);
        if provided_path.exists(){
            return Ok(Path::new(path).to_path_buf())
        }
        else {
            return Err(anyhow!("The path {} could not be found.", provided_path.display()))
        }
    }
}

pub fn parse_output_path(path: &str) -> Result<PathBuf> {
    // Allow ~ to be an alias for HOME.
    if path.starts_with('~') {
        match home_dir() {
            Some(v) => {
                let home = path.replace("~", v.to_str().with_context(|| "Could not convert the home directory into a string slice.")?);
                let home_path = Path::new(&home);
                let home_path = home_path.join("gpo-insight");
                if home_path.exists() {
                    return Err(anyhow!("The output directory {} already exists.", home_path.display()))
                } else {
                    return Ok(home_path.to_path_buf());
                }
            }
            None => {
                return Err(anyhow!("The HOME environment variable could not be found."));
            }
        }
    }
    // Allow pwd or cwd to be and alias for the current working directory
    else if (path.to_lowercase() == "pwd") || (path.to_lowercase() == "cwd") {
        let current_dir = current_dir()?;
        let current_dir_string = current_dir.to_string_lossy().into_owned();
        let new_path = Path::new(&current_dir_string);
        let new_path = new_path.join("gpo-insight");
        if new_path.exists() {
            return Err(anyhow!("The output directory {} already exists.", new_path.display()))
        } else {
            return Ok(new_path.to_path_buf())
        }
    }
    // Check if the directory exists
    else {
        let curr_path = Path::new(path);
        if curr_path.exists() {
            return Err(anyhow!("The output directory {} already exists.", curr_path.display()))
        } else {
            Ok(curr_path.to_path_buf())
        }
    }
}