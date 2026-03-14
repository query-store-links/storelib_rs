use crate::error::StoreError;
use crate::models::fe3::{ApplicabilityBlob, PackageInstance};
use crate::utilities::helpers::string_to_package_type;
use log::{debug, trace, warn};

// ---------------------------------------------------------------------------
// Endpoint constants
// ---------------------------------------------------------------------------

const FE3_DELIVERY: &str = "https://fe3.delivery.mp.microsoft.com/ClientWebService/client.asmx";
const FE3_DELIVERY_SECURED: &str =
    "https://fe3.delivery.mp.microsoft.com/ClientWebService/client.asmx/secured";

// ---------------------------------------------------------------------------
// Embedded XML templates
// ---------------------------------------------------------------------------

const GET_COOKIE_XML: &str = include_str!("../xml/get_cookie.xml");
const WUID_REQUEST_XML: &str = include_str!("../xml/wuid_request.xml");
const FE3_FILE_URL_XML: &str = include_str!("../xml/fe3_file_url.xml");

// ---------------------------------------------------------------------------
// Default MSA device token (from original StoreLib C# source)
// ---------------------------------------------------------------------------

const MSA_TOKEN: &str = "<Device>dAA9AEUAdwBBAHcAQQBzAE4AMwBCAEEAQQBVADEAYgB5AHMAZQBtAGIAZQBEAFYAQwArADMAZgBtADcAbwBXAHkASAA3AGIAbgBnAEcAWQBtAEEAQQBMAGoAbQBqAFYAVQB2AFEAYwA0AEsAVwBFAC8AYwBDAEwANQBYAGUANABnAHYAWABkAGkAegBHAGwAZABjADEAZAAvAFcAeQAvAHgASgBQAG4AVwBRAGUAYwBtAHYAbwBjAGkAZwA5AGoAZABwAE4AawBIAG0AYQBzAHAAVABKAEwARAArAFAAYwBBAFgAbQAvAFQAcAA3AEgAagBzAEYANAA0AEgAdABsAC8AMQBtAHUAcgAwAFMAdQBtAG8AMABZAGEAdgBqAFIANwArADQAcABoAC8AcwA4ADEANgBFAFkANQBNAFIAbQBnAFIAQwA2ADMAQwBSAEoAQQBVAHYAZgBzADQAaQB2AHgAYwB5AEwAbAA2AHoAOABlAHgAMABrAFgAOQBPAHcAYQB0ADEAdQBwAFMAOAAxAEgANgA4AEEASABzAEoAegBnAFQAQQBMAG8AbgBBADIAWQBBAEEAQQBpAGcANQBJADMAUQAvAFYASABLAHcANABBAEIAcQA5AFMAcQBhADEAQgA4AGsAVQAxAGEAbwBLAEEAdQA0AHYAbABWAG4AdwBWADMAUQB6AHMATgBtAEQAaQBqAGgANQBkAEcAcgBpADgAQQBlAEUARQBWAEcAbQBXAGgASQBCAE0AUAAyAEQAVwA0ADMAZABWAGkARABUAHoAVQB0AHQARQBMAEgAaABSAGYAcgBhAGIAWgBsAHQAQQBUAEUATABmAHMARQBGAFUAYQBRAFMASgB4ADUAeQBRADgAagBaAEUAZQAyAHgANABCADMAMQB2AEIAMgBqAC8AUgBLAGEAWQAvAHEAeQB0AHoANwBUAHYAdAB3AHQAagBzADYAUQBYAEIAZQA4AHMAZwBJAG8AOQBiADUAQQBCADcAOAAxAHMANgAvAGQAUwBFAHgATgBEAEQAYQBRAHoAQQBYAFAAWABCAFkAdQBYAFEARQBzAE8AegA4AHQAcgBpAGUATQBiAEIAZQBUAFkAOQBiAG8AQgBOAE8AaQBVADcATgBSAEYAOQAzAG8AVgArAFYAQQBiAGgAcAAwAHAAUgBQAFMAZQBmAEcARwBPAHEAdwBTAGcANwA3AHMAaAA5AEoASABNAHAARABNAFMAbgBrAHEAcgAyAGYARgBpAEMAUABrAHcAVgBvAHgANgBuAG4AeABGAEQAbwBXAC8AYQAxAHQAYQBaAHcAegB5AGwATAAxADIAdwB1AGIAbQA1AHUAbQBwAHEAeQBXAGMASwBSAGoAeQBoADIASgBUAEYASgBXADUAZwBYAEUASQA1AHAAOAAwAEcAdQAyAG4AeABMAFIATgB3AGkAdwByADcAVwBNAFIAQQBWAEsARgBXAE0AZQBSAHoAbAA5AFUAcQBnAC8AcABYAC8AdgBlAEwAdwBTAGsAMgBTAFMASABmAGEASwA2AGoAYQBvAFkAdQBuAFIARwByADgAbQBiAEUAbwBIAGwARgA2AEoAQwBhAGEAVABCAFgAQgBjAHYAdQBlAEMASgBvADkAOABoAFIAQQByAEcAdwA0ACsAUABIAGUAVABiAE4AUwBFAFgAWAB6AHYAWgA2AHUAVwA1AEUAQQBmAGQAWgBtAFMAOAA4AFYASgBjAFoAYQBGAEsANwB4AHgAZwAwAHcAbwBuADcAaAAwAHgAQwA2AFoAQgAwAGMAWQBqAEwAcgAvAEcAZQBPAHoAOQBHADQAUQBVAEgAOQBFAGsAeQAwAGQAeQBGAC8AcgBlAFUAMQBJAHkAaQBhAHAAcABoAE8AUAA4AFMAMgB0ADQAQgByAFAAWgBYAFQAdgBDADAA\
UAA3AHoATwArAGYARwBrAHgAVgBtACsAVQBmAFoAYgBRADUANQBzAHcARQA9ACYAcAA9AA==</Device>";

