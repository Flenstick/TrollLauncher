//! Ely.by accounts: login, token refresh and skins through the Ely.by
//! authserver (a Yggdrasil-compatible API), plus the authlib-injector agent
//! that points the game at Ely.by for joining servers and loading skins.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use chrono::{Duration, Utc};
use reqwest::{StatusCode, Url};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::{
    Credentials, MinecraftCharacterExpressionState, MinecraftProfile,
    MinecraftSkin, MinecraftSkinVariant,
};
use crate::ErrorKind;
use crate::util::fetch::REQWEST_CLIENT;

const AUTHSERVER: &str = "https://authserver.ely.by";
const SKINSYSTEM: &str = "https://skinsystem.ely.by";
const AUTHLIB_INJECTOR_META: &str =
    "https://authlib-injector.yushi.moe/artifact/latest.json";

/// Error text the frontend looks for to ask the user for a 2FA code.
pub const TWO_FACTOR_REQUIRED: &str = "elyby:two-factor-required";

/// How long an Ely.by access token is trusted before it is refreshed.
const TOKEN_LIFETIME_HOURS: i64 = 12;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthResponse {
    access_token: String,
    client_token: String,
    selected_profile: Option<AuthProfile>,
}

#[derive(Deserialize)]
struct AuthProfile {
    id: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthError {
    #[serde(default)]
    error_message: String,
}

async fn post(
    path: &str,
    body: serde_json::Value,
) -> crate::Result<reqwest::Response> {
    Ok(REQWEST_CLIENT
        .post(format!("{AUTHSERVER}{path}"))
        .json(&body)
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await?)
}

async fn auth_error(response: reqwest::Response) -> crate::Error {
    let status = response.status();
    let error = response.json::<AuthError>().await.ok();
    let message = error
        .as_ref()
        .map(|e| e.error_message.to_lowercase())
        .unwrap_or_default();

    if message.contains("two factor") {
        return ErrorKind::OtherError(TWO_FACTOR_REQUIRED.to_owned()).into();
    }
    let text = if message.contains("invalid credentials") {
        "Ely.by: Неверный ник, e-mail или пароль.".to_owned()
    } else if message.contains("account protected")
        || message.contains("banned")
    {
        "Ely.by: Аккаунт заблокирован или защищён, войди на ely.by и проверь его.".to_owned()
    } else if message.contains("too many")
        || status == StatusCode::TOO_MANY_REQUESTS
    {
        "Ely.by: Слишком много попыток входа, подожди немного.".to_owned()
    } else {
        match error {
            Some(e) if !e.error_message.is_empty() => {
                format!("Ely.by: {}", e.error_message)
            }
            _ => format!("Ely.by вернул ошибку {status}"),
        }
    };
    ErrorKind::OtherError(text).into()
}

/// Signs in with an Ely.by username or e-mail and password, and stores the account.
#[tracing::instrument(skip(password, totp, pool))]
pub async fn login(
    username: &str,
    password: &str,
    totp: Option<&str>,
    pool: &sqlx::SqlitePool,
) -> crate::Result<Credentials> {
    let password = match totp.map(str::trim).filter(|code| !code.is_empty()) {
        Some(code) => format!("{password}:{code}"),
        None => password.to_owned(),
    };
    let response = post(
        "/auth/authenticate",
        json!({
            "username": username,
            "password": password,
            "clientToken": Uuid::new_v4().simple().to_string(),
            "requestUser": true,
        }),
    )
    .await?;

    if !response.status().is_success() {
        return Err(auth_error(response).await);
    }
    let auth = response.json::<AuthResponse>().await?;
    let profile = auth.selected_profile.ok_or_else(|| {
        ErrorKind::OtherError(
            "У аккаунта Ely.by нет игрового профиля".to_owned(),
        )
    })?;
    let id = Uuid::parse_str(&profile.id).map_err(|_| {
        ErrorKind::OtherError("Ely.by вернул неверный UUID".to_owned())
    })?;

    sqlx::query(
        "INSERT INTO elyby_accounts (uuid, client_token) VALUES ($1, $2)
         ON CONFLICT (uuid) DO UPDATE SET client_token = $2",
    )
    .bind(id.as_hyphenated().to_string())
    .bind(&auth.client_token)
    .execute(pool)
    .await?;

    let credentials = Credentials {
        offline_profile: MinecraftProfile {
            id,
            name: profile.name,
            ..MinecraftProfile::default()
        },
        access_token: auth.access_token,
        refresh_token: auth.client_token,
        expires: Utc::now() + Duration::hours(TOKEN_LIFETIME_HOURS),
        active: true,
        provider: AccountProvider::ElyBy,
    };
    credentials.upsert(pool).await?;
    Ok(credentials)
}

#[derive(
    serde::Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default,
)]
#[serde(rename_all = "snake_case")]
pub enum AccountProvider {
    #[default]
    Microsoft,
    #[serde(rename = "elyby")]
    ElyBy,
}

