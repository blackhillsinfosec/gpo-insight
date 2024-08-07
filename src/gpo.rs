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

use core::fmt;
use std::{
    fs::{self, File}, io::{BufRead, BufReader, Read, Write}, path::{
        Path, 
        PathBuf
    }
};
use anyhow::{Result, anyhow};
use console::Style;
use encoding_rs_io::DecodeReaderBytesBuilder;
use html2text;
use lazy_static::lazy_static;

lazy_static!{
    static ref ERR_STYLE: Style = Style::new().red().bold();
}

pub fn breakdown_gpo(input_path: &PathBuf, output_path: &PathBuf) -> Result<()> {
    let mut output = Path::new(&output_path);
    if output.exists(){
        return Err(anyhow!("The output directory {} already exists!", output.display()))
    } else {
        std::fs::create_dir_all(output)?;
    }
    
    let html_output = output.join("html");
    output = Path::new(&html_output);
    std::fs::create_dir_all(output)?;

    let in_file = File::open(Path::new(&input_path))?;

    // Deal with UTF-16LE
    let encoding = encoding_rs::UTF_16LE;
    let mut decoder = DecodeReaderBytesBuilder::new()
        .encoding(Some(encoding))
        .build(in_file);
    let mut content = String::new();
    decoder.read_to_string(&mut content)?;

    // Parse the file into separate files
    let buf = BufReader::new(content.as_bytes());

    let mut title_buf:String = String::new();
    let mut content_buf: String = String::new();
    let mut count_buf = 0;
    for (i, line) in buf.lines().enumerate() {
        match line {
            Ok(cont) => {
                // Reset buffers if a new HTML file is detected.
                if cont.starts_with("<html "){
                    title_buf = String::new();
                    content_buf = String::new();
                    count_buf += 1;
                }
                // Write the line to the buffer
                content_buf += &cont;
                content_buf += "\n";

                // If the end of an HTML file is detected.
                if cont.ends_with("</html>"){
                    if title_buf.is_empty() || content_buf.is_empty() {
                        return Err(anyhow!("Parsing the GPO HTML file failed."));
                    } else {
                        let mut title = title_buf.clone();
                        title.push_str(".html");
                        let out_path = output.join(&title);
                        let out_file = Path::new(&out_path);
                        if out_file.exists() {
                            let mut out_attempt = 0;
                            loop {
                                out_attempt += 1;
                                let mut title = title_buf.clone();
                                title.push_str("(");
                                title.push_str(&out_attempt.to_string());
                                title.push_str(")");
                                title.push_str(".html");
                                let out_file_attempt = output.join(&title);
                                let out_file = Path::new(&out_file_attempt);
                                if !out_file.exists(){

                                    #[cfg(debug_assertions)]
                                    println!("{}", out_file.display());

                                    let mut file = File::create(out_file)?;
                                    file.write(content_buf.as_bytes())?;
                                    break
                                }
                            }
                        } else{

                            #[cfg(debug_assertions)]
                            println!("{}", out_file.display());

                            let mut file = File::create(out_file)?;
                            file.write(content_buf.as_bytes())?;
                        }
                    }
                }
                // Set the title buffer if a title is detected.
                else if cont.starts_with("<title>") && cont.contains("</title>") {
                    let trimmed = cont.trim();
                    title_buf = trimmed.replace("<title>", "").replace("</title>", "").replace("/", "_").replace("\\", "_");
                }
            }
            Err(e) => {
                println!("Error reading line {}: {}", i, e);
            }
        }
    }

    println!("GPO Breakdown successful. {} GPOs detected.", count_buf);

    Ok(())
}

pub fn gpo_to_text(output_path: &PathBuf) -> Result<()> {
    let html_path_buf = output_path.join("html");
    let html_path = Path::new(&html_path_buf);
    let files = fs::read_dir(&html_path)?;
    let mut files_count = 0;

    let txt_path_buf = output_path.join("txt");
    let txt_path = Path::new(&txt_path_buf);

    if !txt_path.exists() {
        fs::create_dir_all(txt_path)?;
    }

    for file in files {
        let path = match file {
            Ok(v) => {
                v.path()
            },
            Err(e) => {
                return Err(anyhow!(e));
            }
        };

        let f = File::open(&path)?;
        let text_content = html2text::from_read(f, 300);
        
        let text_file_name = path.file_stem();
        let text_file_stem = match text_file_name {
            Some(v) => {
                v.to_string_lossy()
            }
            None => {
                return Err(anyhow!("Could not find the file stem for {}", path.display()))
            }
        };
        
        let text_file_name = text_file_stem.to_string() + ".txt";
        let text_file_path = txt_path.join(Path::new(&text_file_name));
        let mut output_file = File::create(text_file_path)?;

        let mut valid = false;
        for line in text_content.lines() {
            // There's a lot of CSS/JS at the beginning of the html2text output that isn't parsed.
            // Don't write output until we find the beginning of the real GPO.
            let output_line = line.to_string() + "\n";
            if !valid && line.contains("────"){
                valid = true;
                output_file.write(output_line.as_bytes())?;
            }
            if valid {
                output_file.write(output_line.as_bytes())?;
            }
        }
        files_count += 1;
    }

    println!("{} GPOs successfully converted from HTML to TXT.", files_count);

    Ok(())
}

