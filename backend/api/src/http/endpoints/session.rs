use actix_web::{
    cookie::Cookie,
    web::{Data, Json, Path},
    HttpRequest, HttpResponse,
};
use chrono::{Duration, Utc};
use config::RemoteTarget;
use core::settings::{GoogleOAuth, RuntimeSettings};
use paperclip::actix::{api_v2_operation, web::ServiceConfig};
use shared::{
    api::{endpoints::session, ApiEndpoint},
    domain::session::{
        CreateSessionOAuthRequest, CreateSessionResponse, GetOAuthUrlResponse,
        GetOAuthUrlServiceKind, NewSessionResponse, OAuthUrlKind,
    },
};
use sqlx::PgPool;
use url::Url;

use crate::{
    db,
    domain::RegistrationStatus,
    error::{self, ServiceKind},
    extractor::EmailBasicUser,
    google::{self, oauth_url},
    jwk,
    token::{create_auth_token, SessionMask},
};

#[api_v2_operation]
async fn get_oauth_url(
    req: HttpRequest,
    config: Data<RuntimeSettings>,
    Path((service_kind, url_kind)): Path<(GetOAuthUrlServiceKind, OAuthUrlKind)>,
) -> Result<Json<GetOAuthUrlResponse>, error::Service> {
    match service_kind {
        GetOAuthUrlServiceKind::Google => {}
        it => return Err(anyhow::anyhow!("Unsupported OAuth service kind: {:?}", it).into()),
    }

    let oauth_config = config
        .google_oauth
        .as_ref()
        .ok_or(error::Service::DisabledService(ServiceKind::GoogleOAuth))?;

    let route = oauth_url(config.remote_target(), url_kind);

    let mut url: Url = req.url_for_static("google_cloud_oauth")?;

    // todo: add / verify `state`

    let url = url
        .query_pairs_mut()
        .append_pair("client_id", &oauth_config.client)
        .append_pair("response_type", "code")
        .append_pair("include_granted_scopes", "true")
        .append_pair("redirect_uri", &route)
        .append_pair("scope", "openid email")
        .finish()
        .to_string();

    Ok(Json(GetOAuthUrlResponse { url }))
}

/// Login with basic authorization.
/// May return resources for *signing up* if the user doesn't have a profile.
#[api_v2_operation]
async fn create_session(
    db: Data<PgPool>,
    settings: Data<RuntimeSettings>,
    user: EmailBasicUser,
) -> Result<HttpResponse, error::Server> {
    let login_ttl = settings
        .login_token_valid_duration
        .unwrap_or(Duration::weeks(2));

    let (mask, valid_until) = match user.registration_status {
        RegistrationStatus::New => panic!("This isn't currently possible"),
        RegistrationStatus::Validated => (
            SessionMask::PUT_PROFILE | SessionMask::DELETE_ACCOUNT,
            Utc::now() + Duration::hours(1),
        ),
        RegistrationStatus::Complete => (SessionMask::GENERAL, Utc::now() + login_ttl),
    };

    let mut txn = db.begin().await?;

    let session = db::session::create(&mut txn, user.id, Some(&valid_until), mask, None).await?;

    let (csrf, cookie) = create_auth_token(
        &settings.token_secret,
        settings.is_local(),
        login_ttl,
        &session,
    )?;

    txn.commit().await?;

    let response = NewSessionResponse { csrf };

    let response = if !mask.contains(SessionMask::GENERAL) {
        CreateSessionResponse::Register(response)
    } else {
        CreateSessionResponse::Login(response)
    };

    Ok(HttpResponse::Created().cookie(cookie).json(response))
}

