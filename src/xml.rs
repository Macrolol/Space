extern crate reqwest;
use std::boxed::Box;
use std::fmt::Display;
use std::io::Read;
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


pub trait XmlEndpoint<'a>{

    fn get_string_response(&'a mut self) -> Result<&'a str, Box<dyn std::error::Error>>;
    fn get_xml_response(&'a mut self) -> Result<XmlDocument<'a>, Box<dyn std::error::Error>> {
        let response = self.get_string_response()?;
        let reader = Reader::from_str(response);
        let doc = XmlDocument::from(&mut reader.to_owned());
        Ok(doc.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use mockito::Matcher;
    


    struct XmlEndpointImpl {
        url: String,
        responses: Vec<String>
    }

    impl XmlEndpointImpl{
        pub fn new(url: String) -> Self {
            Self {
                url,
                responses: Vec::new()
            }
        }
    }

    impl XmlEndpoint<'_> for XmlEndpointImpl {
        fn get_string_response(&mut self) -> Result<&str, Box<dyn std::error::Error>> {
            let client = reqwest::blocking::Client::new();
            let response = client.get(self.url.to_owned())
                .send()?;
            let text = response.text()?;
            println!("Response: {}", text);
            self.responses.push(text);
            Ok(self.responses.last().unwrap())
        }
    }

    fn mock_xml(server: &mut Server, xml: &[u8], endpoint: &str,) ->  mockito::Mock {
        Server::mock(server, "GET", endpoint)
            .with_status(200)
            .with_header("content-type", "text/xml")
            .with_body(xml)
            .create()
    }
        
    #[derive(Clone, Debug)]
    enum XmlTest {
        StrTest{
            xml: &'static [u8],
            endpoint: &'static str,
            expected: &'static str
        },
        PtrTest{
            xml: &'static [u8],
            endpoint: &'static str,
            expected: usize
        }
    }



    async fn depth_test(test: XmlTest) {
        match test {
            XmlTest::PtrTest {xml, endpoint, expected} =>{
            let mut server = Server::new();
            let _mock = mock_xml(&mut server, xml, endpoint);
            let url = format!("{}{}", server.url(), endpoint);
            tokio::task::spawn_blocking(move ||{
                    let mut endpoint = XmlEndpointImpl::new(url);
                    match endpoint.get_xml_response() {
                        Ok(doc) => {
                            println!("Ok: {}", doc.to_string());
                            println!("Depth: {}", doc.depth());
                            assert_eq!(doc.depth(), expected);
                        },
                        Err(e) => {
                            println!("Error: {}", e);
                            assert!(false);
                        }
                    }
                }).await.unwrap();
            },
            XmlTest::StrTest {xml, endpoint, expected} =>{ assert!(false, "WRONG TYPE OF TEST"); }
        }
        
    }

    #[tokio::test]
    async fn run_depth_test() {
        let depth_tests = vec![
            XmlTest::PtrTest {
                xml: b"",
                endpoint: "/depth",
                expected: 0
            },
            XmlTest::PtrTest {
                xml: b"<root>value</root>",
                endpoint: "/depth",
                expected: 1
            },
            XmlTest::PtrTest {
                xml: b"<root><child>value</child></root>",
                endpoint: "/depth",
                expected: 2
            },
            XmlTest::PtrTest {
                xml: b"<root><child><child>value</child></child></root>",
                endpoint: "/depth",
                expected: 3
            },
            XmlTest::PtrTest {
                xml: b"<root><child><child><child>value</child></child></child></root>",
                endpoint: "/depth",
                expected: 4
            },
            XmlTest::PtrTest {
                xml: b"<root><child><child><child><child>value</child></child></child></child></root>",
                endpoint: "/depth",
                expected: 5
            }    
        ];

        for test in depth_tests {
            depth_test(test).await;
        }
    }

    async fn get_subtree_by_name_test(test: XmlTest){
        let target = "target";
        match test {
            XmlTest::StrTest {xml, endpoint, expected} =>{
                let mut server = Server::new();
                let _mock = mock_xml(&mut server, xml, endpoint);
                let url = format!("{}{}", server.url(), endpoint);
                tokio::task::spawn_blocking(move ||{
                        let mut endpoint = XmlEndpointImpl::new(url);
                        match endpoint.get_xml_response() {
                            Ok(doc) => {
                                println!("Ok: {}", doc.to_string());
                                match doc.get_subtree_by_name(target) {
                                    Some(subtree) => {
                                        println!("Subtree: {}", subtree.to_string());
                                        assert_eq!(doc.get_subtree_by_name(target).unwrap().to_string(), expected);
                                    },
                                    None => {
                                        println!("Subtree not found");
                                        assert!(false);
                                    }
                                }
                                
                            },
                            Err(e) => {
                                println!("Error: {}", e);
                                assert!(false);
                            }
                        }
                    }).await.unwrap();
            },
            _ =>{ assert!(false, "WRONG TYPE OF TEST"); }
        }
    }


    #[tokio::test]
    async fn run_get_subtree_by_name_tests(){
        let get_subtree_by_name_tests = vec![
            XmlTest::StrTest {
                xml: b"<target><child>value</child></target>",
                endpoint: "/subtree",
                expected: "<target><child>value</child></target>"
            },
            XmlTest::StrTest {
                xml: b"<root><target>value</target></root>",
                endpoint: "/subtree",
                expected: "<target>value</target>"
            },
            XmlTest::StrTest {
                xml: b"<root><target><child>value</child></target></root>",
                endpoint: "/subtree",
                expected: "<target><child>value</child></target>"
            },
            XmlTest::StrTest {
                xml: b"<root><target><child>value1</child></target><target>value2</target></root>",
                endpoint: "/subtree",
                expected: "<target><child>value1</child></target>"
            },
            XmlTest::StrTest {
                xml: b"<root><target><child>value1</child></target><target><child>value2</child></target></root>",
                endpoint: "/subtree",
                expected: "<target><child>value1</child></target>"
            },
            XmlTest::StrTest {
                xml: b"<root><target><child>value1</child></target><target><child>value2</child></target><target><child>value3</child></target></root>",
                endpoint: "/subtree",
                expected: "<target><child>value1</child></target>"
            },
            XmlTest::StrTest {
                xml: b"<root><targetish><child>value1</child></targetish><target><child>value2</child></target><target><child>value3</child></target><target><child>value4</child></target></root>",
                endpoint: "/subtree",
                expected: "<target><child>value2</child></target>"
            },
            XmlTest::StrTest {
                xml: b"<root><not><child>value1</child></not><target><child>value2</child><target><child>value3</child></target></target><target><child>value4</child></target><target><child>value5</child></target></root>",
                endpoint: "/subtree",
                expected: "<target><child>value2</child><target><child></target>"
            },
            XmlTest::StrTest {
                xml: b"<root><target><child>value1</child></target><target><child>value2</child></target><target><child>value3</child></target><target><child>value4</child></target><target><child>value5</child></target><target><child>value6</child></target></root>",
                endpoint: "/subtree",
                expected: "<target><child>value1</child></target>"
            },
            XmlTest::StrTest {
                xml: b"<root><target><child>value1</child></target><target><child>value2</child></target><target><child>value3</child></target><target><child>value4</child></target><target><child>value5</child></target><target><child>value6</child></target><target><child>value7</child></target></root>",
                endpoint: "/subtree",
                expected: "<target><child>value1</child></target>"
            }
        ];
        for test in get_subtree_by_name_tests {
            get_subtree_by_name_test(test).await;
        }
    }


}

