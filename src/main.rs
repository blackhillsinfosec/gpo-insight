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

use std::
    path::{Path, PathBuf}
;
use clap::{arg, Parser};
use anyhow::{anyhow, Result};
mod cli;
mod gpo;
mod analysis;

#[derive(Parser)]
#[command(name="GPO Insight", version)]
struct Args {
    #[arg(short='i', long, required = true)]
    input: String,
    #[arg(short='o', long, default_value = "PWD")]
    output: String,
}

fn main() -> Result<()>{
    println!("GPO Insight v{}, Copyright (C) 2024 Kiersten Gross\n\nThis project is licensed under the GNU General Public License v3.0. <https://www.gnu.org/licenses/>.\nThis program comes with ABSOLUTELY NO WARRANTY.\n", env!("CARGO_PKG_VERSION").to_owned());
    let args = Args::parse();
    let input_path:PathBuf = cli::parse_input_path(&args.input)?;
    let output_path = match cli::parse_output_path(&args.output) {
        Ok(v) => {
            gpo::breakdown_gpo(&input_path, &v)?;
            gpo::gpo_to_text(&v)?;
            v
        } Err(e) => {
            if e.to_string().starts_with("The output directory ") && e.to_string().ends_with(" already exists.") {
                println!("Breakdown skipped. {}", e);
                Path::new(&e.to_string().replace("The output directory ", "").replace(" already exists.", "")).to_path_buf()
            } else {
                return Err(anyhow!(e));
            }
        }
    };
    let policies = gpo::text_to_struct(&output_path)?;
    let analysis_path = output_path.join("analysis");
    if !analysis_path.exists() {
        std::fs::create_dir_all(&analysis_path)?;
    }
    analysis::analyze(&policies, &analysis_path)?;

    Ok(())
}