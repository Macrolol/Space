extern crate reqwest;
extern crate serde_derive;
use serde_derive::{Deserialize};


const REGTAP_URL: &str = "http://reg.g-vo.org/tap";
const REGTAP_CAPABILITIES_ENDPOINT: &str = "capabilities";

use quick_xml::de::{from_str, DeError};

#[derive(Debug, Deserialize)]
pub struct VOTable {
    #[serde(rename = "@version")]
    version: String,

    #[serde(rename = "RESOURCE")]
    resource: Resource,
}

#[derive(Debug, Deserialize)]
pub struct Resource {
    #[serde(rename = "@type")]
    resource_type: String,

    #[serde[rename = "INFO"]]
    info: Vec<Info>,

    #[serde(rename = "TABLE")]
    table: Vec<Table>,
}

#[derive(Debug, Deserialize)]
pub struct Info {
    #[serde(rename = "@name")]
    name: String,

    #[serde(rename = "@value")]
    value: String,
    
    #[serde(rename = "@ucd")]
    ucd: Option<String>,
    
    #[serde(rename = "$text")]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Table {
    #[serde(rename = "@name")]
    name: String,

    #[serde(rename = "GROUP")]
    groups: Vec<Group>,

    #[serde(rename = "FIELD")]
    fields: Vec<Field>,

    #[serde(rename = "DATA")]
    data: Data,
}

#[derive(Debug, Deserialize)]
pub struct Group {
    #[serde(rename = "@ID")]
    id: String,

    #[serde(rename = "@name")]
    name: String,
    
    #[serde(rename = "DESCRIPTION")]
    description: Option<Description>,

    field: Option<Field>,

}

#[derive(Debug, Deserialize)]
pub enum Field {
    #[serde(rename = "FIELD")]
    FieldMetadata{
        #[serde(rename = "@ID")]
        id: String,
    
        name: Option<String>,
        arraysize: Option<String>,
        datatype: Option<String>,
        utype: Option<String>,
        ucd: Option<String>,
        unit: Option<String>,
    
        #[serde(rename = "DESCRIPTION")]
        description: Option<Description>,    
    },
    #[serde(rename = "FIELDref")]
    FieldReference{
        #[serde(rename = "@ref")]
        reference: String,
    }
}

#[derive(Debug, Deserialize)]
pub struct Description {
    #[serde(rename = "$text")]
    text: String,
}

#[derive(Debug, Deserialize)]
pub struct Data{
    #[serde(rename = "$value")]
    content: DataType
}

#[derive(Debug, Deserialize)]
pub enum DataType {
    #[serde(rename = "TABLEDATA")]
    TableData(TableData),

    #[serde(rename = "BINARY")]
    BinaryData(Binary),
}

#[derive(Debug, Deserialize)]
pub struct TableData {
    #[serde(rename = "TR")]
    rows: Vec<Row>,
}

#[derive(Debug, Deserialize)]
pub struct Row {
    #[serde(rename = "TD")]
    cells: Vec<Cell>,
}

#[derive(Debug, Deserialize)]
pub struct Cell {
    #[serde(rename = "$text")]
    text: String,
}

#[derive(Debug, Deserialize)]
pub struct Binary{   
    #[serde(rename = "STREAM")]
    stream: Stream,
}

#[derive(Debug, Deserialize)]
pub struct Stream {
    #[serde(rename = "@encoding")]
    encoding: String,

    #[serde(rename = "@compression")]
    compression: Option<String>,

    #[serde(rename = "$text")]
    text: Option<String>,
}


#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_parse(){
        let xml = include_str!("../tests/resources/test_response.xml");
        let vo_table: VOTable = from_str(xml).unwrap();
        println!("{:#?}", vo_table);
    }
}