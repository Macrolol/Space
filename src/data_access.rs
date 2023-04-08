extern crate polars;
use polars::prelude::*;

use std::fmt::Display;
use std::str::FromStr;
use std::fmt;
use std::collections::HashMap;
use std::fmt::Formatter;
use std::result::Result;
use std::result::Result::Err;
use std::result::Result::Ok;
use std::error::Error as StdError;

type Unit = String;
type Ucd = String;

#[derive(Debug, Clone)]
pub struct ColumnMetadata{
    pub name: String,
    pub description: String,
    pub data_type: DataType,
    pub unit: Option<Unit>,
    pub ucd: Option<Ucd>,
    pub utype: Option<String>,
    pub principal: Option<bool>,
    pub indexed: Option<bool>,
    pub std: Option<bool>,
    pub size: Option<u64>,
    pub width: Option<u64>,
    pub precision: Option<u64>,
    pub xtype: Option<String>,
}

impl ColumnMetadata {
    pub fn new(name: String, description: String, data_type: DataType) -> Self {
        ColumnMetadata {
            name,
            description,
            data_type,
            unit: None,
            ucd: None,
            utype: None,
            principal: None,
            indexed: None,
            std: None,
            size: None,
            width: None,
            precision: None,
            xtype: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableMetadata {
    pub name: String,
    pub description: String,
    pub columns: Vec<ColumnMetadata>,
}




pub trait DAResult<'a>{
    fn get_data(&'a self) -> Result<&'a DataFrame, Box<dyn std::error::Error>>;
    fn get_metadata(&'a self) -> Result<&'a TableMetadata, Box<dyn std::error::Error>>;
}

impl fmt::Display for dyn DAResult<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "DAResult {{ data: {:?}, metadata: {:?} }}", self.get_data(), self.get_metadata())
    }
}

//Error type for data access
#[derive(Debug, Clone)]
pub enum DataAccessError {
    //Error retrieving data from a service
    DataRetrievalError(String),
    //Error retrieving metadata from a service
    MetadataRetrievalError(String),
    //Error parsing data from a service
    DataParseError(String),
    //Error parsing metadata from a service
    MetadataParseError(String),
    //Error parsing data from a service
    DataValidationError(String),
    //Error parsing metadata from a service
    MetadataValidationError(String),
}

impl Display for DataAccessError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            DataAccessError::DataRetrievalError(msg) => write!(f, "DataRetrievalError: {}", msg),
            DataAccessError::MetadataRetrievalError(msg) => write!(f, "MetadataRetrievalError: {}", msg),
            DataAccessError::DataParseError(msg) => write!(f, "DataParseError: {}", msg),
            DataAccessError::MetadataParseError(msg) => write!(f, "MetadataParseError: {}", msg),
            DataAccessError::DataValidationError(msg) => write!(f, "DataValidationError: {}", msg),
            DataAccessError::MetadataValidationError(msg) => write!(f, "MetadataValidationError: {}", msg),
        }
    }
}
impl std::error::Error for DataAccessError {}




#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    pub struct DAResultImpl {
        pub data: Option<DataFrame>,
        pub error: Option<DataAccessError>,
        pub metadata: Option<HashMap<String, String>>,
    }



impl <'a> DAResult<'a> for DAResultImpl{
    fn get_data(&'a self) -> Result<&'a DataFrame, Box<dyn std::error::Error>> {
        match &self.data {
            Some(data) => Ok(data),
            None => Err(Box::new(self.error.clone().unwrap()))
        }
    }

    fn get_metadata(&'a self) -> Result<&'a HashMap<String, String>, Box<dyn std::error::Error>> {
        match &self.metadata {
            Some(metadata) => Ok(metadata),
            None => Err(Box::new(self.error.clone().unwrap()))
        }
    }
}

    #[test]
    fn test_good_data_access(){
        let data = DataFrame::new(vec![
            Series::new("a", &[1, 2, 3]),
            Series::new("b", &[4, 5, 6]),
        ]).unwrap();
        let metadata = HashMap::new();
        let dar = DAResultImpl{data: Some(data), error: None, metadata: Some(metadata)};
        assert!(dar.get_data().is_ok());
        assert!(dar.get_metadata().is_ok());
    }

    #[test]
    fn test_bad_data_access(){
        let data = DataFrame::new(vec![
            Series::new("a", &[1, 2, 3]),
            Series::new("b", &[4, 5, 6]),
        ]).unwrap();
        let metadata = HashMap::new();
        let dar = DAResultImpl{data: None, error: Some(DataAccessError::DataRetrievalError("Error retrieving data".to_string())), metadata: Some(metadata)};
        assert!(dar.get_data().is_err());
        assert!(dar.get_metadata().is_ok());
    }

    #[test]
    fn test_bad_metadata_access(){
        let data = DataFrame::new(vec![
            Series::new("a", &[1, 2, 3]),
            Series::new("b", &[4, 5, 6]),
        ]).unwrap();
        let metadata = HashMap::new();
        let dar = DAResultImpl{data: Some(data), error: None, metadata: None};
        assert!(dar.get_data().is_ok());
        assert!(dar.get_metadata().is_err());
    }


}