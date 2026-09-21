use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use ed25519_dalek::{Signer, SigningKey};
use rivet::update_protocol::{self, MAX_INSTALLER_BYTES, Release, SignedRelease};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command, manifest, installer] if command == "verify" => {
            let mut bytes = Vec::new();
            File::open(manifest)?.take((update_protocol::MAX_METADATA_BYTES + 1) as u64).read_to_end(&mut bytes)?;
            if bytes.len() > update_protocol::MAX_METADATA_BYTES { return Err("Update metadata is too large".into()); }
            let signed: SignedRelease = serde_json::from_slice(&bytes)?;
            let key: [u8; 32] = hex::decode(include_str!("../../assets/update-public-key.hex").trim())?.try_into().map_err(|_| "Invalid public key")?;
            let release = signed.verify(&key)?;
            release.verify_installer(&mut File::open(installer)?)?;
            println!("Verified update {} and installer digest", release.version);
        }
        [command, private, public] if command == "keygen" => {
            let mut seed = [0u8; 32];
            getrandom::getrandom(&mut seed).map_err(|e| format!("Key generation failed: {e}"))?;
            let key = SigningKey::from_bytes(&seed);
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(private)?;
            file.write_all(hex::encode(seed).as_bytes())?;
            file.sync_all()?;
            fs::write(
                public,
                format!("{}\n", hex::encode(key.verifying_key().to_bytes())),
            )?;
            println!("Signing key created. Only the public key belongs in the repository.");
        }
        [command, version, installer, output] if command == "sign" => {
            update_protocol::stable_version(version)?;
            let secret = std::env::var("RIVET_UPDATE_SIGNING_KEY")?;
            let seed: [u8; 32] = hex::decode(secret.trim())?
                .try_into()
                .map_err(|_| "Invalid signing key length")?;
            let key = SigningKey::from_bytes(&seed);
            let expected = fs::read_to_string("assets/update-public-key.hex")?;
            if expected.trim() != hex::encode(key.verifying_key().to_bytes()) {
                return Err("Signing key does not match the embedded public key".into());
            }
            let (size, sha256) =
                update_protocol::digest(&mut File::open(installer)?, MAX_INSTALLER_BYTES)?;
            let release = Release {
                version: version.clone(),
                platform: "windows-x86_64".into(),
                size,
                sha256,
            };
            release.validate()?;
            if Path::new(installer).file_name().and_then(|x| x.to_str())
                != Some(release.filename().as_str())
            {
                return Err("Installer name must match the release version".into());
            }
            let payload = serde_json::to_string(&release)?;
            let signature = hex::encode(key.sign(payload.as_bytes()).to_bytes());
            fs::write(
                output,
                serde_json::to_vec_pretty(&SignedRelease { payload, signature })?,
            )?;
            println!("Signed update metadata written to {output}");
        }
        _ => return Err(
            "Usage: update-release keygen PRIVATE_FILE PUBLIC_FILE | sign VERSION INSTALLER OUTPUT | verify MANIFEST INSTALLER"
                .into(),
        ),
    }
    Ok(())
}
