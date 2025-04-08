#![allow(warnings)]

use crate::bdecoder::OwnedValue;
use bendy::{encoding::AsString, value::Value};
use clap::{command, Arg, ArgAction, ArgMatches};
use std::collections::BTreeMap;
use std::fmt;
use std::{borrow::Cow, ops::Add};

#[derive(Default, Debug, Clone)]
pub struct File {
    pub length: usize,
    pub md5sum: Option<String>,
    pub path: String,
}
#[derive(Default, Debug, Clone)]
pub struct Info {
    pub files: Option<Vec<File>>,
    pub length: usize,
    pub name: String,
    pub piece_length: usize,
    pub pieces: Vec<[u8; 20]>,
}
#[derive(Default, Debug, Clone)]
pub struct MetaInfo {
    pub info: Info,
    pub announce: String,
    pub creation_date: Option<i64>,
    pub comment: Option<String>,
    pub created_by: Option<String>,
}

fn format_char(c: char) -> String {
    if (c as u32) < 0x20 || (c as u32) > 0x7E {
        // Format non-printable or non-ASCII characters
        format!("\\u{:04x}", c as u32)
    } else {
        // Return printable ASCII characters as they are
        c.to_string()
    }
}

impl fmt::Display for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Displaying the mandatory fields
        write!(
            f,
            "\t\t\"length\": \"{}\"\n\t\t\"path\": \"{}\"",
            self.length,
            self.path.chars().map(format_char).collect::<String>()
        )?;

        // For the optional 'md5sum' field
        if let Some(md5sum) = &self.md5sum {
            write!(f, "\n\t\t\"md5sum\": \"{}\"", md5sum)?;
        }

        Ok(())
    }
}

impl fmt::Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Displaying the mandatory fields
        write!(
            f,
            "\t\"length\": \"{}\"\n\t\"name\": \"{}\"\n\t\"piece-length\": \"{}\"",
            self.length,
            self.name.chars().map(format_char).collect::<String>(),
            self.piece_length
        );

        write!(f, "\n\t\"pieces\": [")?;
        for (i, piece) in self.pieces.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            for byte in piece {
                write!(f, "{:02x}", byte)?;
            }
        }
        write!(f, "]");

        // For the optional 'files' field
        if let Some(files) = &self.files {
            // Assuming each file in the vector also implements Display
            // You can format it as per your requirement
            write!(f, "\n\t\"files\":\n")?;
            for (i, file) in files.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", file)?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for MetaInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Start by displaying the mandatory fields
        write!(
            f,
            "{}\n\t\"announce\": \"{}\"",
            self.info,
            self.announce.chars().map(format_char).collect::<String>()
        )?;

        // For optional fields, you can use a pattern like this:
        if let Some(creation_date) = self.creation_date {
            write!(f, "\n\t\"creation-date\": \"{}\"", creation_date)?;
        }

        if let Some(comment) = &self.comment {
            write!(
                f,
                "\n\t\"comment: \"{}\"",
                comment.chars().map(format_char).collect::<String>()
            )?;
        }

        if let Some(created_by) = &self.created_by {
            write!(
                f,
                "\n\t\"created-by: \"{}\"",
                created_by.chars().map(format_char).collect::<String>()
            )?;
        }

        Ok(())
    }
}

fn parse_info(d: &BTreeMap<String, OwnedValue>) -> Info {
    /* Retrieve fields */
    let length: usize = d
        .get("length")
        .and_then(|x| extract_integer(x))
        .unwrap_or_default() as usize;
    let name = d.get("name").unwrap();
    let piece_length = d
        .get("piece length")
        .and_then(|x| extract_integer(x))
        .unwrap_or_default() as usize;
    let pieces = d
        .get("pieces")
        .and_then(|x| extract_groups_bytes(x))
        .unwrap_or_default();

    /* Retrive 'files' field */
    let list_files = extract_list_files(length, d);

    /* Initialize Info struct */
    let info_data = Info {
        files: Some(list_files),
        length,
        name: String::from_utf8(extract_bytes(name).unwrap()).unwrap(),
        piece_length,
        pieces,
    };
    info_data
}

