
extern crate reqwest;
use std::boxed::Box;
use std::fmt::Display;
use std::io::Read;
use std::str::FromStr;
use quick_xml::reader::{Reader, Span};

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

    fn read_from<'a>(event: BytesStart<'a>) -> Vec<XmlAttribute> {
        event.attributes()
             .map(|attr| {
            let attr = attr.unwrap();
            XmlAttribute {
                name: String::from_utf8(attr.key.0.to_vec()).unwrap(),
                value: String::from_utf8(attr.value.to_vec()).unwrap(),
            }
        }).collect()
    }
}

type XmlText = String;

#[derive(Clone)]
pub enum XmlElement<'a>{
    OpenTag(Name, Vec<XmlAttribute>),
    EmptyTag(Name, Vec<XmlAttribute>),
    Text(XmlText),
    SomethingElse(Event<'a>),
    ElementEnd(Name),
    End
}


impl <'a> XmlElement<'a> {
    fn parse_event(event: Event) -> Result<XmlElement, XmlError> {
        match event {
            Event::Empty(open) => {
                Ok(XmlElement::EmptyTag(
                    String::from_utf8(open.name().0.to_vec()).unwrap(),
                    XmlAttribute::read_from(open.into_owned())
                ))
            }Event::Start(open) => {
                Ok(XmlElement::OpenTag(
                    String::from_utf8(open.name().0.to_vec()).unwrap(),
                    XmlAttribute::read_from(open.into_owned())
                ))
            },
            Event::Text(text) => {
                Ok(XmlElement::Text(String::from_utf8(text.to_vec()).unwrap()))
            },
            Event::End(close) => Ok(XmlElement::ElementEnd(String::from_utf8(close.name().0.to_vec()).unwrap())),
            Event::Eof => Ok(XmlElement::End),
            _ => Err(XmlError::StrangeEvent(event.into_owned())),
        }
    }
}

impl <'a> Display for XmlElement<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XmlElement::OpenTag(name, attrs) => {
                match attrs.len() {
                    0 => write!(f, "<{}>", name),
                    _ => write!(f, "<{} {}>", name, attrs.iter().map(|attr| attr.to_string()).collect::<Vec<String>>().join(" "))
                }
            },
            XmlElement::EmptyTag(name, attrs) => {
                match attrs.len() {
                    0 => write!(f, "<{}/>", name),
                    _ => write!(f, "<{} {}/>", name, attrs.iter().map(|attr| attr.to_string()).collect::<Vec<String>>().join(" "))
                }
            },
            XmlElement::Text(text) => {
                write!(f, "{}", text)
            },
            XmlElement::SomethingElse(event) => {
                write!(f, "{}", String::from_utf8(event.to_vec()).unwrap())
            },
            XmlElement::ElementEnd(name) => {
                write!(f, "</{}>", name)
            },
            XmlElement::End => {
                write!(f, "End of XML")
            }
        }
    }
}

#[derive(Debug, Clone)]
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

#[derive(Clone)]
pub struct XmlDocument<'a>(Vec<XmlElement<'a>>);


impl <'a> From<&mut Reader<&'a[u8]>> for XmlDocument<'a>{
    fn from(reader: &mut Reader<&'a[u8]>) -> Self {
        let mut events = Vec::new();
        loop {
            events.push(
                match reader.read_event() {
                Err(e) => {
                    panic!("Error: {:?}", {e});
                }
                Ok(event) => {
                    match XmlElement::parse_event(event){
                        Err(e) => {
                            panic!("Error: {:?}", {e});
                        }
                        Ok(elem) => match elem {
                                XmlElement::End => break,
                                _ => elem
                            },
                    }
                }
            });
        };
        XmlDocument(events.to_owned())
    }
}


impl <'a> XmlDocument<'a> {
    pub fn name(&self) -> Option<&str> {
        match self.0.first() {
            Some(XmlElement::OpenTag(name, _)) => Some(name),
            Some(XmlElement::EmptyTag(name, _)) => Some(name),
            _ => None
        }
    }

