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

use crate::gpo::GroupPolicy;
use std::fs::{self, ReadDir};
use std::
    path::{Path, PathBuf};
use anyhow::{Result, anyhow};
use std::io::BufReader;
use std::io::prelude::*;
use std::fs::File;

pub fn analyze(gpos: &Vec<GroupPolicy>, output_path: &PathBuf) -> Result<()>{
    let d_analysis_file_buf = output_path.join("desireables.txt");
    let d_analysis_file = Path::new(&d_analysis_file_buf);
    if d_analysis_file.exists() {
        std::fs::remove_file(d_analysis_file)?;
    } 
    let mut d_file = File::create(d_analysis_file)?;

    let u_analysis_file_buf = output_path.join("undesireables.txt");
    let u_analysis_file = Path::new(&u_analysis_file_buf);
    if u_analysis_file.exists() {
        std::fs::remove_file(u_analysis_file)?;
    } 
    let mut u_file = File::create(u_analysis_file)?;

    let w_analysis_file_buf = output_path.join("warnings.txt");
    let w_analysis_file = Path::new(&w_analysis_file_buf);
    if w_analysis_file.exists() {
        std::fs::remove_file(w_analysis_file)?;
    } 
    let mut w_file = File::create(w_analysis_file)?;

    let m_analysis_file_buf = output_path.join("missing.txt");
    let m_analysis_file = Path::new(&m_analysis_file_buf);
    if m_analysis_file.exists() {
        std::fs::remove_file(m_analysis_file)?;
    } 
    let mut m_file = File::create(m_analysis_file)?;



    let current_dir_buf = std::env::current_dir()?;
    let current_query_files_buf = current_dir_buf.join("queries");
    let current_query_files_path = Path::new(&current_query_files_buf);
    let exe_buf = std::env::current_exe()?;
    let exe_parent_buf = exe_buf.parent().ok_or(anyhow!("No parent directory could be determined for the current executable path."))?;
    let exe_files_buf = exe_parent_buf.join("queries");
    let exe_query_files_path = Path::new(&exe_files_buf);
    let query_files : ReadDir;

    #[cfg(debug_assertions)]
    println!("Queries search logic -- Debug:");
    #[cfg(debug_assertions)]
    println!("Current Directory: {} Exists: {}", current_query_files_path.display(), current_query_files_path.exists());
    #[cfg(debug_assertions)]
    println!("Current Directory: {} Exists: {}", exe_query_files_path.display(), exe_query_files_path.exists());

    if !current_query_files_path.exists() && !exe_query_files_path.exists() { return Err(anyhow!("The queries directory could not be found.")); }
    if exe_query_files_path.exists() {
        query_files = fs::read_dir(&exe_query_files_path)?;
    } else {
        query_files = fs::read_dir(&current_query_files_path)?;
    }

    let mut undesirables = String::new();
    let mut warnings = String::new();
    let mut desirables = String::new();
    let mut missings = String::new();

    for query_file in query_files {
        let query_path = match query_file {
            Ok(v) => {
                v.path()
            }
            Err(e) => {
                return Err(anyhow!(e));
            }
        };
        let qf = File::open(&query_path)?;
        let qf = BufReader::new(qf);
        for line in qf.lines() {
            match line {
                Err(e) => {
                    println!("The following error was thrown while trying to read the file {}.\n{}", &query_path.display(), e)
                }
                Ok(v) => {
                    let line_raw: Vec<&str> = v.split("--").collect();
                    if v.is_empty() {
                        ()
                    } else if line_raw.len() == 2  && line_raw[0].to_lowercase().trim().starts_with("u") {
                        let line_queries_raw = line_raw[1].split("|");

                        for policy in gpos.iter() {
                            let mut query_failed = false;
                            for line_query in line_queries_raw.clone() {
                                if !policy.query_gpo(&line_query.trim_start().trim_end()){
                                    query_failed = true;
                                    break
                                }
                            }
                            if !query_failed {
                                let mut matching = String::new();
                                let full_query_strings: Vec<&str> = v.split("--").collect();
                                let query_strings = full_query_strings[1].split("|");
                                let mut output_query_string = "Query Condition(s):\n\t".to_string();
                                for query in query_strings {
                                    output_query_string += query.trim_start().trim_end();
                                    output_query_string += "\n\t";
                                }
                                for line_query in line_queries_raw.clone() {
                                    let matching_string = &policy.get_matching_conditions(&line_query.trim_start().trim_end());
                                    if !matching_string.is_empty() && !matching.contains(matching_string) {
                                        matching.push_str(matching_string);
                                        matching.push_str("\n\t")
                                    }
                                }
                                undesirables += &format!("{}\n{}\n\t{}\n\tMatching Value(s):\n\t{}\n\n", query_path.display(), policy, output_query_string.trim_end(), matching.trim_end());
                            }
                        }
                    } else if line_raw.len() == 2  &&  line_raw[0].to_lowercase().trim().starts_with("d") {
                        let line_queries_raw = line_raw[1].split("|");

                        for policy in gpos.iter() {
                            let mut query_failed = false;
                            for line_query in line_queries_raw.clone() {
                                if !policy.query_gpo(&line_query.trim_start().trim_end()){
                                    query_failed = true;
                                    break
                                }
                            }
                            if !query_failed {
                                let mut matching = String::new();
                                let full_query_strings: Vec<&str> = v.split("--").collect();
                                let query_strings = full_query_strings[1].split("|");
                                let mut output_query_string = "Query Condition(s):\n\t".to_string();
                                for query in query_strings {
                                    output_query_string += query.trim_start().trim_end();
                                    output_query_string += "\n\t";
                                }
                                for line_query in line_queries_raw.clone() {
                                    let matching_string = &policy.get_matching_conditions(&line_query.trim_start().trim_end());
                                    if !matching_string.is_empty() && !matching.contains(matching_string) {
                                        matching.push_str(matching_string);
                                        matching.push_str("\n\t")
                                    }
                                }
                                desirables += &format!("{}\n{}\n\t{}\n\tMatching Value(s):\n\t{}\n\n", query_path.display(), policy, output_query_string.trim_end(), matching.trim_end());
                            }
                        }
                    } else if line_raw.len() == 2  && line_raw[0].to_lowercase().trim().starts_with("w") {
                        let line_queries_raw = line_raw[1].split("|");

                        for policy in gpos.iter() {
                            let mut query_failed = false;
                            for line_query in line_queries_raw.clone() {
                                if !policy.query_gpo(&line_query.trim_start().trim_end()){
                                    query_failed = true;
                                    break
                                }
                            }
                            if !query_failed {
                                let mut matching = String::new();
                                let full_query_strings: Vec<&str> = v.split("--").collect();
                                let query_strings = full_query_strings[1].split("|");
                                let mut output_query_string = "Query Condition(s):\n\t".to_string();
                                for query in query_strings {
                                    output_query_string += query.trim_start().trim_end();
                                    output_query_string += "\n\t";
                                }
                                for line_query in line_queries_raw.clone() {
                                    let matching_string = &policy.get_matching_conditions(&line_query.trim_start().trim_end());
                                    if !matching_string.is_empty() && !matching.contains(matching_string) {
                                        matching.push_str(matching_string);
                                        matching.push_str("\n\t")
                                    }
                                }
                                warnings += &format!("{}\n{}\n\t{}\n\tMatching Value(s):\n\t{}\n\n", query_path.display(), policy, output_query_string.trim_end(), matching.trim_end());
                            }
                        }
                    } else if line_raw.len() == 2  && line_raw[0].to_lowercase().trim().starts_with("m") {
                        let line_queries_raw = line_raw[1].split("|");
                        let mut policy_exists = false;
                        for policy in gpos.iter() {
                            let mut query_failed = false;
                            for line_query in line_queries_raw.clone() {
                                if !policy.query_gpo(&line_query.trim_start().trim_end()){
                                    query_failed = true;
                                    break
                                }
                            }
                            if !query_failed {
                                policy_exists = true;
                            }
                        }
                        if !policy_exists {
                            let full_query_strings: Vec<&str> = v.split("--").collect();
                            let query_strings = full_query_strings[1].split("|");
                            let mut output_query_string = "Query Condition(s):\n\t".to_string();
                            for query in query_strings {
                                output_query_string += query.trim_start().trim_end();
                                output_query_string += "\n\t";
                            }
                            missings += &format!("{}\n\t{}\n\n", query_path.display(), output_query_string.trim_end());
                        }
                    }
                }
            }
        }
    }

    if !undesirables.is_empty() {
        u_file.write(format!("Undesirables\n{}", undesirables).as_bytes())?;
        println!("Undesirables\n{}", undesirables);
    }
    if !desirables.is_empty() {
        d_file.write(format!("Desirables\n{}", desirables).as_bytes())?;
        println!("Desirables\n{}", desirables);
    }
    if !warnings.is_empty() {
        w_file.write(format!("Warning\n{}", warnings).as_bytes())?;
        println!("Warning\n{}", warnings);
    }
    if !missings.is_empty() {
        m_file.write(format!("Missings\n{}", missings).as_bytes())?;
        println!("Missings\n{}", missings);
    }

    Ok(())
}