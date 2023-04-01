extern crate reqwest;
use std::boxed::Box;
use quick_xml::reader::Reader;
use quick_xml::events::Event;
use quick_xml::name::QName;

const REGTAP_URL: &str = "http://reg.g-vo.org/tap";
const REGTAP_CAPABILITIES_ENDPOINT: &str = "capabilities";

pub struct RegTapCapability {
    
}

pub struct RegTapService {
    pub url: String,
    pub client: reqwest::Client,
}

impl RegTapService {
    pub fn new(url: &str) -> Self {
        let client = reqwest::Client::new();
        RegTapService {
            url: url.to_string(),
            client,
        }
    }
    
    pub async fn get_capabilities(&self) -> Result<String, Box<dyn std::error::Error>> {
        let response = self.client.get(format!("{}/{}", &self.url, &REGTAP_CAPABILITIES_ENDPOINT).as_str())
            .send()
            .await?
            .text()
            .await?;
        Ok(response)
    }
}

impl Default for RegTapService {
    fn default() -> Self {
        RegTapService::new(REGTAP_URL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_regtap() {
        let regtap = RegTapService::default();
        let res = regtap.get_capabilities().await.unwrap();
        println!("{:?}", res);
    }
}