    pub fn depth(&self) -> usize {
        let mut depth = 0;
        let mut max_depth = 0;
        self.0.iter().for_each(|e| {
            match e {
                XmlElement::OpenTag(_, _) => {
                    depth += 1;
                    if depth > max_depth {
                        max_depth = depth;
                    }
                },
                XmlElement::ElementEnd(_) => {
                    depth -= 1;
                },
                _ => {}
            }
        });
        max_depth.to_owned()
    }

    pub fn flatten(&'a self) -> impl Iterator<Item=&XmlElement<'a>> {
        XmlIterator::new(self)
    }

    pub fn get_element_by_name(&self, name: &str) -> Option<XmlElement> {
        self.flatten().find_map(|el|{
            match el {
                XmlElement::OpenTag(n, _) => {
                    if n == name {
                        Some(el.to_owned())
                    } else {
                        None
                    }
                },
                XmlElement::EmptyTag(n, _) => {
                    if n == name {
                        Some(el.to_owned())
                    } else {
                        None
                    }
                },
                _ => None
            }
        })
    }

    pub fn get_element_at_index(&'a self, index: usize) -> Option<XmlElement<'a>> {
        self.flatten().nth(index).cloned()
    }

    pub fn iter_subtrees(&self) -> impl Iterator<Item=XmlDocument> {
        XmlDocIterator::new(self)
    }

    pub fn get_subtree_by_name(&self, name: &str) -> Option<XmlDocument> {
        self.iter_subtrees().find_map(|doc|{
            match doc.name() {
                Some(n) => {
                    if n == name {
                        Some(doc)
                    } else {
                        None
                    }
                },
                None => None
            }
        })
    }

    
}

impl Display for XmlDocument<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let write_result = self.0.iter().find_map(|e| {
            match write!(f, "{}", e) {
                Err(e) => return Some(e),
                _ => {None}
            }
        });

        match write_result {
            Some(e) => Err(e),
            None => Ok(())
        }
    }
} 

pub struct XmlIterator<'a>{
    xml: &'a XmlDocument<'a>,
    index: usize,
}

impl <'a> XmlIterator<'a> {
    pub fn new(xml: &'a XmlDocument<'a>) -> Self {
        XmlIterator {
            xml,
            index: 0
        }
    }
}

impl <'a> Iterator for XmlIterator<'a> {
    type Item = &'a XmlElement<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.xml.0.get(self.index);
        self.index += 1;
        item
    }
}

pub struct XmlDocIterator<'a>{
    xml: &'a XmlDocument<'a>,
    index: usize,
}

impl <'a> XmlDocIterator<'a> {
    pub fn new(xml: &'a XmlDocument<'a>) -> Self {
        XmlDocIterator {
            xml,
            index: 0
        }
    }
}

impl <'a> Iterator for XmlDocIterator<'a> {
    type Item = XmlDocument<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let start = self.xml.get_element_at_index(self.index)?;
        match start {
            XmlElement::OpenTag(_, _) => {
                let mut depth = 1;
                let mut end_index = self.index;
                for (i, el) in self.xml.0.iter().enumerate().skip(self.index + 1) {
                    match el {
                        XmlElement::OpenTag(_, _) => {
                            depth += 1;
                        },
                        XmlElement::ElementEnd(_) => {
                            depth -= 1;
                            if depth == 0 {
                                end_index = i;
                                break;
                            }
                        },
                        _ => {}
                    }
                }
                let subtree = self.xml.0[self.index..end_index+1].to_owned();
                self.index = end_index + 1;
                Some(XmlDocument(subtree))
            },
            XmlElement::EmptyTag(_, _) => {
                self.index += 1;
                Some(XmlDocument(vec![start.to_owned()]))
            },
            _ => None
        }
    }
}




