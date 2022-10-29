use std::convert::TryInto;
use std::{collections::HashMap, path::Path};

use anyhow::*;
use crypto_box::rand_core::OsRng;
use crypto_box::{seal, seal_open, PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use sha2::Sha256;

/// Environment variable name containing the secret key
pub const SECRET_KEY_ENV: &str = "AMBER_SECRET";

/// Current version of the file format
const FILE_FORMAT_VERSION: u32 = 1;

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
    sha256: [u8; 32],
    /// Ciphertext encrypted with our public key
    cipher: Vec<u8>,
}

impl Config {
    /// Create a new keypair and config file
    pub fn new() -> (SecretKey, Self) {
        let secret_key = SecretKey::generate(&mut OsRng);
        let config = Config {
            public_key: secret_key.public_key(),
            secrets: HashMap::new(),
        };
        (secret_key, config)
    }

    fn from_raw(raw: ConfigRaw) -> Result<Self> {
        ensure!(
            raw.file_format_version == FILE_FORMAT_VERSION,
            "Unsupported file format detected. Detected format is {}, we only support {}.",
            raw.file_format_version,
            FILE_FORMAT_VERSION
        );
        let public_key: [u8; 32] = hex::decode(&raw.public_key)
            .ok()
            .context("Public key is not hex")?
            .try_into()
            .map_err(|_| anyhow!("Invalid Public key"))?;

        let public_key = PublicKey::from(public_key);

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
                sha256: hex::encode(value.sha256),
                cipher: hex::encode(&value.cipher),
            })
            .collect();
        secrets.sort_unstable_by(|x, y| x.name.cmp(&y.name));
        ConfigRaw {
            file_format_version: FILE_FORMAT_VERSION,
            public_key: hex::encode(&self.public_key),
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
    pub fn encrypt(&mut self, key: String, value: &str) -> Result<()> {
        let mut hasher = Sha256::new();
        hasher.update(value);
        let hash = hasher.finalize_reset().into();
        if let Some(old_secret) = self.secrets.get(&key) {
            if old_secret.sha256 == hash {
                log::info!("New value matches old value, doing nothing");
                return Ok(());
            } else {
                log::warn!("Overwriting old secret value");
            }
        }

        let cipher = seal(&mut OsRng, &self.public_key, value.as_bytes())
            .map_err(|_| anyhow!("Error during encryption"))?;

        self.secrets.insert(
            key,
            Secret {
                cipher,
                sha256: hash,
            },
        );
        Ok(())
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
            let bs: [u8; 32] = hex::decode(&hex)
                .ok()
                .context("Invalid hex encoding")?
                .try_into()
                .map_err(|_| anyhow!("Invalid secret key"))?;
            let secret: SecretKey = SecretKey::from(bs);
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
                .decrypt(secret_key, key)
                .map(|plain| (key, plain))
        })
    }

    /// Look up a specific secret value
    pub(crate) fn get_secret(&self, key: &str, secret_key: &SecretKey) -> Result<String> {
        self.secrets
            .get(key)
            .with_context(|| format!("Key does not exist: {}", key))
            .and_then(|secret| secret.decrypt(secret_key, key))
    }
}

impl Secret {
    fn from_raw(raw: SecretRaw) -> Result<(String, Self)> {
        let digest: [u8; 32] = hex::decode(&raw.sha256)
            .ok()
            .context("Non-hex sha256")?
            .try_into()
            .map_err(|_| anyhow!("Error parsing into digest"))?;
        Ok((
            raw.name,
            Secret {
                sha256: digest,
                // sha256: hasher.finalize_reset().into(),
                cipher: hex::decode(&raw.cipher)
                    .ok()
                    .context("Non-hex ciphertext")?,
            },
        ))
    }

    /// Decrypt this secret, key is used for error message displays only
    fn decrypt(&self, secret_key: &SecretKey, key: &str) -> Result<String> {
        (|| {
            let plain = seal_open(secret_key, &self.cipher[..])
                .map_err(|_| anyhow!("Unable to decrypt secret"))?;
            let mut hasher = Sha256::new();
            hasher.update(&plain);
            let digest: [u8; 32] = hasher.finalize_reset().into();
            ensure!(
                digest == self.sha256,
                "Hash mismatch, expected {}, received {}",
                hex::encode(self.sha256),
                hex::encode(digest)
            );
            String::from_utf8(plain.to_vec()).context("Invalid UTF-8 encoding")
        })()
        .with_context(|| format!("Error while decrypting secret named {}", key))
    }
}