// ---------------------------------------------------------------------------
// FE3 handler (all associated functions – no instance state required)
// ---------------------------------------------------------------------------

pub struct FE3Handler;

impl FE3Handler {
    // -----------------------------------------------------------------------
    // Cookie
    // -----------------------------------------------------------------------

    /// POST the GetCookie SOAP envelope to FE3 and return the `EncryptedData`
    /// value extracted from the response XML.
    pub async fn get_cookie(client: &reqwest::Client) -> Result<String, StoreError> {
        debug!("FE3: POST {FE3_DELIVERY} (GetCookie)");
        let response = client
            .post(FE3_DELIVERY)
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .body(GET_COOKIE_XML)
            .send()
            .await
            .map_err(StoreError::Http)?;

        let status = response.status();
        debug!("FE3 GetCookie response: HTTP {status}");

        let body = response.text().await.map_err(StoreError::Http)?;
        trace!("FE3 GetCookie body:\n{body}");

        let doc = roxmltree::Document::parse(&body).map_err(|e| StoreError::Xml(e.to_string()))?;

        let cookie = doc
            .descendants()
            .find(|n| n.tag_name().name() == "EncryptedData")
            .and_then(|n| n.text())
            .ok_or_else(|| {
                StoreError::Xml("EncryptedData node not found in cookie response".into())
            })?;

        debug!("FE3: cookie obtained ({} bytes)", cookie.len());
        Ok(cookie.to_owned())
    }

    // -----------------------------------------------------------------------
    // SyncUpdates
    // -----------------------------------------------------------------------

    /// Fetch a FE3 cookie, then POST a `SyncUpdates` request for the given
    /// `wu_category_id`.  Returns the HTML-decoded SOAP response body.
    pub async fn sync_updates(
        wu_category_id: &str,
        msa_token: Option<&str>,
        client: &reqwest::Client,
    ) -> Result<String, StoreError> {
        let cookie = Self::get_cookie(client).await?;
        let token = msa_token.unwrap_or(MSA_TOKEN);
        let body = WUID_REQUEST_XML
            .replace("{0}", &cookie)
            .replace("{1}", wu_category_id)
            .replace("{2}", token);

        debug!("FE3: POST {FE3_DELIVERY} (SyncUpdates, WuCategoryId={wu_category_id})");
        let response = client
            .post(FE3_DELIVERY)
            .header("Content-Type", "application/soap+xml; charset=utf-8")
            .body(body)
            .send()
            .await
            .map_err(StoreError::Http)?;

        let status = response.status();
        debug!("FE3 SyncUpdates response: HTTP {status}");

        let raw = response.text().await.map_err(StoreError::Http)?;
        let decoded = html_decode(&raw);
        trace!("FE3 SyncUpdates body:\n{decoded}");
        Ok(decoded)
    }

    // -----------------------------------------------------------------------
    // Process update IDs
    // -----------------------------------------------------------------------