pub fn text_to_struct(output_path: &PathBuf) -> Result<Vec<GroupPolicy>> {
    let mut policies: Vec<GroupPolicy> = Vec::new();
    let text_dir_path = output_path.join("txt");
    let files = fs::read_dir(text_dir_path)?;

    for file in files {
        let path = match file {
            Ok(v) => {
                v.path()
            },
            Err(e) => {
                return Err(anyhow!(e));
            }
        };
        let extention = path.clone().extension().map(|f| f == "txt").unwrap_or(false);
        if extention {
            let new_gpo = GroupPolicy::new(&path);
            match new_gpo {
                Ok(g) => {
                    policies.push(g);
                }
                Err(e) => {
                    println!("Could not parse GPO {}.\n{}", path.display(), e);
                }
            }
        }
    }

    Ok(policies)
}

#[derive(Debug, Clone)]
pub struct Details {
    pub id: String,
    pub status: String,
    pub domain: String,
    pub owner: String,
    pub created: String,
    pub modified: String,
}

impl Details {
    fn new() -> Self {
        Self {
            id: String::new(),
            status: String::new(),
            domain: String::new(),
            owner: String::new(),
            created: String::new(),
            modified: String::new(),
        }
    }
    fn set_id(&mut self, value: String) {
        self.id = value;
    }
    fn set_status(&mut self, value: String) {
        self.status = value;
    }
    fn set_domain(&mut self, value: String) {
        self.domain = value;
    }
    fn set_owner(&mut self, value: String) {
        self.owner = value;
    }
    fn set_created(&mut self, value: String) {
        self.created = value;
    }
    fn set_modified(&mut self, value: String) {
        self.modified = value;
    }
    fn is_empty(&mut self) -> bool {
        return self.id.is_empty() && self.status.is_empty() && self.domain.is_empty() && self.owner.is_empty() && self.created.is_empty() && self.modified.is_empty()
    }
    // fn is_id(self, id: &str) -> bool {
    //     self.id.to_lowercase() == id.to_lowercase()
    // }
    // fn is_enabled(self) -> bool {
    //     self.status.to_lowercase() == "enabled"
    // }
    // fn is_domain(self, domain: &str) -> bool {
    //     self.domain.to_lowercase() == domain.to_lowercase()
    // }
    fn is_owner(self, owner: &str) -> bool {
        if owner.starts_with(">"){
            self.owner.to_lowercase().ends_with(&owner[1..].to_lowercase())
        } else if owner.starts_with("<"){
            self.owner.to_lowercase().starts_with(&owner[1..].to_lowercase())
        } else {
            self.owner.to_lowercase() == owner.to_lowercase()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Link {
    pub location: String,
    pub enforced: String,
    pub status: String,
    pub path: String,
}

impl Link {
    fn new() -> Self {
        Self {
            location : String::new(),
            enforced : String::new(),
            status: String::new(),
            path : String::new()
        }
    }
    fn set_location(&mut self, value: &str) {
        self.location = value.to_string();
    }
    fn set_enforced(&mut self, value: &str) {
        self.enforced = value.to_string();
    }
    fn set_status(&mut self, value: &str) {
        self.status = value.to_string();
    }
    fn set_path(&mut self, value: &str) {
        self.path = value.to_string();
    }
    fn is_location(self, value: &str) -> bool {
        if value.starts_with(">") {
            self.location.to_lowercase().ends_with(&value[1..].to_lowercase())
        } else if value.starts_with("<") {
            self.location.to_lowercase().starts_with(&value[1..].to_lowercase())
        } else {
            self.location.to_lowercase() == value.to_lowercase()
        }
    }
    // fn is_enforced(self, value: &str) -> bool {
    //     self.enforced.to_lowercase() == value.to_lowercase()
    // }
    // fn is_status(self, value: &str) -> bool {
    //     self.status.to_lowercase() == value.to_lowercase()
    // }
    // fn is_path(self, value: &str) -> bool {
    //     self.path.to_lowercase() == value.to_lowercase()
    // }
}

#[derive(Debug, Clone)]
pub struct Delegation {
    pub name: String,
    pub permissions: Vec<String>,
    pub inherited: String,
}

impl Delegation {
    pub fn new() -> Self {
        Self {
            name: "".to_string(),
            permissions: Vec::new(),
            inherited: "".to_string(),
        }
    }
    pub fn set_name(&mut self, new_name: &str){
        self.name = new_name.to_string();
    }
    pub fn add_permission(&mut self, permission: &str){
        self.permissions.push(permission.to_string());
    }
    pub fn set_inheritence(&mut self, inheritence: &str){
        self.inherited = inheritence.to_string();
    }
    fn is_name(self, value: &str) -> bool {
        if value.starts_with(">") {
            self.name.to_lowercase().ends_with(&value[1..].to_lowercase())
        }
        else if value.starts_with("<") {
            self.name.to_lowercase().starts_with(&value[1..].to_lowercase())
        }
        else {
            self.name.to_lowercase() == value.to_lowercase()
        }
    }
    fn contains_permission(self, value: &str) -> bool {
        if value.starts_with(">"){
            let iterator = self.permissions.iter();
            for permission in iterator {
                if permission.to_lowercase().ends_with(&value[1..].to_lowercase()){
                    return true;
                }
            }
            false
        } 
        else if value.starts_with("<"){
            let iterator = self.permissions.iter();
            for permission in iterator {
                if permission.to_lowercase().starts_with(&value[1..].to_lowercase()){
                    return true;
                }
            }
            false
        }
        else {
            let iterator = self.permissions.iter();
            for permission in iterator {
                if permission.to_lowercase().contains(&value.to_lowercase()){
                    return true;
                }
            }
            false
        }
    }
    fn is_inherited(self) -> bool {
        self.inherited.to_lowercase() == "yes"
    }
}

impl fmt::Display for Delegation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut output_string = String::new();
        output_string += &format!("\tName: {} | Permissions: ", &self.name.as_str());
        for permission in self.permissions.clone() {
            output_string.push_str(&permission);
            output_string.push_str(" ");
        }
        output_string += &format!("| Inherited {}\n", &self.inherited);

        write!(f, "{}", output_string)
    }
}

#[derive(Debug, Clone)]
pub struct Policy {
    pub value: String,
    pub setting: Vec<String>,
}

impl Policy {
    fn new() -> Self {
        Self {
            value: "".to_string(),
            setting: Vec::new(),
        }
    }
    fn set_value(&mut self, value: &str) {
        self.value = value.to_string();
    }
    fn add_setting(&mut self, value: &str) {
        self.setting.push(value.to_string());
    }
    fn is_value(self, value: &str) -> bool {
        if value.starts_with(">") {
            self.value.to_lowercase().ends_with(&value[1..].to_lowercase())
        }
        else if value.starts_with("<") {
            self.value.to_lowercase().starts_with(&value[1..].to_lowercase())
        }
        else {
            self.value.to_lowercase() == value.to_lowercase()
        }
    }
    fn contains_setting(self, value: &str) -> bool {
        let iterator = self.setting.iter();
        for setting in iterator {
            if value.starts_with(">"){
                if setting.to_lowercase().ends_with(&value[1..].to_lowercase()) {
                    return true;
                }
            }
            else if value.starts_with("<"){
                if setting.to_lowercase().starts_with(&value[1..].to_lowercase()) {
                    return true;
                }
            }
            else if value.starts_with("#>=") && value[3..].parse::<i32>().is_ok(){
                if setting.parse::<i32>().is_ok(){
                    if value[3..].parse::<i32>().unwrap() < setting.parse::<i32>().unwrap() {
                        return true;
                    }
                }
            }
            else if value.starts_with("#>") && value[2..].parse::<i32>().is_ok(){
                if setting.parse::<i32>().is_ok(){
                    if value[2..].parse::<i32>().unwrap() < setting.parse::<i32>().unwrap() {
                        return true;
                    }
                }
            }
            else if value.starts_with("#<=") {
                if setting.parse::<i32>().is_ok(){
                    if value[3..].parse::<i32>().unwrap() >= setting.parse::<i32>().unwrap() {
                        return true;
                    }
                }
            }
            else if value.starts_with("#<") {
                if setting.parse::<i32>().is_ok(){
                    if value[2..].parse::<i32>().unwrap() > setting.parse::<i32>().unwrap() {
                        return true;
                    }
                }
            }
            else {
                if setting.to_lowercase() == value.to_lowercase() {
                    return true;
                }
            }
        }
        false
    }
}

impl fmt::Display for Policy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut output_string = String::new();
        output_string += &format!("Policy: {} | Settings: ", &self.value.as_str());
        for setting in self.setting.clone() {
            output_string.push_str(&setting);
            output_string.push_str(" ");
        }

