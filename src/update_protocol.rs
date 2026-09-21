use std::io::{self, Read};

use ed25519_dalek::{Signature, VerifyingKey};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MAX_METADATA_BYTES: usize = 16 * 1024;
pub const MAX_INSTALLER_BYTES: u64 = 128 * 1024 * 1024;
pub const FEED_URL: &str =
    "https://github.com/mgelsinger/rivetnotes/releases/latest/download/update.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedRelease {
    pub payload: String,
    pub signature: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Release {
    pub version: String,
    pub platform: String,
    pub size: u64,
    pub sha256: String,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub fn stable_version(text: &str) -> io::Result<Version> {
    let version = Version::parse(text).map_err(|_| invalid("Invalid update version"))?;
    if !version.pre.is_empty() || !version.build.is_empty() || version.to_string() != text {
        return Err(invalid("Updates require a canonical stable version"));
    }
    Ok(version)
}

impl SignedRelease {
    pub fn verify(&self, public_key: &[u8; 32]) -> io::Result<Release> {
        if self.payload.len() > MAX_METADATA_BYTES || self.signature.len() != 128 {
            return Err(invalid("Invalid update metadata size"));
        }
        let key = VerifyingKey::from_bytes(public_key)
            .map_err(|_| invalid("Invalid update public key"))?;
        let signature =
            hex::decode(&self.signature).map_err(|_| invalid("Invalid update signature"))?;
        let signature =
            Signature::from_slice(&signature).map_err(|_| invalid("Invalid update signature"))?;
        key.verify_strict(self.payload.as_bytes(), &signature)
            .map_err(|_| invalid("Update signature verification failed"))?;
        let release: Release =
            serde_json::from_str(&self.payload).map_err(|_| invalid("Invalid update metadata"))?;
        release.validate()?;
        Ok(release)
    }
}

impl Release {
    pub fn validate(&self) -> io::Result<()> {
        stable_version(&self.version)?;
        if self.platform != "windows-x86_64" || self.size == 0 || self.size > MAX_INSTALLER_BYTES {
            return Err(invalid("Unsupported update platform or size"));
        }
        if self.sha256.len() != 64 || hex::decode(&self.sha256).is_err() {
            return Err(invalid("Invalid update digest"));
        }
        Ok(())
    }

    pub fn is_newer_than(&self, current: &str) -> io::Result<bool> {
        Ok(stable_version(&self.version)? > stable_version(current)?)
    }

    pub fn filename(&self) -> String {
        format!("rivet-{}-setup.exe", self.version)
    }

    pub fn download_url(&self) -> String {
        format!(
            "https://github.com/mgelsinger/rivetnotes/releases/download/v{}/{}",
            self.version,
            self.filename()
        )
    }

    pub fn verify_installer(&self, reader: &mut impl Read) -> io::Result<()> {
        let (size, hash) = digest(reader, self.size)?;
        if size != self.size || !hash.eq_ignore_ascii_case(&self.sha256) {
            return Err(invalid("Installer size or digest verification failed"));
        }
        Ok(())
    }
}

pub fn digest(reader: &mut impl Read, limit: u64) -> io::Result<(u64, String)> {
    let mut hash = Sha256::new();
    let mut size = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        size += count as u64;
        if size > limit {
            return Err(invalid("Download exceeds expected size"));
        }
        hash.update(&buffer[..count]);
    }
    Ok((size, hex::encode(hash.finalize())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn signed(version: &str) -> (SignedRelease, [u8; 32]) {
        let key = SigningKey::from_bytes(&[17; 32]);
        let release = Release {
            version: version.into(),
            platform: "windows-x86_64".into(),
            size: 3,
            sha256: hex::encode(Sha256::digest(b"abc")),
        };
        let payload = serde_json::to_string(&release).unwrap();
        let signature = hex::encode(key.sign(payload.as_bytes()).to_bytes());
        (
            SignedRelease { payload, signature },
            key.verifying_key().to_bytes(),
        )
    }

    #[test]
    fn accepts_authentic_update_and_rejects_tampering() {
        let (mut signed, key) = signed("0.5.0");
        let release = signed.verify(&key).unwrap();
        assert!(release.is_newer_than("0.4.22").unwrap());
        assert!(!release.is_newer_than("0.5.0").unwrap());
        assert!(!release.is_newer_than("1.0.0").unwrap());
        assert!(release.verify_installer(&mut &b"abc"[..]).is_ok());
        assert!(release.verify_installer(&mut &b"ab"[..]).is_err());
        assert!(release.verify_installer(&mut &b"abd"[..]).is_err());
        assert!(release.verify_installer(&mut &b"abcd"[..]).is_err());
        assert!(
            signed
                .verify(&SigningKey::from_bytes(&[18; 32]).verifying_key().to_bytes())
                .is_err()
        );
        signed.payload = signed.payload.replace("0.5.0", "9.0.0");
        assert!(signed.verify(&key).is_err());
    }

    #[test]
    fn rejects_prereleases_paths_and_invalid_platforms() {
        for version in ["0.5.0-beta", "0.5.0+build", "../setup", "v0.5.0", "01.2.3"] {
            let (signed, key) = signed(version);
            assert!(signed.verify(&key).is_err());
        }
        let (signed, key) = signed("0.5.0");
        let mut release = signed.verify(&key).unwrap();
        release.platform = "other".into();
        assert!(release.validate().is_err());
        release.platform = "windows-x86_64".into();
        release.size = MAX_INSTALLER_BYTES + 1;
        assert!(release.validate().is_err());
    }
}
