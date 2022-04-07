use std::{collections::HashMap, path::Path};

use anyhow::*;
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::hash::sha256::{self, Digest};
use sodiumoxide::{
    crypto::{
        box_,
        box_::{PublicKey, SecretKey},
        sealedbox,
    },
    hex,
};

/// Environment variable name containing the secret key
pub const SECRET_KEY_ENV: &str = "AMBER_SECRET";

/// Current version of the file format
const FILE_FORMAT_VERSION: u32 = 2;

/// Raw version of [Config], the thing actually serialized/deserialized
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct ConfigRaw {
    /// Version of the file format represented here
    file_format_version: u32,

    /// Hex encoded public key
    public_key: String,

    /// Use a Vec instead of a HashMap to get guaranteed order in the output for
    /// minimal deltas
    secrets: Vec<SecretRaw>,
}

/// Raw version of [Secret], allowing for consistent ordering
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct SecretRaw {
    environment: String,
    name: String,
    sha256: String,
    cipher: String,
}

/// Config file
#[derive(Debug)]
pub struct Config {
    /// Public key in hex
    public_key: PublicKey,
    /// Encrypted secrets
    secrets: HashMap<String, Secret>,
}

/// The contents of an individual secret, still encrypted
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct Secret {
    /// Digest of the plaintext, to avoid unnecessary updates and minimize diffs
    sha256: Digest,
    /// Ciphertext encrypted with our public key
    cipher: Vec<u8>,
}

impl Config {
    /// Create a new keypair and config file
    pub fn new() -> (SecretKey, Self) {
        let (public, secret) = box_::gen_keypair();
        let config = Config {
            public_key: public,
            secrets: HashMap::new(),
        };
        (secret, config)
    }

    fn from_raw(raw: ConfigRaw) -> Result<Self> {
        ensure!(
            raw.file_format_version == FILE_FORMAT_VERSION,
            "Unsupported file format detected. Detected format is {}, we only support {}.",
            raw.file_format_version,
            FILE_FORMAT_VERSION
        );
        let public_key = hex::decode(&raw.public_key)
            .ok()
            .context("Public key is not hex")?;
        let public_key = PublicKey::from_slice(&public_key).context("Invalid public key")?;

        let mut secrets = HashMap::new();

        for pair in raw.secrets.into_iter().map(Secret::from_raw) {
            let (key, secret) = pair?;
            ensure!(
                !secrets.contains_key(&key),
                "Duplicated secret key: {}",
                key
            );
            let old = secrets.insert(key, secret);
            assert!(old.is_none());
        }
        Ok(Config {
            public_key,
            secrets,
        })
    }

    fn to_raw(&self) -> ConfigRaw {
        let mut secrets: Vec<SecretRaw> = self
            .secrets
            .iter()
            .map(|(key, value)| SecretRaw {
                name: key.clone(),
                sha256: hex::encode(&value.sha256),
                cipher: hex::encode(&value.cipher),
            })
            .collect();
        secrets.sort_unstable_by(|x, y| x.name.cmp(&y.name));
        ConfigRaw {
            file_format_version: FILE_FORMAT_VERSION,
            public_key: hex::encode(self.public_key),
            secrets,
        }
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let res: Result<Self> = (|| {
            let mut file = fs_err::File::open(path)?;
            let config = serde_yaml::from_reader(&mut file)?;
            Config::from_raw(config)
        })();
        res.with_context(|| format!("Unable to read file {}", path.display()))
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let res: Result<()> = (|| {
            let parent = path.parent().context("File must have a parent directory")?;
            fs_err::create_dir_all(parent).context("Unable to create parent directory")?;
            let mut file = fs_err::File::create(path)?;
            serde_yaml::to_writer(&mut file, &self.to_raw())?;
            Ok(())
        })();
        res.with_context(|| format!("Unable to write file {}", path.display()))
    }

    /// Encrypt a new value, replacing as necessary
    pub fn encrypt(&mut self, environment: String, key: String, value: &str) {
        let hash = sha256::hash(value.as_bytes());
        if let Some(old_secret) = self.secrets.get(&key) {
            if old_secret.sha256 == hash {
                log::info!("New value matches old value, doing nothing");
                return;
            } else {
                log::warn!("Overwriting old secret value");
            }
        }

        self.secrets.insert(
            environment,
            key,
            Secret {
                cipher: sealedbox::seal(value.as_bytes(), &self.public_key),
                sha256: hash,
            },
        );
    }

    /// Remove a value, if present
    pub fn remove(&mut self, key: &str) {
        if self.secrets.remove(key).is_none() {
            log::warn!("Asked to remove non-present secret {}, doing nothing", key);
        }
    }

    /// Get the secret key from the environment variable
    ///
    /// Validates that it matches up with the public key
    pub fn load_secret_key(&self) -> Result<SecretKey> {
        (|| {
            let hex = std::env::var(SECRET_KEY_ENV)?;
            let bs = hex::decode(&hex).ok().context("Invalid hex encoding")?;
            let secret = SecretKey::from_slice(&bs).context("Invalid secret key")?;
            ensure!(
                secret.public_key() == self.public_key,
                "Secret key does not match config file's public key"
            );
            Ok(secret)
        })()
        .with_context(|| {
            format!(
                "Error loading secret key from environment variable {}",
                SECRET_KEY_ENV
            )
        })
    }

    /// Iterate over the secrets
    pub fn iter_secrets<'a>(
        &'a self,
        secret_key: &'a SecretKey,
    ) -> impl Iterator<Item = Result<(&'a String, String)>> {
        self.secrets.iter().map(move |(key, secret)| {
            secret
                .decrypt(&self.public_key, secret_key, key)
                .map(|plain| (key, plain))
        })
    }

    /// Look up a specific secret value
    pub(crate) fn get_secret(&self, key: &str, secret_key: &SecretKey) -> Result<String> {
        self.secrets
            .get(key)
            .with_context(|| format!("Key does not exist: {}", key))
            .and_then(|secret| secret.decrypt(&self.public_key, secret_key, key))
    }
}

impl Secret {
    fn from_raw(raw: SecretRaw) -> Result<(String, Self)> {
        Ok((
            raw.name,
            Secret {
                sha256: Digest::from_slice(
                    &hex::decode(&raw.sha256).ok().context("Non-hex sha256")?,
                )
                .context("Invalid SHA256 digest")?,
                cipher: hex::decode(&raw.cipher)
                    .ok()
                    .context("Non-hex ciphertext")?,
            },
        ))
    }

    /// Decrypt this secret, key is used for error message displays only
    fn decrypt(&self, public_key: &PublicKey, secret_key: &SecretKey, key: &str) -> Result<String> {
        (|| {
            let plain = sealedbox::open(&self.cipher, public_key, secret_key)
                .ok()
                .context("Unable to decrypt secret")?;
            let digest = sha256::hash(&plain);
            ensure!(
                digest == self.sha256,
                "Hash mismatch, expected {}, received {}",
                hex::encode(self.sha256),
                hex::encode(digest)
            );
            String::from_utf8(plain).context("Invalid UTF-8 encoding")
        })()
        .with_context(|| format!("Error while decrypting secret named {}", key))
    }
}
