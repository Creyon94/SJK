//! Where the manifest and the files come from: the table of JoF's repository
//! hosts. Supporting another host is a new entry here, not new code.

/// One host of JoF's repository.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Source {
    /// The short name the player chooses it by (`jof.contentSource`) and logs show.
    pub name: &'static str,
    pub manifest: &'static str,
    /// The repository's root: a file is `<files>/<localfolder>/<name>`.
    pub files: &'static str,
}

/// JoF Cloud (`cloud.jof.app`, S3 behind CloudFront): an hourly byte-for-byte
/// copy of the origin's manifest and files, run by JoF to take the load off the
/// origin. JoF's launcher tries it first (`lib/models/repository_source.dart`,
/// 16/10/2026).
pub const MIRROR: Source = Source {
    name: "mirror",
    manifest: "https://cloud.jof.app/cloud/manifest/getclouddata.json",
    files: "https://cloud.jof.app/cloud/repository",
};

/// jofhub.eu, the repository itself: used when the mirror is down or has not
/// copied a new file yet.
pub const ORIGIN: Source = Source {
    name: "origin",
    manifest: "https://api.jofhub.eu/cloud/filemanager/getclouddata",
    files: "https://jofhub.eu/apps/cloud/repository",
};

/// Every known host, in the order `auto` tries them.
pub const SOURCES: &[Source] = &[MIRROR, ORIGIN];

/// The player's choice of host (`jof.contentSource`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SourceChoice {
    /// The mirror, then the origin.
    #[default]
    Auto,
    /// One host only.
    Only(Source),
}

impl SourceChoice {
    /// `auto`, or a host's name; anything else is `auto`.
    pub fn parse(value: &str) -> Self {
        SOURCES
            .iter()
            .find(|source| source.name.eq_ignore_ascii_case(value.trim()))
            .map_or(Self::Auto, |source| Self::Only(*source))
    }

    /// The hosts to try, in order.
    pub fn sources(self) -> Vec<Source> {
        match self {
            Self::Auto => SOURCES.to_vec(),
            Self::Only(source) => vec![source],
        }
    }
}

impl Source {
    /// The file's address, each path segment percent-encoded on its own, so
    /// names with spaces, braces, quotes or `&` survive.
    pub fn file_url(&self, asset: &crate::Asset) -> String {
        let mut url = self.files.to_owned();
        for segment in asset
            .repository_path
            .iter()
            .chain(std::iter::once(&asset.name))
        {
            url.push('/');
            encode_segment(segment, &mut url);
        }
        url
    }
}

fn encode_segment(segment: &str, out: &mut String) {
    for byte in segment.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_addresses_encode_each_segment() {
        let manifest = crate::Manifest::parse(crate::manifest::tests::SAMPLE.as_bytes()).unwrap();
        let asset = &manifest.group(2).unwrap().assets[0];
        assert_eq!(
            MIRROR.file_url(asset),
            "https://cloud.jof.app/cloud/repository/assets/%7BJoF%7D%27s-GWing.pk3"
        );
        assert_eq!(
            ORIGIN.file_url(&manifest.group(2).unwrap().assets[1]),
            "https://jofhub.eu/apps/cloud/repository/assets/maps%20two.pk3"
        );
    }

    #[test]
    fn the_choice_names_a_host_or_tries_them_all() {
        assert_eq!(SourceChoice::parse("auto").sources(), [MIRROR, ORIGIN]);
        assert_eq!(SourceChoice::parse("Origin").sources(), [ORIGIN]);
        assert_eq!(SourceChoice::parse("nonsense"), SourceChoice::Auto);
    }
}