    /// Parse the raw `SyncUpdates` XML and extract `(update_ids, revision_ids)`.
    ///
    /// Only nodes whose XML fragment contains a `SecuredFragment` child are
    /// included, matching the logic from the original C# code.
    pub fn process_update_ids(xml: &str) -> Result<(Vec<String>, Vec<String>), StoreError> {
        let doc = roxmltree::Document::parse(xml).map_err(|e| StoreError::Xml(e.to_string()))?;

        let mut update_ids = Vec::new();
        let mut revision_ids = Vec::new();

        for node in doc.descendants() {
            if node.tag_name().name() != "SecuredFragment" {
                continue;
            }

            // SecuredFragment -> parent (Properties) -> parent (Xml/Update element)
            // -> first_child (UpdateIdentity)
            let identity = node
                .parent()
                .and_then(|p| p.parent())
                .and_then(|gp| gp.first_element_child());

            if let Some(identity) = identity {
                if let (Some(uid), Some(rev)) = (
                    identity.attribute("UpdateID"),
                    identity.attribute("RevisionNumber"),
                ) {
                    debug!("FE3: update ID={uid} revision={rev}");
                    update_ids.push(uid.to_owned());
                    revision_ids.push(rev.to_owned());
                }
            } else {
                warn!("FE3: SecuredFragment node has unexpected parent structure; skipping");
            }
        }

        debug!("FE3: process_update_ids found {} ID(s)", update_ids.len());
        Ok((update_ids, revision_ids))
    }

    // -----------------------------------------------------------------------
    // Package instances
    // -----------------------------------------------------------------------

    /// Parse `AppxMetadata` nodes from the `SyncUpdates` XML and build
    /// [`PackageInstance`] values (without resolved download URLs).
    pub async fn get_package_instances(xml: &str) -> Result<Vec<PackageInstance>, StoreError> {
        let doc = roxmltree::Document::parse(xml).map_err(|e| StoreError::Xml(e.to_string()))?;

        let mut instances = Vec::new();

        for node in doc.descendants() {
            if node.tag_name().name() != "AppxMetadata" {
                continue;
            }

            // Must have at least 3 attributes (PackageMoniker, PackageType, ...)
            let attrs: Vec<_> = node.attributes().collect();
            if attrs.len() < 3 {
                continue;
            }

            let moniker = match node.attribute("PackageMoniker") {
                Some(v) => v.to_owned(),
                None => continue,
            };
            let pkg_type_str = node.attribute("PackageType").unwrap_or("");
            let pkg_type = string_to_package_type(pkg_type_str);

            debug!("FE3: package instance moniker={moniker} type={pkg_type_str}");

            // First child text node carries the ApplicabilityBlob JSON.
            let blob: Option<ApplicabilityBlob> =
                node.first_child().and_then(|c| c.text()).and_then(|t| {
                    trace!("FE3: ApplicabilityBlob JSON: {t}");
                    serde_json::from_str(t).ok()
                });

            instances.push(PackageInstance {
                package_moniker: moniker,
                package_uri: None,
                package_type: pkg_type,
                applicability_blob: blob,
                update_id: String::new(),
            });
        }

        Ok(instances)
    }

    // -----------------------------------------------------------------------
    // File URLs
    // -----------------------------------------------------------------------

    /// For each `(update_id, revision_id)` pair, POST a
    /// `GetExtendedUpdateInfo2` SOAP request to FE3 and collect the resulting
    /// file URLs (blockmap entries – always length 99 – are filtered out).
    pub async fn get_file_urls(
        update_ids: &[String],
        revision_ids: &[String],
        msa_token: Option<&str>,
        client: &reqwest::Client,
    ) -> Result<Vec<String>, StoreError> {
        let token = msa_token.unwrap_or(MSA_TOKEN);
        let mut urls = Vec::new();

        for (i, update_id) in update_ids.iter().enumerate() {
            let revision_id = match revision_ids.get(i) {
                Some(r) => r.as_str(),
                None => continue,
            };

            let body = FE3_FILE_URL_XML
                .replace("{0}", update_id)
                .replace("{1}", revision_id)
                .replace("{2}", token);

            debug!("FE3: POST {FE3_DELIVERY_SECURED} (GetExtendedUpdateInfo2, UpdateID={update_id} RevisionID={revision_id})");
            let response = client
                .post(FE3_DELIVERY_SECURED)
                .header("Content-Type", "application/soap+xml; charset=utf-8")
                .body(body)
                .send()
                .await
                .map_err(StoreError::Http)?;

            let status = response.status();
            debug!("FE3 GetExtendedUpdateInfo2 response: HTTP {status}");

            let raw = response.text().await.map_err(StoreError::Http)?;
            trace!("FE3 GetExtendedUpdateInfo2 body:\n{raw}");

            let doc = match roxmltree::Document::parse(&raw) {
                Ok(d) => d,
                Err(e) => return Err(StoreError::Xml(e.to_string())),
            };

            for file_loc in doc.descendants() {
                if file_loc.tag_name().name() != "FileLocation" {
                    continue;
                }
                for child in file_loc.children() {
                    if child.tag_name().name() == "Url" {
                        if let Some(text) = child.text() {
                            if text.len() != 99 {
                                debug!("FE3: URL resolved: {text}");
                                urls.push(text.to_owned());
                            } else {
                                trace!("FE3: skipping blockmap URL (len=99)");
                            }
                        }
                    }
                }
            }
        }

        Ok(urls)
    }
}