fn extract_list_files(length: usize, d: &BTreeMap<String, OwnedValue>) -> Vec<File> {
    let mut list_files: Vec<File> = Vec::new();
    if length == 0 {
        let files = d.get("files").and_then(|x| extract_list(x)).unwrap();
        for file in files {
            if let OwnedValue::Dict(fdict) = file {
                let flength: usize = fdict
                    .get("length")
                    .and_then(|x| extract_integer(x))
                    .unwrap_or_default() as usize;
                let md5sum = Some(convert_option_bytes_to_string(
                    fdict.get("md5sum").and_then(|x| extract_bytes(x)),
                ));
                let fpath = fdict.get("path");
                let mut full_path = String::new();
                if let Some(OwnedValue::List(path)) = fpath {
                    for p in path {
                        full_path.push_str(&convert_option_bytes_to_string(extract_bytes(p)));
                        full_path.push_str("/");
                    }
                    full_path.pop(); 
                }

                let new_file = File {
                    length: flength,
                    md5sum,
                    path: full_path,
                };
                list_files.push(new_file);
            } else {
                panic!("parse_metainfo: error when extracting info[files]");
            }
        }
    }
    list_files
}

fn extract_integer(value: &OwnedValue) -> Option<i64> {
    if let OwnedValue::Integer(num) = value {
        Some(*num)
    } else {
        None
    }
}

fn extract_bytes(value: &OwnedValue) -> Option<Vec<u8>> {
    if let OwnedValue::Str(bytes) = value {
        Some(bytes.clone().as_bytes().to_vec())
    } else {
        None
    }
}

fn extract_groups_bytes(value: &OwnedValue) -> Option<Vec<[u8; 20]>> {
    if let OwnedValue::Str(bytes) = value {
        let byte_vec = bytes.clone().as_bytes().to_vec();
        if byte_vec.len() % 20 == 0 {
            let mut chunks = Vec::new();
            for chunk in byte_vec.chunks_exact(20) {
                let mut array = [0u8; 20];
                array.copy_from_slice(chunk);
                chunks.push(array);
            }
            Some(chunks)
        } else {
            // Handle the case where the bytes are not a multiple of 20
            None
        }
    } else {
        None
    }
}

fn extract_list(value: &OwnedValue) -> Option<&Vec<OwnedValue>> {
    if let OwnedValue::List(ref list) = value {
        Some(list)
    } else {
        None
    }
}

fn convert_option_bytes_to_string(option: Option<Vec<u8>>) -> String {
    option.map_or(String::from(""), |bytes| {
        String::from_utf8(bytes).unwrap_or_default()
    })
}

pub fn parse_metainfo(dict: BTreeMap<String, OwnedValue>) -> MetaInfo {
    /* Retrieve fields */
    let info = dict.get("info").expect("Required field missing: info");
    let announce = dict
        .get("announce")
        .expect("Required field missing: announce");
    let creation_date = dict.get("creation-date");
    let comment = dict.get("comment");
    let created_by = dict.get("created-by");

    if let OwnedValue::Dict(d) = info {
        let info_data = parse_info(d);

        /* Initialize MetaInfo struct */
        let meta_info = MetaInfo {
            info: info_data,
            announce: if let OwnedValue::Str(s) = announce {
                String::from_utf8(s.clone().as_bytes().to_vec()).unwrap()
            } else {
                String::from("")
            },
            creation_date: Some(
                creation_date
                    .and_then(|x| extract_integer(x))
                    .unwrap_or_default(),
            ),
            comment: Some(convert_option_bytes_to_string(
                comment.and_then(|x| extract_bytes(x)),
            )),
            created_by: Some(convert_option_bytes_to_string(
                created_by.and_then(|x| extract_bytes(x)),
            )),
        };

        meta_info
    } else {
        panic!("Wrong type for field info");
    }
}