/// UUIDs of accounts that belong to Ely.by.
pub async fn elyby_account_ids(
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
) -> crate::Result<HashSet<Uuid>> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT uuid FROM elyby_accounts")
            .fetch_all(exec)
            .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(uuid,)| Uuid::parse_str(&uuid).ok())
        .collect())
}

pub async fn forget_account(
    uuid: Uuid,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
) -> crate::Result<()> {
    sqlx::query("DELETE FROM elyby_accounts WHERE uuid = $1")
        .bind(uuid.as_hyphenated().to_string())
        .execute(exec)
        .await?;
    Ok(())
}

// ───────────── OAuth2: вход через браузер ─────────────
//
// Приложение типа «Настольное приложение» на https://account.ely.by/dev/applications:
// публичный клиент, без секрета, с PKCE (S256); redirect — любой порт на 127.0.0.1 (RFC 8252).
// Client ID: константа ELYBY_CLIENT_ID ниже, переменная окружения ELYBY_CLIENT_ID при сборке
// или запуске, либо `elyby-oauth.json` в папке лаунчера ({"client_id": "..."}).
// Для старых «веб»-приложений можно указать client_secret и redirect_uri.
// У таких аккаунтов в `refresh_token` лежит `oauth:<refresh token>`.

const OAUTH_AUTHORIZE: &str = "https://account.ely.by/oauth2/v1";
const OAUTH_TOKEN: &str = "https://account.ely.by/api/oauth2/v1/token";
const OAUTH_INFO: &str = "https://account.ely.by/api/account/v1/info";
const OAUTH_SCOPE: &str = "account_info minecraft_server_session offline_access";
const OAUTH_PREFIX: &str = "oauth:";
/// Client ID настольного приложения TrollLauncher на Ely.by (публичный, не секрет).
const ELYBY_CLIENT_ID: &str = "trolllauncher";

#[derive(Deserialize, Clone, Debug)]
pub struct OAuthClient {
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    /// Задан только для «веб»-приложений; у настольного redirect выбирается сам.
    #[serde(default)]
    pub redirect_uri: Option<String>,
}

/// Ключи OAuth-приложения Ely.by, если они настроены.
pub fn oauth_client() -> Option<OAuthClient> {
    if let Some(dirs) = crate::state::DirectoryInfo::global_handle_if_ready()
        && let Ok(bytes) = std::fs::read(dirs.settings_dir.join("elyby-oauth.json"))
        && let Ok(c) = serde_json::from_slice::<OAuthClient>(&bytes)
        && !c.client_id.is_empty()
    {
        return Some(c);
    }
    let id = std::env::var("ELYBY_CLIENT_ID")
        .ok()
        .or(option_env!("ELYBY_CLIENT_ID").map(String::from))
        .or(Some(ELYBY_CLIENT_ID.to_owned()))
        .filter(|id| !id.is_empty())?;
    let secret = std::env::var("ELYBY_CLIENT_SECRET").ok().or(option_env!("ELYBY_CLIENT_SECRET").map(String::from)).unwrap_or_default();
    Some(OAuthClient { client_id: id, client_secret: secret, redirect_uri: std::env::var("ELYBY_REDIRECT_URI").ok() })
}

/// Начатый вход: ссылка для браузера + то, что нужно, чтобы его завершить.
#[derive(Clone, Debug)]
pub struct OAuthStart {
    pub url: String,
    pub state: String,
    pub redirect_uri: String,
    pub verifier: String,
}

