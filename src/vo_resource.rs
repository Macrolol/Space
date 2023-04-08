use quick_xml::de::Deserializer;
use serde_derive::Deserialize;
use quick_xml::events::{Event, BytesStart};
use quick_xml::reader::Reader;



impl AccessUrl {
    pub fn new(url: Url) -> Self {
        Self(url)
    }       
}

type Url = String;
/*
    AccessURL element as described in
    http://www.ivoa.net/xml/VOResource/v1.0

    The URL (or base URL) that a client uses to access the
    service.  How this URL is to be interpreted and used
    depends on the specific Interface subclass
*/
#[derive(Debug, Deserialize)]
pub struct AccessUrl(Url);

/*
    A URL available as a mirror of an access URL.

    These come with a human-readable title intended to aid in mirror
    selection.
 */
type Title = String;
#[derive(Debug, Deserialize)]
pub struct MirrorUrl(Url, Title);
#[derive(Debug, Deserialize)]

pub struct SecurityMethod {
    pub standardid: String,
    pub role: String,
}

/*    Interface element as described in
    http://www.ivoa.net/xml/VOResource/v1.0

    A description of a service interface.

    Since this type is abstract, one must use an Interface subclass to describe
    an actual interface.

    Additional interface subtypes (beyond WebService and WebBrowser) are
    defined in the VODataService schema.
*/
#[derive(Debug, Deserialize)]

pub struct IterfaceAccess {
    pub access_url: AccessUrl,
    pub mirror_urls: Vec<MirrorUrl>,
    pub security_methods: Vec<SecurityMethod>,
}

#[derive(Debug, Deserialize)]
pub enum Interface {
    /*    
    WebService element as described in
    http://www.ivoa.net/xml/VOResource/v1.0

    A Web Service that is describable by a WSDL document.
    The accessURL element gives the Web Service's endpoint URL. 
    */
    WebService{
        access: IterfaceAccess,
        wsdl: Url,
    },
    /*
    WebBrowser element as described in
    http://www.ivoa.net/xml/VOResource/v1.0
    */
    WebBrowser{
        access: IterfaceAccess,
    }
}



/* 
    ValidationLevel element as described in
    http://www.ivoa.net/xml/VOResource/v1.0

    the allowed values for describing the resource descriptions and interfaces.
    See the RM (v1.1, section 4) for more guidance on the use of these values.

    Possible values:

    0:
        The resource has a description that is stored in a
        registry. This level does not imply a compliant
        description.
    1:
        In addition to meeting the level 0 definition, the
        resource description conforms syntactically to this
        standard and to the encoding scheme used.
    2:
        In addition to meeting the level 1 definition, the
        resource description refers to an existing resource that
        has demonstrated to be functionally compliant.

        When the resource is a service, it is consider to exist
        and functionally compliant if use of the
        service accessURL responds without error when used as
        intended by the resource. If the service is a standard
        one, it must also demonstrate the response is syntactically
        compliant with the service standard in order to be
        considered functionally compliant. If the resource is
        not a service, then the ReferenceURL must be shown to
        return a document without error.
    3:
        In addition to meeting the level 2 definition, the
        resource description has been inspected by a human and
        judged to comply semantically to this standard as well
        as meeting any additional minimum quality criteria (e.g.,
        providing values for important but non-required
        metadata) set by the human inspector.
    4:
        In addition to meeting the level 3 definition, the
        resource description meets additional quality criteria
        set by the human inspector and is therefore considered
        an excellent description of the resource. Consequently,
        the resource is expected to be operate well as part of a
        VO application or research study.
         */
#[derive(Debug, Deserialize)]
pub struct ValidationLevel {
    pub level: u8,
}

/*
    Capability element as described in
    http://www.ivoa.net/xml/VOResource/v1.0

    a description of what the service does
    (in terms of context-specific behavior), and how to use it
    (in terms of an interface)
 */
#[derive(Debug, Deserialize)]

pub struct Capability {
    pub interface: Interface,
    pub validation_level: ValidationLevel,
}


/*
    Resource element as described in
    http://www.ivoa.net/xml/VOResource/v1.0

    This will hold all the information about a resource.
*/
#[derive(Debug, Deserialize)]

pub struct Resource {
    pub identifier: String,
    pub title: String,
    pub description: String,
}


pub enum ResourceError {
    InvalidUrl,
    InvalidXml,
    InvalidResource,
}


type TapQuery = String;

#[derive(Debug, Deserialize)]
pub enum SearchParameter {
    Identifier(String),
    Title(String),
    Description(String),
    TapQuery(TapQuery),
}

/*
    A service is one particular collection of resources
    hosted by a single entity.
*/
pub trait Service {
    fn search_resources(&self) -> Vec<Resource>;
    fn get_capabilities(&self) -> Vec<Capability>;
}



#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Read;
    use std::io::Write;
    use std::path::Path;
    use quick_xml::Reader;
    use schema_analysis::analysis::InferredSchema;
    use serde_json;



    #[test]
    fn test_infer_schema() {
        let schema_path = include_str!("../VoResourceSchema.xsd");
        let schema: InferredSchema = quick_xml::de::from_str(schema_path).unwrap();
        /*{
            schema_analysis::helpers::xml::cleanup_xml_schema(&mut schema.schema);
        }*/
        let schema = serde_json::to_string_pretty(&schema.schema).unwrap();
        
        let output_path = Path::new("tests/resources/ivoa.json");
        match output_path.parent() {
            Some(parent) => std::fs::create_dir_all(parent).unwrap(),
            None => (),
        };

        let mut file = File::create(output_path).unwrap();
        file.write_all(schema.as_bytes()).unwrap();
    }

    #[test]
    fn test_deserialize_resource() {
        let path = Path::new("tests/resources/ivoa.xml");
        let file = File::open(path).unwrap();
        let mut reader = BufReader::new(file);
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();
        let mut de = Deserializer::from_reader(buf.as_slice());
        let resource: Resource = Resource::deserialize(&mut de).unwrap();
        println!("{:?}", resource);
    }
}