// ---------------------------------------------------------------------------
// HTML entity decoder
// ---------------------------------------------------------------------------

/// Minimal HTML entity decoder covering the entities used in SOAP responses.
pub(crate) fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_decode_all_entities() {
        assert_eq!(html_decode("a &amp; b"), "a & b");
        assert_eq!(html_decode("&lt;tag&gt;"), "<tag>");
        assert_eq!(html_decode("say &quot;hello&quot;"), "say \"hello\"");
        assert_eq!(html_decode("it&apos;s"), "it's");
    }

    #[test]
    fn html_decode_no_entities() {
        assert_eq!(html_decode("plain text"), "plain text");
    }

    #[test]
    fn html_decode_chained_entities() {
        // Replacements are sequential: &amp;lt; -> &lt; -> <
        assert_eq!(html_decode("&amp;lt;"), "<");
    }

    #[test]
    fn process_update_ids_empty_xml() {
        let xml = r#"<?xml version="1.0"?><root></root>"#;
        let (ids, revs) = FE3Handler::process_update_ids(xml).unwrap();
        assert!(ids.is_empty());
        assert!(revs.is_empty());
    }

    #[test]
    fn process_update_ids_parses_secured_fragment() {
        // Minimal SyncUpdates-style XML with one SecuredFragment node.
        let xml = r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
  <s:Body>
    <SyncUpdatesResult>
      <NewUpdates>
        <UpdateInfo>
          <ID>1</ID>
          <Xml>
            <UpdateIdentity UpdateID="abc-123" RevisionNumber="200"/>
            <Properties>
              <SecuredFragment/>
            </Properties>
          </Xml>
        </UpdateInfo>
      </NewUpdates>
    </SyncUpdatesResult>
  </s:Body>
</s:Envelope>"#;
        let (ids, revs) = FE3Handler::process_update_ids(xml).unwrap();
        assert_eq!(ids, vec!["abc-123"]);
        assert_eq!(revs, vec!["200"]);
    }

    #[test]
    fn process_update_ids_skips_nodes_without_secured_fragment() {
        let xml = r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
  <s:Body>
    <SyncUpdatesResult>
      <NewUpdates>
        <UpdateInfo>
          <ID>1</ID>
          <Xml>
            <UpdateIdentity UpdateID="no-fragment" RevisionNumber="1"/>
            <Properties/>
          </Xml>
        </UpdateInfo>
      </NewUpdates>
    </SyncUpdatesResult>
  </s:Body>
</s:Envelope>"#;
        let (ids, _) = FE3Handler::process_update_ids(xml).unwrap();
        assert!(ids.is_empty());
    }

    #[test]
    fn process_update_ids_multiple() {
        let xml = r#"<?xml version="1.0"?>
<root>
  <Update>
    <UpdateIdentity UpdateID="id-1" RevisionNumber="10"/>
    <Properties>
      <SecuredFragment/>
    </Properties>
  </Update>
  <Update>
    <UpdateIdentity UpdateID="id-2" RevisionNumber="20"/>
    <Properties>
      <SecuredFragment/>
    </Properties>
  </Update>
</root>"#;
        let (ids, revs) = FE3Handler::process_update_ids(xml).unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], "id-1");
        assert_eq!(revs[0], "10");
        assert_eq!(ids[1], "id-2");
        assert_eq!(revs[1], "20");
    }

    #[test]
    fn process_update_ids_invalid_xml_returns_error() {
        let result = FE3Handler::process_update_ids("not xml at all <<<");
        assert!(result.is_err());
    }
}