        write!(f, "{}", output_string.trim_end())
    }
}

#[derive(Debug)]
pub struct GroupPolicy {
    pub name: String,
    pub details: Details,
    pub links: Vec<Link>,
    pub filtering: Vec<String>,
    pub delegation: Vec<Delegation>,
    pub policies: Vec<Policy>,
}

impl fmt::Display for GroupPolicy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut output_string = String::new();
        output_string += &format!("\tName: {} | GPO Status: {}\n", &self.name.as_str(), &self.details.status);
        let mut links_string = String::new();
        for link in self.links.iter() {
            links_string.push_str(link.to_string().as_str());
            links_string.push_str(" ");
        }
        output_string += &format!("\tLinks: [ {} ]", &links_string.trim_end());

        write!(f, "{}", output_string)
    }
}

impl fmt::Display for Link {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut output_string = String::new();
        output_string += &format!("L:{}:", &self.location.as_str());
        output_string += &format!("S:{} ", &self.status.as_str());

        write!(f, "{}", output_string.trim_end())
    }
}

impl GroupPolicy {
    // New
    pub fn new(group_policy_txt: &PathBuf) -> Result<Self> {
        let mut f = File::open(group_policy_txt)?;
        let mut content = String::new();
        File::read_to_string(&mut f, &mut content)?;

        let mut name = String::new();
        let mut name_block = true;
        let mut details:  Details = Details::new();
        let mut details_block = false;
        let mut links: Vec<Link> = Vec::new();
        let mut links_block = false;
        let mut filtering: Vec<String> = Vec::new();
        let mut filtering_block = 0;
        let mut delegation: Vec<Delegation> = Vec::new();
        let mut delegation_block = 0;
        let mut policies: Vec<Policy> = Vec::new();
        let mut policy_block = false;

        for line in content.lines(){
            
            // Name Parsing Logic
            if name_block && name.is_empty() && !line.is_empty() && !line.contains("────"){
                name = line.trim_start().trim_end().to_string();
                name_block = false;
            }

            // Detail Parsing Logic
            if line.starts_with("Details") && details_block == false && details.is_empty(){
                details_block = true;
            }
            else if details_block && line.is_empty() && !details.is_empty() {
                details_block = false;
            }
            else if details_block && !line.is_empty() {
                if line.starts_with("Domain"){
                    let split_line: Vec<&str> = line.split("│").collect();
                    details.set_domain(split_line[1].trim_end().to_string());
                }
                else if line.starts_with("Owner"){
                    let split_line: Vec<&str> = line.split("│").collect();
                    details.set_owner(split_line[1].trim_end().to_string());
                }
                else if line.starts_with("Created"){
                    let split_line: Vec<&str> = line.split("│").collect();
                    details.set_created(split_line[1].trim_end().to_string());
                }
                else if line.starts_with("Modified"){
                    let split_line: Vec<&str> = line.split("│").collect();
                    details.set_modified(split_line[1].trim_end().to_string());
                }
                else if line.starts_with("Unique ID"){
                    let split_line: Vec<&str> = line.split("│").collect();
                    details.set_id(split_line[1].trim_end().to_string());
                }
                else if line.starts_with("GPO Status"){
                    let split_line: Vec<&str> = line.split("│").collect();
                    details.set_status(split_line[1].trim_end().to_string());
                }
            }

            // Link Parsing Logic
            if line.starts_with("Links") && links_block == false && links.is_empty(){
                links_block = true;
            }
            else if links_block && line.is_empty() && !links.is_empty() {
                links_block = false;
            }
            else if links_block && !line.is_empty() && !line.contains("────") && !line.starts_with("Location") {
                let mut curr_link: Link = Link::new();
                let line_values: Vec<&str> = line.split("│").collect();
                if line_values.len() == 4 {
                    curr_link.set_location(&line_values[0].trim_end());
                    curr_link.set_enforced(&line_values[1].trim_end());
                    curr_link.set_status(&line_values[2].trim_end());
                    curr_link.set_path(&line_values[3].trim_end());
                    links.push(curr_link);
                }
            }

            // Security Filtering Parsing Logic
            // Find where the Security Filtering section starts
            if line.starts_with("Security Filtering") && filtering_block == 0 && filtering.is_empty() {
                filtering_block = 1;
            }
            // Find the table header
            else if filtering_block == 1 && line.starts_with("Name") {
                filtering_block = 2;
            }
            // After leaving the table
            else if filtering_block == 2 && (line.is_empty() || line.starts_with("Delegation")) && !filtering.is_empty() {
                filtering_block = 0;
            }
            // Items of the table
            else if filtering_block == 2 && !line.is_empty() && !line.contains("────") {
                filtering.push(line.to_string());
            }

            // Delegation parsing logic
            if line.starts_with("Delegation") && delegation_block == 0 && delegation.is_empty() {
                delegation_block = 1;
            }
            else if delegation_block == 1 && line.starts_with("Name") {
                delegation_block = 2;
            }
            else if delegation_block == 2 && (line.is_empty() || line.starts_with("User Configuration") || line.starts_with("Computer Configuration")) && !delegation.is_empty() {
                delegation_block = 0;
            }
            else if delegation_block == 2 && !line.is_empty() && !line.contains("────") {
                let mut new_delegation = Delegation::new();
                let split_full_string: Vec<&str> = line.split("│").collect();
                if split_full_string.len() == 3 {
                    let delegation_name = split_full_string[0];
                    let delegation_permissions = split_full_string[1];
                    let delegation_interited = split_full_string[2];

                    new_delegation.set_name(delegation_name.trim_end());
                    for permission in delegation_permissions.split(","){
                        new_delegation.add_permission(permission.trim_start().trim_end())
                    }
                    new_delegation.set_inheritence(delegation_interited.trim_end());

                    delegation.push(new_delegation);
                }
            }

            // Policy parsing logic
            if line.starts_with("Policy") && policy_block == false {
                policy_block = true;
            }
            else if policy_block && line.is_empty() {
                policy_block = false;
            }
            else if policy_block && !line.is_empty() && !line.starts_with("────") && (line.split("│").count() > 1) {
                let mut new_policy = Policy::new();
                let full_policy_string:Vec<&str> = line.split("│").collect();
                new_policy.set_value(full_policy_string[0].trim_start().trim_end());
                for setting in full_policy_string[1].split(","){
                    new_policy.add_setting(setting.trim_start().trim_end());
                }
                policies.push(new_policy);
            }
        }

        Ok(Self {
            name: name,
            details: details,
            links: links,
            filtering: filtering,
            delegation: delegation,
            policies: policies,
        })
    }

