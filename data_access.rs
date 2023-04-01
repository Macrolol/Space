extern crate polars;
use polars::prelude::*;

use std::str::FromStr;
use std::fmt;
use std::collections::HashMap;
use std::fmt::Formatter;
use std::error::Error;
use std::result::Result;
use std::result::Result::Err;
use std::result::Result::Ok;
use std::error::Error as StdError;




#[derive(Debug)]
pub struct FieldMetadata {
    name: String,
    datatype: String,
    description: String,
    unit: Option<u::Unit>,
    ucd: String,
    arraysize: String,
    xtype: String,
    reference: String,
    dtype: Option<DataType>,
}

impl FieldMetadata {
    pub fn from_field(field: ap::io::votable::tree::Field) -> Self {
        Self {
            name: field.name,
            datatype: field.datatype,
            description: field.description,
            unit: if field.unit.is_empty() { None } else { Some(u::Unit::from_str(&field.unit).unwrap()) },
            ucd: field.ucd,
            arraysize: field.arraysize,
            xtype: field.xtype,
            reference: field.reference,
            dtype: None,
        }
    }

    pub fn to_polars_dtype(&self) -> DataType {
        if self.datatype == "char" {
            return Utf8;
        } else if self.datatype == "double" {
            return Float64;
        } else if self.datatype == "float" {
            return Float32;
        } else if self.datatype == "int" {
            return Int32;
        } else if self.datatype == "long" {
            return Int64;
        } else if self.datatype == "short" {
            return Int16;
        } else if self.datatype == "boolean" {
            return Boolean;
        } else {
            return Utf8;
        }
    }

    pub fn to_json(&self) -> String {
        return serde_json::to_string(self).unwrap();
    }

    pub fn from_json(json_str: String) -> Result<Self, Box<dyn Error>> {
        let json_dict: HashMap<String, String> = serde_json::from_str(&json_str)?;
        let mut json_dict = json_dict;
        if json_dict["unit"].is_empty() {
            json_dict["unit"] = None;
        } else {
            json_dict["unit"] = Some(u::Unit::from_str(&json_dict["unit"]).unwrap());
        }

        return Self::new(json_dict);
    }
}