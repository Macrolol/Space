pub mod vo_resource;
pub (crate) mod registry;
pub (crate) mod xml;

use xml::{XmlDocument};
use quick_xml::reader::Reader;

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