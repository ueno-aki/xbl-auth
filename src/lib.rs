use anyhow::{anyhow, Result};
use cache::{create_hash, get_msa_cache, get_xsts_cache, update_msa_cache, update_xsts_cache};
use chrono::DateTime;
use expire::ExpiringValue;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path, time::Duration};
use tokio::{
    fs,
    time::{sleep, Instant},
};

pub use reqwest::Client;
pub mod cache;
pub mod expire;

pub struct AuthOption<'a> {
    pub user_name: &'a str,
    pub cache_path: &'a Path,
}

pub async fn xbox_auth(client: Client, option: AuthOption<'_>) -> Result<ExpiringValue<XSTSToken>> {
    if !option.cache_path.exists() {
        fs::create_dir(option.cache_path).await?;
    }
    let user_hash = create_hash(option.user_name);
    if let Ok(xsts) = get_xsts_cache(option.cache_path, &user_hash).await {
        if !xsts.is_expired() {
            return Ok(xsts);
        }
    }
    let msa_responce = get_msa_token(client.clone(), &option, &user_hash).await?;
    let xbl = get_xbox_token(client.clone(), &msa_responce.get()?.access_token).await?;
    let xsts = get_xsts_token(client.clone(), &xbl.get()?.token).await?;
    update_xsts_cache(option.cache_path, &user_hash, &xsts).await?;
    Ok(xsts)
}

async fn get_msa_token(
    client: Client,
    option: &AuthOption<'_>,
    user_hash: &str,
) -> Result<ExpiringValue<MSATokenResponce>> {
    const SWITCH_CLIENT_ID: &str = "00000000441cc96b";
    const LIVE_DEVICE_CODE_REQUEST: &str = "https://login.live.com/oauth20_connect.srf";
    const LIVE_ACCESS_TOKEN_REQUEST: &str = "https://login.live.com/oauth20_token.srf";
    if let Ok(msa) = get_msa_cache(option.cache_path, user_hash).await {
        if !msa.is_expired() {
            return Ok(msa);
        }
    }
    let DeviceCodeResponse {
        user_code,
        device_code,
        verification_uri,
        interval,
        expires_in,
    } = client
        .post(LIVE_DEVICE_CODE_REQUEST)
        .form(&[
            ("scope", "service::user.auth.xboxlive.com::MBI_SSL"),
            ("client_id", SWITCH_CLIENT_ID),
            ("response_type", "device_code"),
        ])
        .send()
        .await?
        .json()
        .await?;

    println!(
        "To sign in as {}, use a web browser to open the page {verification_uri}?otc={user_code}",
        option.user_name
    );
    let expires_at = Instant::now() + Duration::from_secs(expires_in);
    let access_token: Result<MSATokenResponce> = loop {
        if Instant::now() > expires_at {
            break Err(anyhow!("Authentication failed, timed out."));
        }
        sleep(Duration::from_secs(interval)).await;
        let response = client
            .post(LIVE_ACCESS_TOKEN_REQUEST)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", &device_code),
                ("client_id", SWITCH_CLIENT_ID),
            ])
            .send()
            .await?;
        if let Ok(token) = response.json().await {
            break Ok(token);
        }
    };
    let msa = access_token.map(|token| {
        let expires_in = token.expires_in;
        ExpiringValue::with_duration_secs(token, expires_in)
    })?;
    update_msa_cache(option.cache_path, user_hash, &msa).await?;
    Ok(msa)
}

#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MSATokenResponce {
    pub token_type: String,
    pub scope: String,
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: String,
    pub expires_in: u64,
}

async fn get_xbox_token(
    client: Client,
    msa_access_token: &str,
) -> Result<ExpiringValue<XBLAuthResponse>> {
    const XBOX_USER_AUTH: &str = "https://user.auth.xboxlive.com/user/authenticate";
    let request_json = format!(
        r#"{{
        "Properties": {{
            "AuthMethod": "RPS",
            "SiteName": "user.auth.xboxlive.com",
            "RpsTicket": "t={}"
        }},
        "RelyingParty": "http://auth.xboxlive.com",
        "TokenType": "JWT"
    }}"#,
        msa_access_token
    );
    let xbl = client
        .post(XBOX_USER_AUTH)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("x-xbl-contract-version", "2")
        .body(request_json)
        .send()
        .await?
        .json::<XBLAuthResponse>()
        .await?;

    let expired_at = DateTime::parse_from_rfc3339(&xbl.not_after)?.timestamp() as u64;
    Ok(ExpiringValue::with_timestamp(xbl, expired_at))
}

async fn get_xsts_token(client: Client, xbl_token: &str) -> Result<ExpiringValue<XSTSToken>> {
    const XSTS_AUTH: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
    let request_json = format!(
        r#"{{
        "Properties": {{
            "SandboxId": "RETAIL",
            "UserTokens": ["{}"]
        }},
        "RelyingParty": "http://xboxlive.com",
        "TokenType": "JWT"
    }}"#,
        xbl_token
    );
    let XBLAuthResponse {
        token,
        display_claims,
        not_after,
        ..
    } = client
        .post(XSTS_AUTH)
        .header("Content-Type", "application/json")
        .header("x-xbl-contract-version", "1")
        .body(request_json)
        .send()
        .await?
        .json::<XBLAuthResponse>()
        .await?;
    let expired_at = DateTime::parse_from_rfc3339(&not_after)?.timestamp() as u64;
    Ok(ExpiringValue::with_timestamp(
        XSTSToken {
            user_hash: display_claims["xui"][0]["uhs"].clone(),
            xuid: display_claims["xui"][0]["xid"].clone(),
            xsts: token,
        },
        expired_at,
    ))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct XBLAuthResponse {
    pub issue_instant: String,
    pub not_after: String,
    pub token: String,
    pub display_claims: HashMap<String, [HashMap<String, String>; 1]>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct XSTSToken {
    pub user_hash: String,
    pub xuid: String,
    pub xsts: String,
}