    // Name
    fn is_name(&self, name: &str) -> bool {
        self.name.to_lowercase() == name.to_lowercase()
    }

    // Details
    // fn is_id(&self, id: &str) -> bool {
    //     self.details.clone().is_id(id)
    // }
    // fn is_enabled(&self) -> bool {
    //     self.details.clone().is_enabled()
    // }
    // fn is_domain(&self, domain: &str) -> bool {
    //     self.details.clone().is_domain(domain)
    // }
    fn is_owner(&self, owner: &str) -> bool {
        self.details.clone().is_owner(owner)
    }

    // Links
    fn contains_link_location(&self, value: &str) -> i32 {
        let iterator = self.links.iter();
        for (index, link) in iterator.enumerate() {
            if link.clone().is_location(value) {
                return index as i32;
            }
        }
        -1
    }

    // fn is_link_active(&self, value: &str) -> bool {
    //     let link_to_check = self.contains_link_location(value);

    //     if link_to_check != -1 {
    //         let link_clone = self.links[link_to_check as usize].clone();
    //         link_clone.is_status("Enabled")
    //     } else {
    //         false
    //     }
    // }

    // Filtering
    fn contains_filter(&self, value: &str) -> bool {
        let iterator = self.filtering.iter();

        for filter in iterator {
            if value.starts_with(">"){
                if filter.to_lowercase().ends_with(&value[1..].to_lowercase()){
                    return true;
                }
            }
            else if value.starts_with("<"){
                if filter.to_lowercase().starts_with(&value[1..].to_lowercase()){
                    return true;
                }
            } else {
                if filter.to_lowercase() == value.to_lowercase() {
                    return true;
                }
            }
        }
        false
    }