/// PKCE: случайный verifier и S256-challenge (RFC 7636).
fn pkce() -> (String, String) {
    use base64::Engine;
    use rand::RngCore;
    let mut raw = [0u8; 48];
    rand::thread_rng().fill_bytes(&mut raw);
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let verifier = b64.encode(raw);
    let challenge = b64.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

/// `redirect_uri` — адрес, который слушает лаунчер (например http://127.0.0.1:PORT/ely).
pub fn oauth_start(client: &OAuthClient, redirect_uri: &str) -> OAuthStart {
    let (verifier, challenge) = pkce();
    let state = Uuid::new_v4().simple().to_string();
    let mut url = Url::parse(OAUTH_AUTHORIZE).expect("valid url");
    url.query_pairs_mut()
        .append_pair("client_id", &client.client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", OAUTH_SCOPE)
        .append_pair("state", &state)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("description", "TrollLauncher");
    OAuthStart { url: url.to_string(), state, redirect_uri: redirect_uri.to_owned(), verifier }
}

#[derive(Deserialize)]
struct OAuthToken {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

#[derive(Deserialize)]
struct OAuthInfo {
    uuid: String,
    username: String,
}

async fn oauth_token(client: &OAuthClient, form: &[(&str, &str)]) -> crate::Result<OAuthToken> {
    let mut form: Vec<(&str, &str)> = form.to_vec();
    form.push(("client_id", client.client_id.as_str()));
    if !client.client_secret.is_empty() {
        form.push(("client_secret", client.client_secret.as_str()));
    }
    let response = REQWEST_CLIENT
        .post(OAUTH_TOKEN)
        .form(&form)
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await?;
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(ErrorKind::OtherError(format!("Ely.by OAuth: {status} {text}")).into());
    }
    Ok(response.json::<OAuthToken>().await?)
}

/// Меняет код из браузера на токены и сохраняет аккаунт.
#[tracing::instrument(skip_all)]
pub async fn login_oauth(code: &str, start: &OAuthStart, pool: &sqlx::SqlitePool) -> crate::Result<Credentials> {
    let client = oauth_client().ok_or_else(|| ErrorKind::OtherError("Ely.by OAuth не настроен".to_owned()))?;
    let token = oauth_token(
        &client,
        &[
            ("redirect_uri", start.redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code),
            ("code_verifier", start.verifier.as_str()),
        ],
    )
    .await?;
    let refresh = token.refresh_token.clone().ok_or_else(|| ErrorKind::OtherError("Ely.by не выдал refresh token".to_owned()))?;
    let info = REQWEST_CLIENT
        .get(OAUTH_INFO)
        .bearer_auth(&token.access_token)
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await?
        .error_for_status()?
        .json::<OAuthInfo>()
        .await?;
    let id = Uuid::parse_str(&info.uuid).map_err(|_| ErrorKind::OtherError("Ely.by вернул неверный UUID".to_owned()))?;
    let stored = format!("{OAUTH_PREFIX}{refresh}");
    sqlx::query(
        "INSERT INTO elyby_accounts (uuid, client_token) VALUES ($1, $2)
         ON CONFLICT (uuid) DO UPDATE SET client_token = $2",
    )
    .bind(id.as_hyphenated().to_string())
    .bind(&stored)
    .execute(pool)
    .await?;
    let credentials = Credentials {
        offline_profile: MinecraftProfile { id, name: info.username, ..MinecraftProfile::default() },
        access_token: token.access_token,
        refresh_token: stored,
        expires: Utc::now() + Duration::seconds(token.expires_in.unwrap_or(86_400).clamp(600, 86_400 * 7) - 300),
        active: true,
        provider: AccountProvider::ElyBy,
    };
    credentials.upsert(pool).await?;
    Ok(credentials)
}

async fn refresh_oauth(credentials: &mut Credentials, refresh: &str) -> crate::Result<()> {
    let client = oauth_client().ok_or_else(|| ErrorKind::OtherError("Ely.by OAuth не настроен — войди в аккаунт заново".to_owned()))?;
    let token = oauth_token(&client, &[("scope", OAUTH_SCOPE), ("refresh_token", refresh), ("grant_type", "refresh_token")]).await?;
    credentials.access_token = token.access_token;
    credentials.expires = Utc::now() + Duration::seconds(token.expires_in.unwrap_or(86_400).clamp(600, 86_400 * 7) - 300);
    Ok(())
}

/// Refreshes an Ely.by access token; the client token is kept in `refresh_token`.
pub async fn refresh(credentials: &mut Credentials) -> crate::Result<()> {
    if let Some(r) = credentials.refresh_token.strip_prefix(OAUTH_PREFIX).map(str::to_owned) {
        return refresh_oauth(credentials, &r).await;
    }
    let response = post(
        "/auth/refresh",
        json!({
            "accessToken": credentials.access_token,
            "clientToken": credentials.refresh_token,
            "requestUser": false,
        }),
    )
    .await?;
    if !response.status().is_success() {
        return Err(auth_error(response).await);
    }
    let auth = response.json::<AuthResponse>().await?;
    credentials.access_token = auth.access_token;
    credentials.refresh_token = auth.client_token;
    credentials.expires = Utc::now() + Duration::hours(TOKEN_LIFETIME_HOURS);
    Ok(())
}

/// Revokes the access token on Ely.by; failures are ignored because the account is removed anyway.
pub async fn invalidate(credentials: &Credentials) {
    if credentials.refresh_token.starts_with(OAUTH_PREFIX) {
        return;
    }
    let _ = post(
        "/auth/invalidate",
        json!({
            "accessToken": credentials.access_token,
            "clientToken": credentials.refresh_token,
        }),
    )
    .await;
}

#[derive(Deserialize)]
struct Textures {
    #[serde(rename = "SKIN")]
    skin: Option<Texture>,
}

#[derive(Deserialize)]
struct Texture {
    url: String,
    #[serde(default)]
    metadata: Option<TextureMetadata>,
}

#[derive(Deserialize)]
struct TextureMetadata {
    #[serde(default)]
    model: Option<String>,
}

/// Builds a profile with the player's current Ely.by skin.
pub async fn profile(
    base: &MinecraftProfile,
) -> crate::Result<MinecraftProfile> {
    let response = REQWEST_CLIENT
        .get(format!("{SKINSYSTEM}/textures/{}", base.name))
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;

    let mut profile = MinecraftProfile {
        id: base.id,
        name: base.name.clone(),
        fetch_time: Some(Instant::now()),
        ..MinecraftProfile::default()
    };

    if response.status() == StatusCode::OK
        && let Ok(textures) = response.json::<Textures>().await
        && let Some(skin) = textures.skin
        && let Ok(url) =
            Url::parse(&skin.url.replacen("http://", "https://", 1))
    {
        let slim = skin
            .metadata
            .and_then(|m| m.model)
            .is_some_and(|model| model == "slim");
        profile.skins.push(MinecraftSkin {
            id: {
                let digest = Sha256::digest(url.as_str().as_bytes());
                let mut bytes = [0u8; 16];
                bytes.copy_from_slice(&digest[..16]);
                Uuid::from_bytes(bytes)
            },
            state: MinecraftCharacterExpressionState::Active,
            texture_key: None,
            url: Arc::new(url),
            variant: if slim {
                MinecraftSkinVariant::Slim
            } else {
                MinecraftSkinVariant::Classic
            },
            name: None,
        });
    }

    Ok(profile)
}

#[derive(Deserialize)]
struct InjectorArtifact {
    download_url: String,
    checksums: InjectorChecksums,
}

#[derive(Deserialize)]
struct InjectorChecksums {
    sha256: String,
}

static INJECTOR_LOCK: Mutex<()> = Mutex::const_new(());

/// Downloads (once) and returns the path to authlib-injector, verified by SHA-256.
pub async fn authlib_injector(
    libraries_dir: PathBuf,
) -> crate::Result<PathBuf> {
    let _guard = INJECTOR_LOCK.lock().await;
    let dir = libraries_dir.join("authlib-injector");
    let jar = dir.join("authlib-injector.jar");

    let artifact = match REQWEST_CLIENT
        .get(AUTHLIB_INJECTOR_META)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
    {
        Ok(response) => response.json::<InjectorArtifact>().await.ok(),
        Err(e) => {
            tracing::warn!("Could not check authlib-injector version: {e}");
            None
        }
    };

    let Some(artifact) = artifact else {
        return if jar.exists() {
            Ok(jar)
        } else {
            Err(ErrorKind::OtherError(
                "Не удалось скачать authlib-injector для входа через Ely.by"
                    .to_owned(),
            )
            .into())
        };
    };

    if let Ok(existing) = tokio::fs::read(&jar).await
        && format!("{:x}", Sha256::digest(&existing))
            == artifact.checksums.sha256
    {
        return Ok(jar);
    }

    let bytes = REQWEST_CLIENT
        .get(&artifact.download_url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    if format!("{:x}", Sha256::digest(&bytes)) != artifact.checksums.sha256 {
        return Err(ErrorKind::OtherError(
            "Контрольная сумма authlib-injector не совпала".to_owned(),
        )
        .into());
    }
    tokio::fs::create_dir_all(&dir).await?;
    tokio::fs::write(&jar, &bytes).await?;
    Ok(jar)
}