#[api_v2_operation]
/// Login with OAuth
/// May return resources for *signing up* if the user doesn't exist.
async fn create_oauth_session(
    db: Data<PgPool>,
    settings: Data<RuntimeSettings>,
    req: Json<CreateSessionOAuthRequest>,
    jwks: Data<jwk::JwkVerifier>,
) -> Result<HttpResponse, error::OAuth> {
    let (response, cookie) = match req.into_inner() {
        CreateSessionOAuthRequest::Google {
            code,
            redirect_kind,
        } => {
            let config = settings
                .google_oauth
                .as_ref()
                .ok_or(error::GoogleOAuth::Disabled)?;

            handle_google_oauth(
                &db,
                &config,
                &settings.token_secret,
                settings.is_local(),
                &jwks,
                &code,
                settings.login_token_valid_duration,
                settings.remote_target(),
                redirect_kind,
            )
            .await?
        }

        other => return Err(anyhow::anyhow!("Unsupported OAuth request kind: {:?}", other).into()),
    };

    Ok(HttpResponse::Created().cookie(cookie).json(response))
}

async fn handle_google_oauth(
    db: &PgPool,
    config: &GoogleOAuth,
    token_secret: &[u8; 32],
    local_insecure: bool,
    jwks: &jwk::JwkVerifier,
    code: &str,
    login_token_valid_duration: Option<Duration>,
    remote_target: RemoteTarget,
    redirect_kind: OAuthUrlKind,
) -> Result<(CreateSessionResponse, Cookie<'static>), error::OAuth> {
    let redirect_url = google::oauth_url(remote_target, redirect_kind);

    let tokens = google::convert_oauth_code(config, code, &redirect_url).await?;

    let claims = jwks.verify_oauth(&tokens.id_token, 3).await?;

    let mut txn = db.begin().await?;

    let google_auth = sqlx::query!(
        "select user_id from user_auth_google where google_id = $1",
        &claims.google_id
    )
    .fetch_optional(&mut txn)
    .await?;

    let (user_id, mask) = match &google_auth {
        Some(google_auth) => {
            // make sure that the user either has a profile, or can only *create* one.
            let check_profile = sqlx::query!(
                r#"select exists(select 1 from user_profile where user_id = $1) as "exists!""#,
                google_auth.user_id
            )
            .fetch_one(&mut txn)
            .await?;

            let mask = if check_profile.exists {
                SessionMask::GENERAL
            } else {
                SessionMask::PUT_PROFILE | SessionMask::DELETE_ACCOUNT
            };

            (google_auth.user_id, mask)
        }
        None => {
            if !claims.email_verified {
                return Err(error::OAuth::Google(error::GoogleOAuth::UnverifiedEmail));
            }

            let id = sqlx::query!(r#"insert into "user" default values returning id"#)
                .fetch_one(&mut txn)
                .await?
                .id;

            sqlx::query!(
                r"insert into user_auth_google (user_id, google_id) values ($1, $2)",
                id,
                &claims.google_id
            )
            .execute(&mut txn)
            .await?;

            // todo: handle duplicate email here (return a conflict error)

            sqlx::query!(
                "insert into user_email (user_id, email) values ($1, $2::text)",
                id,
                &claims.email
            )
            .execute(&mut txn)
            .await?;
            (id, SessionMask::PUT_PROFILE)
        }
    };

    let login_ttl = login_token_valid_duration.unwrap_or(Duration::weeks(2));

    let valid_until = Utc::now()
        + if mask.contains(SessionMask::PUT_PROFILE) {
            Duration::hours(1)
        } else {
            login_ttl
        };

    let session = db::session::create(&mut txn, user_id, Some(&valid_until), mask, None).await?;

    txn.commit().await?;

    let (csrf, cookie) = create_auth_token(token_secret, local_insecure, login_ttl, &session)?;

    let response = NewSessionResponse { csrf };

    let response = if !mask.contains(SessionMask::GENERAL) {
        CreateSessionResponse::Register(response)
    } else {
        CreateSessionResponse::Login(response)
    };

    Ok((response, cookie))
}

pub fn configure(cfg: &mut ServiceConfig<'_>) {
    cfg.route(
        session::GetOAuthUrl::PATH,
        session::GetOAuthUrl::METHOD.route().to(get_oauth_url),
    )
    .route(
        session::Create::PATH,
        session::Create::METHOD.route().to(create_session),
    )
    .route(
        session::CreateOAuth::PATH,
        session::CreateOAuth::METHOD
            .route()
            .to(create_oauth_session),
    );
}