    // Delegations
    fn contains_delegation_name(&self, value: &str) -> i32 {
        let iterator = self.delegation.iter();

        for (index, delegation) in iterator.enumerate() {
            if delegation.clone().is_name(value){
                return index as i32;
            }
        }
        -1
    }

    fn contains_delegation_permission(&self, delegation_name: &str, delegation_permission: &str) -> bool {
        let del_index = self.contains_delegation_name(&delegation_name);
        if del_index != -1 {
            let delegation_clone: Delegation = self.delegation[del_index as usize].clone();
            delegation_clone.contains_permission(delegation_permission)
        } else {
            false
        }   
    }

    fn is_delegation_inherited(&self, delegation_name: &str) -> bool {
        let perm_index = self.contains_delegation_name(&delegation_name);
        if perm_index != -1 {
            let delegation_clone: Delegation = self.delegation[perm_index as usize].clone();
            delegation_clone.is_inherited()
        } else {
            false
        }  
    }

    // Policies
    fn contains_policy(&self, policy_value: &str) -> i32 {
        let iterator = self.policies.iter();

        for (index, policy) in iterator.enumerate() {
            if policy.clone().is_value(&policy_value){
                return index as i32
            }
        }

        -1
    }

    fn contains_policy_setting(&self, policy_value: &str, policy_setting: &str) -> bool {
        let pol_index = self.contains_policy(policy_value);
        if pol_index != -1 {
            let policy_clone: Policy = self.policies[pol_index as usize].clone();
            policy_clone.contains_setting(policy_setting)
        } else {
            false
        }
    }

    pub fn query_gpo(&self, gpo_query_string: &str) -> bool {
        let without_comment: Vec<&str> = gpo_query_string.split("//").collect();
        if !without_comment[0].trim_start().trim_end().is_empty(){
            if gpo_query_string.to_lowercase().starts_with("name") && gpo_query_string.split("::").count() == 2 {
                // Name:Value
                let commentless:Vec<&str> = gpo_query_string.split("//").collect();
                let split_values: Vec<&str> = commentless[0].trim_start().trim_end().split("::").collect();
                let name_to_check = split_values[1];
                self.is_name(&name_to_check)
            }
            else if gpo_query_string.to_lowercase().starts_with("details") && gpo_query_string.split("::").count() == 2 {
                // Details:Owner
                let commentless:Vec<&str> = gpo_query_string.split("//").collect();
                let split_values: Vec<&str> = commentless[0].trim_start().trim_end().split("::").collect();
                let owner = split_values[1];
                if owner.starts_with("!"){
                    !self.is_owner(&owner[1..])
                } else {
                    self.is_owner(&owner)
                }
            }
            else if gpo_query_string.to_lowercase().starts_with("links") && gpo_query_string.split("::").count() == 2 {
                // Links:location
                let commentless:Vec<&str> = gpo_query_string.split("//").collect();
                let split_values: Vec<&str> = commentless[0].trim_start().trim_end().split("::").collect();
                let location = split_values[1];
                let location_index = self.contains_link_location(&location);
                return if location_index != -1 { true } else { false };
            }
            else if gpo_query_string.to_lowercase().starts_with("filtering") && gpo_query_string.split("::").count() == 2 {
                // Filtering:Value
                let commentless:Vec<&str> = gpo_query_string.split("//").collect();
                let split_values: Vec<&str> = commentless[0].trim_start().trim_end().split("::").collect();
                let filter = split_values[1];
                self.contains_filter(&filter)
            }
            else if gpo_query_string.to_lowercase().starts_with("delegation") && gpo_query_string.split("::").count() == 4 {
                // Delegation:Name:Permissions:Inherited
                let commentless:Vec<&str> = gpo_query_string.split("//").collect();
                let split_values: Vec<&str> = commentless[0].trim_start().trim_end().split("::").collect();
                let name = split_values[1];
                let permission = split_values[2];
                let inherited = split_values[3];
    
                (!name.is_empty() && !permission.is_empty() && self.contains_delegation_permission(&name, &permission)) && (inherited.is_empty() || if inherited.to_lowercase() == "yes" { self.is_delegation_inherited(&name) } else { !self.is_delegation_inherited(&name) })
            }
            else if gpo_query_string.to_lowercase().starts_with("policy") && gpo_query_string.split("::").count() == 3 {
                // Policy:Value:Setting
                let commentless:Vec<&str> = gpo_query_string.split("//").collect();
                let split_values: Vec<&str> = commentless[0].trim_start().trim_end().split("::").collect();
                let value = split_values[1];
                let setting = split_values[2];
                if setting.starts_with("!"){
                    self.contains_policy(&value) != -1 && !self.contains_policy_setting(&value, &setting[1..])
                } else {
                    self.contains_policy_setting(&value, &setting) || (setting.is_empty() && if self.contains_policy(&value) != -1 { true } else { false })
                }
            }
            else {
                false
            }
        }
        else {
            false
        }
    }

