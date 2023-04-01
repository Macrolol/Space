extern crate reqwest;
use std::boxed::Box;
use std::fmt::Display;
use std::str::FromStr;
use quick_xml::reader::Reader;
use quick_xml::events::{Event, BytesStart};
use std::option::Option;

type Name = String;

#[derive(Clone)]
pub struct XmlAttribute {
    pub name: Name,
    pub value: String,
}

impl Display for XmlAttribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}=\"{}\"", self.name, self.value)
    }
}

impl XmlAttribute {

    fn new(name: Name, value: String) -> Self {
        Self {
            name,
            value
        }
    }

    fn read_from<'a>(event: BytesStart<'a>, reader: &'a mut Reader<&'a [u8]>) -> Vec<XmlAttribute> {
        event.attributes()
             .map(|attr| {
            let attr = attr.unwrap();
            XmlAttribute {
                name: format!("{:?}", attr.key),
                value: format!("{:?}", attr.value)
            }
        }).collect()
    }
}

#[derive(Clone)]
pub enum XmlElement<'a>{
    Root(Vec<XmlElement<'a>>),
    Branch{
        name: Name,
        attrs: Vec<XmlAttribute>,
        children: Vec<XmlElement<'a>>
    },
    Leaf{
        name: Name,
        attrs: Vec<XmlAttribute>,
        value: Option<String>,
    },
    SomethingElse(Event<'a>)
}



#[derive(Debug)]
pub enum XmlError<'a>{
    EndOfXml,
    StrangeEvent(Event<'a>),
    ReadError(quick_xml::Error),
    InvalidStartingEvent(Event<'a>),
}

impl Display for XmlError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XmlError::EndOfXml => write!(f, "End of XML"),
            XmlError::StrangeEvent(v) => write!(f, "Strange event: {:?}", v),
            XmlError::ReadError(e) => write!(f, "Read error: {:?}", e),
            XmlError::InvalidStartingEvent(e) => write!(f, "Invalid starting event: {:?}", e)
        }
    }
}

impl std::error::Error for XmlError<'_> {}

impl XmlElement<'_> {
    fn read_from(reader: &mut Reader<&[u8]>) -> Result<Self, Box<dyn std::error::Error>> {
        let event = reader.read_event().unwrap();
        let mut buf = Vec::new();
        match event {
            Event::Start(e) => {
                let name = String::from(e.name().0.iter().collect());
                let attrs = XmlAttribute::read_from(e, reader);
                loop {
                    let read = match reader.read_event()? {
                        Event::Start(e) => Ok(XmlElement::Branch {
                                name,
                                attrs,
                                children: Self::read_children(reader).iter()
                                                                        .filter(|r| match r {
                                                                            Ok(e) => true,
                                                                            Err(e) => match e.downcast_ref() {
                                                                                Some(XmlError::EndOfXml) => false,
                                                                                _ => true
                                                                                }
                                                                        })
                                                                        .map(|r|{
                                                                            match r {
                                                                                Ok(e) => Some(e.to_owned()),
                                                                                Err(e) => match e.downcast_ref() {
                                                                                    Some(xmle) =>
                                                                                        match xmle {
                                                                                            XmlError::EndOfXml => None,
                                                                                            XmlError::StrangeEvent(e) | XmlError::InvalidStartingEvent(e) => Some(XmlElement::SomethingElse(e.to_vec())),
                                                                                            XmlError::ReadError(e) => Some(XmlElement::SomethingElse(e.to_string().as_bytes().to_vec())),
                                                                                            
                                                                                        },
                                                                                    None => None
                                                                                }
                                                                            }
                                                                        })
                                                                        .filter(|e| e.is_some())
                                                                        .map(|e| e.unwrap())
                                                                        .collect()
                        }),
                        Event::End(e) => Err(Box::new(XmlError::EndOfXml)),
                        Event::Eof => Err(Box::new(XmlError::EndOfXml)),
                        e => Err(Box::new(XmlError::StrangeEvent(e)))
                    };
                    match read {
                        Ok(e) => return Ok(e),
                        Err(e) => return Err(e)
                    }
                }
            },
            Event::End(e) => Ok({
                XmlElement::Leaf {
                    name: String::from(e.name().0.iter().collect()),
                    attrs: Vec::new(),
                    value: None
                }
            }),
            e => Err(Box::new(XmlError::StrangeEvent(e.)))
        }
    }

    fn read_children(reader: &mut Reader<&[u8]>) -> Vec<Result<Self, Box<dyn std::error::Error>>> {
        let mut children: Vec<Result<Self, Box<dyn std::error::Error>>> = Vec::new();
        loop {
            let event = reader.read_event();
            match event {
                Err(e) => {
                    children.push(Err(Box::new(XmlError::ReadError(e))));
                },
                Ok(e) => {
                    match e {
                        Event::Start(e) => {
                            children.push(Self::read_from(reader));
                        },
                        Event::End(e) => break,
                        Event::Eof => break,
                        _ => {}
                    }
                }
            }
        };
        Vec::from_iter(children.iter()
                .map(|r| {
                    match r {
                            Ok(e) => Ok(e.to_owned()),
                            Err(e) => Err(e.to_owned())
                        }
                    }))
    }
}
    


pub struct XmlDocument {
    raw: String,
    buffer: Vec<u8>
}

impl XmlDocument {
    
    fn offset(&self) -> usize {
        self.buffer.len()
    }

    fn unread(&self) -> &str{
        &self.raw[self.offset()..]
    }

    fn reader(&self) -> Reader<&[u8]> {
        Reader::from_str(self.unread())
    }

}

impl Iterator for XmlDocument {
    type Item = XmlElement;

    fn next(&mut self) -> Option<Self::Item> {
        loop {}
    }
}

impl<'a> FromStr for XmlDocument {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            raw: s.to_owned(),
            buffer: Vec::new()
        })
    }
}

pub trait XmlEndpoint{

    fn get_string_response(&self) -> Result<String, Box<dyn std::error::Error>>;
    fn get_xml_response(&self) -> Result<XmlDocument, Box<dyn std::error::Error>> {
        let response = self.get_string_response()?;
        let e = Reader::from_str(response.as_str());
        XmlDocument::from_str(response.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use mockito::Matcher;

    #[tokio::test]
    async fn test_get_xml_response() {
        let mut server = Server::new();
        let m = Server::mock(&mut server, "GET", "/");
        m.match_header("accept", "application/xml")
            .with_status(200)
            .with_body("<root><child>value</child></root>")
            .create();


        let endpoint = XmlEndpointImpl::new(server.url());
        let response = endpoint.get_xml_response();
        assert!(response.is_ok());
        let xml = response.unwrap();

    }

    struct XmlEndpointImpl {
        url: String,
    }

    impl XmlEndpointImpl {
        pub fn new(url: String) -> Self {
            Self {
                url,
            }
        }
    }

    impl XmlEndpoint for XmlEndpointImpl {
        fn get_string_response(&self) -> Result<String, Box<dyn std::error::Error>> {
            let client = reqwest::blocking::Client::new();
            let response = client.get(self.url.clone())
                .send();
            Ok(response.unwrap().text().unwrap())
        }
    }
}

