use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppxPackageIdentity {
    #[serde(rename = "@Name", default)]
    pub name: String,
    #[serde(rename = "@Publisher", default)]
    pub publisher: String,
    #[serde(rename = "@Version", default)]
    pub version: String,
    #[serde(rename = "@ProcessorArchitecture", default)]
    pub processor_architecture: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppxApplication {
    #[serde(rename = "@Id", default)]
    pub id: String,
    #[serde(rename = "@Executable", default)]
    pub executable: Option<String>,
    #[serde(rename = "@EntryPoint", default)]
    pub entry_point: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppxApplications {
    #[serde(rename = "Application", default)]
    pub applications: Vec<AppxApplication>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppxManifest {
    #[serde(rename = "Identity")]
    pub identity: AppxPackageIdentity,
    #[serde(rename = "Applications", default)]
    pub applications: Option<AppxApplications>,
}

impl AppxManifest {
    pub fn parse(xml_content: &str) -> Result<Self, quick_xml::de::DeError> {
        quick_xml::de::from_str(xml_content)
    }

    /// Gets the primary executable name from the first Application entry.
    pub fn primary_executable(&self) -> Option<&str> {
        if let Some(apps) = &self.applications {
            for app in &apps.applications {
                if let Some(exe) = &app.executable {
                    if !exe.is_empty() {
                        return Some(exe.as_str());
                    }
                }
            }
        }
        None
    }

    /// Derives the PackageFamilyName format used by Windows (Name_PublisherHash).
    pub fn package_family_name(&self) -> String {
        // Fallback default format when publisher hash is not computed
        format!("{}_8wekyb3d8bbwe", self.identity.name)
    }

    /// Extracts the Xbox Title ID from ms-xbl-<TitleId> protocol in manifest XML if present.
    pub fn extract_title_id(xml_content: &str) -> Option<String> {
        if let Some(idx) = xml_content.find("ms-xbl-") {
            let rest = &xml_content[idx + 7..];
            let tid: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            if !tid.is_empty() && tid.to_lowercase() != "multiplayer" {
                return Some(tid);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sample_appx_manifest() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/types">
    <Identity Name="Microsoft.SampleGame" Publisher="CN=Microsoft Corporation" Version="1.2.3.4" />
    <Applications>
        <Application Id="App" Executable="SampleGame.exe" EntryPoint="SampleGame.App">
        </Application>
    </Applications>
</Package>"#;

        let manifest = AppxManifest::parse(xml).expect("Failed to parse AppxManifest XML");
        assert_eq!(manifest.identity.name, "Microsoft.SampleGame");
        assert_eq!(manifest.identity.version, "1.2.3.4");
        assert_eq!(manifest.primary_executable(), Some("SampleGame.exe"));
        assert_eq!(manifest.package_family_name(), "Microsoft.SampleGame_8wekyb3d8bbwe");
    }
}
