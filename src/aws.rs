use std::str::FromStr;

use anyhow::*;
use rusoto_core::Region;
use rusoto_secretsmanager::{CreateSecretRequest, GetSecretValueRequest, SecretsManager};
use sodiumoxide::crypto::box_::{PublicKey, SecretKey};
use sodiumoxide::hex;

fn get_client(region: &str) -> Result<rusoto_secretsmanager::SecretsManagerClient> {
    let region =
        Region::from_str(region).with_context(|| format!("Invalid AWS region: {}", region))?;
    Ok(rusoto_secretsmanager::SecretsManagerClient::new(region))
}

#[tokio::main]
/// Does not guarantee public and secret keys match
pub async fn load(region: &str, public: &PublicKey) -> Result<SecretKey> {
    log::debug!("Loading a secret key from AWS Secrets Manager");
    let client = get_client(region)?;
    let req = GetSecretValueRequest {
        secret_id: format!("amber-{}", hex::encode(public)),
        version_id: None,
        version_stage: None,
    };
    let res = client
        .get_secret_value(req)
        .await
        .context("Unable to load secret key from AWS Secrets Manager")?;
    let encoded = res
        .secret_string
        .context("AWS response missing secret_string")?;
    let binary = hex::decode(&encoded)
        .ok()
        .context("AWS secret_string is not hex encoded")?;

    // Don't bother confirming they match, that's handled by our caller
    SecretKey::from_slice(&binary).context("AWS secret_string is not a valid secret key")
}

#[tokio::main]
pub async fn save(region: &str, public: &PublicKey, secret: &SecretKey) -> Result<()> {
    log::debug!("Saving a secret key into AWS Secrets Manager");
    let client = get_client(region)?;
    let req = CreateSecretRequest {
        add_replica_regions: None,
        client_request_token: Some(hex::encode(public)),
        description: Some("Amber secret key".to_owned()),
        force_overwrite_replica_secret: None,
        kms_key_id: None,
        name: format!("amber-{}", hex::encode(public)),
        secret_binary: None,
        secret_string: Some(hex::encode(secret)),
        tags: None,
    };
    let res = client.create_secret(req).await?;
    eprintln!(
        "Added new secret to AWS named {} with ARN {}",
        res.name.context("No friendly name returned from AWS")?,
        res.arn.context("No ARN returned from AWS")?,
    );
    Ok(())
}