    pub fn get_matching_conditions(&self, gpo_query_string: &str) -> String {
        let mut match_string = String::new();
        let without_comment: Vec<&str> = gpo_query_string.split("//").collect();
        if !without_comment[0].trim_start().trim_end().is_empty(){
            if gpo_query_string.to_lowercase().starts_with("name") && gpo_query_string.split("::").count() == 2 {
                // Name:Value
                let commentless:Vec<&str> = gpo_query_string.split("//").collect();
                let split_values: Vec<&str> = commentless[0].trim_start().trim_end().split("::").collect();
                let name_to_check = split_values[1];
                if self.is_name(&name_to_check) {
                    match_string.push_str(self.name.as_str());
                }
                match_string.trim_start().trim_end().to_string()
            }
            else if gpo_query_string.to_lowercase().starts_with("details") && gpo_query_string.split("::").count() == 2 {
                // Details:Owner
                let commentless:Vec<&str> = gpo_query_string.split("//").collect();
                let split_values: Vec<&str> = commentless[0].trim_start().trim_end().split("::").collect();
                let owner = split_values[1];
                if owner.starts_with("!"){
                    if !self.is_owner(&owner[1..]) {
                        match_string.push_str(&self.details.owner);
                    }
                    match_string.trim_start().trim_end().to_string()
                } else {
                    if self.is_owner(&owner) {
                        match_string.push_str(&self.details.owner);
                    }
                    match_string.trim_start().trim_end().to_string()
                }
            }
            else if gpo_query_string.to_lowercase().starts_with("links") && gpo_query_string.split("::").count() == 2 {
                // Links:location
                let commentless:Vec<&str> = gpo_query_string.split("//").collect();
                let split_values: Vec<&str> = commentless[0].trim_start().trim_end().split("::").collect();
                let location = split_values[1];
                let location_index = self.contains_link_location(&location);
                if location_index != -1 {
                    match_string.push_str(&self.links[location_index as usize].to_string());
                    match_string.trim_start().trim_end().to_string()
                } else {
                    match_string.trim_start().trim_end().to_string()
                }
            }
            else if gpo_query_string.to_lowercase().starts_with("filtering") && gpo_query_string.split("::").count() == 2 {
                // Filtering:Value
                let commentless:Vec<&str> = gpo_query_string.split("//").collect();
                let split_values: Vec<&str> = commentless[0].trim_start().trim_end().split("::").collect();
                let filter = split_values[1];
                for gpo_filter in self.filtering.clone() {
                    if gpo_filter.contains(&filter){
                        match_string.push_str(&gpo_filter);
                    }
                }
                match_string.trim_start().trim_end().to_string()
            }
            else if gpo_query_string.to_lowercase().starts_with("delegation") && gpo_query_string.split("::").count() == 4 {
                // Delegation:Name:Permissions:Inherited
                let commentless:Vec<&str> = gpo_query_string.split("//").collect();
                let split_values: Vec<&str> = commentless[0].trim_start().trim_end().split("::").collect();
                let name = split_values[1];
                let permission = split_values[2];
                let inherited = split_values[3];

                for deleg in self.delegation.clone() {
                    if !name.is_empty() {
                        if permission.is_empty() && inherited.is_empty() {
                            if deleg.clone().is_name(name) {
                                match_string.push_str(&deleg.to_string());
                                match_string.push_str("\n");
                            }
                        }
                        else if inherited.is_empty() {
                            if deleg.clone().is_name(name) && deleg.clone().contains_permission(permission) {
                                match_string.push_str(&deleg.to_string());
                                match_string.push_str("\n");
                            }
                        } else {
                            if deleg.clone().is_name(name) && deleg.clone().contains_permission(permission) && deleg.clone().inherited == inherited {
                                match_string.push_str(&deleg.to_string());
                                match_string.push_str("\n");
                            }
                        }
                    }
                }
                match_string.trim_start().trim_end().to_string()
            }
            else if gpo_query_string.to_lowercase().starts_with("policy") && gpo_query_string.split("::").count() == 3 {
                // Policy:Value:Setting
                let commentless:Vec<&str> = gpo_query_string.split("//").collect();
                let split_values: Vec<&str> = commentless[0].trim_start().trim_end().split("::").collect();
                let value = split_values[1];
                let setting = split_values[2];
                if setting.starts_with("!"){
                    if self.contains_policy(&value) != -1 && 
                    !self.contains_policy_setting(&value, &setting[1..]) {
                        for policy in self.policies.clone() {
                            if policy.clone().is_value(value) && !policy.clone().contains_setting(setting){
                                match_string.push_str(&policy.to_string());
                                match_string.push_str("\n\t");
                            }
                        }
                    }
                    match_string.trim_start().trim_end().to_string()
                } else {
                    if self.contains_policy_setting(&value, &setting) ||
                    setting.is_empty() && self.contains_policy(&value) != -1 {
                        for policy in self.policies.clone() {
                            if policy.clone().is_value(value) && policy.clone().contains_setting(setting){
                                match_string.push_str(&policy.to_string());
                                match_string.push_str("\n\t");
                            }
                        }
                    }
                    match_string.trim_start().trim_end().to_string()
                }
            }
            else {
                match_string.trim_start().trim_end().to_string()
            }
        }
        else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn gen_empty_gpo() -> GroupPolicy {
        GroupPolicy {
            name: "".to_string(),
            details: Details::new(),
            links: Vec::new(),
            filtering: Vec::new(),
            delegation: Vec::new(),
            policies: Vec::new(),
        }
    }

    #[test]
    fn test_is_gpo_name() {
        let mut test_gpo = gen_empty_gpo();
        test_gpo.name = "TEST GPO".to_string();

        assert_eq!(test_gpo.is_name("TEST GPO"), true);
        assert_ne!(test_gpo.is_name("GPO TEST"), true);
    }

    // #[test]
    // fn test_is_id(){
    //     let mut test_gpo = gen_empty_gpo();
    //     test_gpo.details.set_id("af0a9f3d-0143-4721-9c82-570172cf71c3".to_string());

    //     assert_eq!(test_gpo.is_id("af0a9f3d-0143-4721-9c82-570172cf71c3"), true);
    //     assert_ne!(test_gpo.is_id("id"), true);
    // }

    // #[test]
    // fn test_is_enabled(){
    //     let mut test_gpo = gen_empty_gpo();
    //     test_gpo.details.set_status("enabled".to_string());

    //     assert_eq!(test_gpo.is_enabled(), true);
    // }

    // #[test]
    // fn test_is_domain(){
    //     let mut test_gpo = gen_empty_gpo();
    //     test_gpo.details.set_domain("labs.local".to_string());

    //     assert_eq!(test_gpo.is_domain("labs.local"), true);
    //     assert_ne!(test_gpo.is_domain("local.labs"), true);
    // }

    #[test]
    fn test_is_owner(){
        let mut test_gpo = gen_empty_gpo();
        test_gpo.details.set_owner("LABS\\Domain Admins".to_string());

        // Test Equals
        assert_eq!(test_gpo.is_owner("LABS\\Domain Admins"), true);
        // Test Ends With
        assert_eq!(test_gpo.is_owner(">Domain Admins"), true);
        // Test Starts With
        assert_eq!(test_gpo.is_owner("<LABS"), true);
    }

    #[test]
    fn test_link_location(){
        let mut test_gpo = gen_empty_gpo();
        test_gpo.links.push(
            Link {
                location: "Domain Controllers".to_string(),
                enforced: "No".to_string(),
                status: "Enabled".to_string(),
                path: "labs.local/Domain Controllers".to_string(),
            }
        );

        assert_eq!(test_gpo.contains_link_location("Domain Controllers"), 0);
        assert_eq!(test_gpo.contains_link_location("Domain Admins"), -1);
    }

    // #[test]
    // fn test_link_status(){
    //     let mut test_gpo = gen_empty_gpo();
    //     test_gpo.links.push(
    //         Link {
    //             location: "Domain Controllers".to_string(),
    //             enforced: "No".to_string(),
    //             status: "Enabled".to_string(),
    //             path: "labs.local/Domain Controllers".to_string(),
    //         }
    //     );

    //     assert_eq!(test_gpo.is_link_active("Domain Controllers"), true);
    //     assert_ne!(test_gpo.is_link_active("Domain Admins"), true);
    // }

    #[test]
    fn test_filtering(){
        let mut test_gpo = gen_empty_gpo();
        test_gpo.filtering.push(
            "NT AUTHORITY\\Authenticated Users".to_string()
        );

        // Test Equals
        assert_eq!(test_gpo.contains_filter("NT AUTHORITY\\Authenticated Users"), true);
        // Test Ends With
        assert_eq!(test_gpo.contains_filter(">Authenticated Users"), true);
        // Test Starts With
        assert_eq!(test_gpo.contains_filter("<NT AUTHORITY"), true);
    }

    #[test]
    fn test_delegation(){
        let mut test_gpo = gen_empty_gpo();
        test_gpo.delegation.push(
            Delegation {
                name: "NT AUTHORITY\\Authenticated Users".to_string(),
                permissions: vec!("Read (from Security Filtering)".to_string()),
                inherited: "No".to_string()
            }
        );

        // Test Equals
        assert_eq!(test_gpo.contains_delegation_permission("NT AUTHORITY\\Authenticated Users", "Read (from Security Filtering)"), true);
        // Test Ends With
        assert_eq!(test_gpo.contains_delegation_permission(">Authenticated Users", "Read (from Security Filtering)"), true);
        // Test Starts With
        assert_eq!(test_gpo.contains_delegation_permission("<NT AUTHORITY", "Read (from Security Filtering)"), true);
    }

    #[test]
    fn test_policy(){
        let mut test_gpo = gen_empty_gpo();
        test_gpo.policies.push(
            Policy {
                value: "Debug programs".to_string(),
                setting: vec!("BUILTIN\\Administrators".to_string())
            }
        );

        assert_eq!(test_gpo.contains_policy_setting("Debug programs", "BUILTIN\\Administrators"), true);
        assert_ne!(test_gpo.contains_policy_setting("Debug programs", "Everyone"), true);
    }

    #[test]
    fn test_policy_setting_number_operators(){
        let mut test_gpo = gen_empty_gpo();
        test_gpo.policies.push(
            Policy {
                value: "Minimum password length".to_string(),
                setting: vec!("7".to_string())
            }
        );

        assert_eq!(test_gpo.contains_policy_setting("Minimum password length", "#<=14"), true);
        assert_ne!(test_gpo.contains_policy_setting("Minimum password length", "#>=14"), true);
        assert_eq!(test_gpo.contains_policy_setting("Minimum password length", "#<8"), true);
        assert_ne!(test_gpo.contains_policy_setting("Minimum password length", "#>7"), true);
    }

    #[test]
    fn test_name_query() {
        let mut test_gpo = gen_empty_gpo();
        test_gpo.name = "TEST GPO".to_string();

        assert_eq!(test_gpo.query_gpo("Name::Test GPO"), true);
        assert_ne!(test_gpo.query_gpo("Name::GPO Test"), true);
    }

    #[test]
    fn test_detail_query() {
        let mut test_gpo = gen_empty_gpo();
        test_gpo.details.set_owner("LABS\\Domain Admins".to_string());

        assert_eq!(test_gpo.query_gpo("Details::LABS\\Domain Admins"), true);
        assert_eq!(test_gpo.query_gpo("Details::>Domain Admins"), true);
        assert_eq!(test_gpo.query_gpo("Details::<LABS"), true);
        assert_ne!(test_gpo.query_gpo("Details::Everyone"), true);
        // Test Is Not
        test_gpo.details.set_owner("Everybody".to_string());
        assert_eq!(test_gpo.query_gpo("Details::!>Domain Admins"), true)
    }

    #[test]
    fn test_links_query() {
        let mut test_gpo = gen_empty_gpo();
        test_gpo.links.push(
            Link {
                location: "Domain Controllers".to_string(),
                enforced: "No".to_string(),
                status: "Enabled".to_string(),
                path: "labs.local/Domain Controllers".to_string(),
            }
        );

        assert_eq!(test_gpo.query_gpo("Links::Domain Controllers"), true);
        assert_ne!(test_gpo.query_gpo("Links::Everyone"), true);
    }

    #[test]
    fn test_filter_query() {
        let mut test_gpo = gen_empty_gpo();
        test_gpo.filtering.push(
            "NT AUTHORITY\\Authenticated Users".to_string()
        );

        // Test Equals
        assert_eq!(test_gpo.query_gpo("Filtering::NT AUTHORITY\\Authenticated Users"), true);
        // Test Ends With
        assert_eq!(test_gpo.query_gpo("Filtering::>Authenticated Users"), true);
        // Test Starts With
        assert_eq!(test_gpo.query_gpo("Filtering::<NT AUTHORITY"), true);

        assert_ne!(test_gpo.query_gpo("Filtering::Everyone"), true)
    }

    #[test]
    fn test_delegation_query() {
        let mut test_gpo = gen_empty_gpo();
        test_gpo.delegation.push(
            Delegation {
                name: "NT AUTHORITY\\Authenticated Users".to_string(),
                permissions: vec!("Read (from Security Filtering)".to_string()),
                inherited: "Yes".to_string()
            }
        );

        // Test Equals
        assert_eq!(test_gpo.query_gpo("Delegation::NT AUTHORITY\\Authenticated Users::Read (from Security Filtering)::"), true);
        // Test Ends With
        assert_eq!(test_gpo.query_gpo("Delegation::NT AUTHORITY\\Authenticated Users::>(from Security Filtering)::"), true);
        assert_eq!(test_gpo.query_gpo("Delegation::>Authenticated Users::Read (from Security Filtering)::"), true);
        // Test Starts With
        assert_eq!(test_gpo.query_gpo("Delegation::<NT AUTHORITY::Read (from Security Filtering)::"), true);
        assert_eq!(test_gpo.query_gpo("Delegation::NT AUTHORITY\\Authenticated Users::<Read::"), true);
        // Test inherited
        assert_eq!(test_gpo.query_gpo("Delegation::NT AUTHORITY\\Authenticated Users::Read (from Security Filtering)::Yes"), true);
        test_gpo.delegation[0].set_inheritence("No");
        assert_eq!(test_gpo.query_gpo("Delegation::NT AUTHORITY\\Authenticated Users::Read (from Security Filtering)::No"), true);
        // Test missing name
        assert_ne!(test_gpo.query_gpo("Delegation::::Read (from Security Filtering)::"), true);
        // Test missing permission
        assert_ne!(test_gpo.query_gpo("Delegation::::NT AUTHORITY\\Authenticated Users::"), true);

        assert_ne!(test_gpo.query_gpo("Delegation::Everybody::Read (from Security Filtering)::"), true);
        assert_ne!(test_gpo.query_gpo("Delegation::Everybody::::"), true);
    }

    #[test]
    fn test_policy_query() {
        let mut test_gpo = gen_empty_gpo();
        test_gpo.policies.push(
            Policy {
                value: "Debug programs".to_string(),
                setting: vec!("BUILTIN\\Administrators".to_string())
            }
        );

        // Test Equals
        assert_eq!(test_gpo.query_gpo("Policy::Debug programs::BUILTIN\\Administrators"), true);
        // Test Ends With
        assert_eq!(test_gpo.query_gpo("Policy::Debug programs::>Administrators"), true);
        assert_eq!(test_gpo.query_gpo("Policy::>programs::BUILTIN\\Administrators"), true);
        // Test Starts With
        assert_eq!(test_gpo.query_gpo("Policy::Debug programs::<BUILTIN"), true);
        assert_eq!(test_gpo.query_gpo("Policy::<Debug::BUILTIN\\Administrators"), true);
        // Validate the setting can be empty -- Searching for policy's existence
        assert_eq!(test_gpo.query_gpo("Policy::Debug programs::"), true);
        // Validate the policy's value can't be empty
        assert_ne!(test_gpo.query_gpo("Policy::::BUILTIN\\Administrators"), true);
        // Test Is Not Query
        assert_eq!(test_gpo.query_gpo("Policy::Debug programs::!Everyone"), true);
        assert_eq!(test_gpo.query_gpo("Policy::Debug programs::!>Authenticated Users"), true);
        assert_eq!(test_gpo.query_gpo("Policy::Debug programs::!>Administrators"), false);
    }

    #[test]
    fn test_policy_number_operator_query(){
        let mut test_gpo = gen_empty_gpo();
        test_gpo.policies.push(
            Policy {
                value: "Minimum password length".to_string(),
                setting: vec!("7".to_string())
            }
        );

        assert_eq!(test_gpo.query_gpo("Policy::Minimum password length::#>=14"), false);
        assert_eq!(test_gpo.query_gpo("Policy::Minimum password length::#<14"), true);
    }

    #[test]
    fn test_query_comment() {
        let mut test_gpo = gen_empty_gpo();
        test_gpo.policies.push(
            Policy {
                value: "Minimum password length".to_string(),
                setting: vec!("7".to_string())
            }
        );

        assert_eq!(test_gpo.query_gpo("Policy::Minimum password length::#>=14 // This is a comment"), false);
        assert_eq!(test_gpo.query_gpo(" // This is a comment"), false);
        assert_eq!(test_gpo.query_gpo("This is a comment"), false);
        assert_eq!(test_gpo.query_gpo("Policy::Minimum password length::#<14  // This is a comment"), true);
    }

}