use std::time::Duration;

use oauth2_mock::{Config, UserEntry, app, derived_id, issuer_path};
use openidconnect::core::{
    CoreAuthenticationFlow, CoreClient, CoreErrorResponseType, CoreIdTokenClaims,
    CoreProviderMetadata, CoreTokenResponse,
};
use openidconnect::reqwest::{self, header::LOCATION, redirect::Policy};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet, EndpointNotSet,
    EndpointSet, HttpClientError, IssuerUrl, Nonce, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, RequestTokenError, StandardErrorResponse, TokenResponse,
};
use tokio::net::TcpListener;
use url::Url;

const CLIENT_ID: &str = "curroya-local";
const CLIENT_SECRET: &str = "curroya-local-secret";
const REDIRECT_URI: &str = "http://localhost:8080/v1/oauth2/callback";
const EMAIL: &str = "candidate@curroya.es";
const CONFIGURED_EMAIL: &str = "owner@curroya.test";
const CONFIGURED_ID: &str = "6f8d3c1e-0b2a-4f5c-9d7e-1a2b3c4d5e6b";
const CONFIGURED_NAME: &str = "Venue Owner";
const TEST_KEY_SEED: &str = "test";
const TEST_TOKEN_LIFETIME: Duration = Duration::from_secs(60);
const ROOT_PATH: &str = "";

type DiscoveredClient = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

type ExchangeError = RequestTokenError<
    HttpClientError<reqwest::Error>,
    StandardErrorResponse<CoreErrorResponseType>,
>;

struct Provider {
    http: reqwest::Client,
    client: DiscoveredClient,
}

struct SignIn {
    authorization_url: Url,
    verifier: PkceCodeVerifier,
    nonce: Nonce,
    state: CsrfToken,
}

impl Provider {
    async fn start(path: &str) -> Self {
        let issuer = start_mock_provider(path).await;
        let http = reqwest::Client::builder()
            .redirect(Policy::none())
            .build()
            .unwrap();
        let client = discovered_client(&issuer, &http).await;
        Self { http, client }
    }

    fn begin_sign_in(&self) -> SignIn {
        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        let (authorization_url, state, nonce) = self
            .client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .set_pkce_challenge(challenge)
            .url();
        SignIn {
            authorization_url,
            verifier,
            nonce,
            state,
        }
    }

    async fn approve_with_login_hint(&self, sign_in: &SignIn, email: &str) -> String {
        let mut url = sign_in.authorization_url.clone();
        url.query_pairs_mut().append_pair("login_hint", email);
        let response = self.http.get(url).send().await.unwrap();
        let callback = callback_from(&response);

        assert_eq!(query_value(&callback, "state"), *sign_in.state.secret());
        query_value(&callback, "code")
    }

    async fn verified_claims(&self, code: &str, sign_in: SignIn) -> CoreIdTokenClaims {
        let response = self.exchange(code, sign_in.verifier).await.unwrap();
        let id_token = response.id_token().unwrap();
        id_token
            .claims(&self.client.id_token_verifier(), &sign_in.nonce)
            .unwrap()
            .clone()
    }

    async fn exchange(
        &self,
        code: &str,
        verifier: PkceCodeVerifier,
    ) -> Result<CoreTokenResponse, ExchangeError> {
        self.client
            .exchange_code(AuthorizationCode::new(code.to_owned()))
            .unwrap()
            .set_pkce_verifier(verifier)
            .request_async(&self.http)
            .await
    }
}

fn configured_user() -> UserEntry {
    UserEntry {
        email: CONFIGURED_EMAIL.to_owned(),
        name: Some(CONFIGURED_NAME.to_owned()),
        id: Some(CONFIGURED_ID.to_owned()),
    }
}

fn name_claim(claims: &CoreIdTokenClaims) -> String {
    let localized_name = claims.name().unwrap();
    localized_name.get(None).unwrap().to_string()
}

async fn start_mock_provider(path: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let issuer = format!("http://{}{path}", listener.local_addr().unwrap());
    let config = Config {
        port: 0,
        base_path: issuer_path(&issuer).unwrap(),
        browser_base_url: issuer.clone(),
        issuer: issuer.clone(),
        client_id: CLIENT_ID.to_owned(),
        client_secret: CLIENT_SECRET.to_owned(),
        key_seed: TEST_KEY_SEED.to_owned(),
        token_lifetime: TEST_TOKEN_LIFETIME,
        users: vec![configured_user()],
    };
    let router = app(config).unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    issuer
}

async fn discovered_client(issuer: &str, http: &reqwest::Client) -> DiscoveredClient {
    let issuer_url = IssuerUrl::new(issuer.to_owned()).unwrap();
    let metadata = CoreProviderMetadata::discover_async(issuer_url, http)
        .await
        .unwrap();
    CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(CLIENT_ID.to_owned()),
        Some(ClientSecret::new(CLIENT_SECRET.to_owned())),
    )
    .set_redirect_uri(RedirectUrl::new(REDIRECT_URI.to_owned()).unwrap())
}

fn callback_from(response: &reqwest::Response) -> Url {
    assert!(response.status().is_redirection());
    let location = response.headers()[LOCATION].to_str().unwrap();
    let callback = Url::parse(location).unwrap();
    assert!(callback.as_str().starts_with(REDIRECT_URI));
    callback
}

fn query_value(url: &Url, name: &str) -> String {
    let (_, parameter_value) = url.query_pairs().find(|(key, _)| key == name).unwrap();
    parameter_value.into_owned()
}

fn form_fields_for(sign_in: &SignIn, email: &str) -> Vec<(String, String)> {
    let mut fields: Vec<(String, String)> = sign_in
        .authorization_url
        .query_pairs()
        .into_owned()
        .collect();
    fields.push(("login_hint".to_owned(), email.to_owned()));
    fields
}

#[tokio::test]
async fn signs_in_with_pkce_under_an_issuer_path() {
    let provider = Provider::start("/dex").await;
    let sign_in = provider.begin_sign_in();
    let code = provider.approve_with_login_hint(&sign_in, EMAIL).await;

    let claims = provider.verified_claims(&code, sign_in).await;

    assert_eq!(claims.subject().as_str(), derived_id(EMAIL));
    assert_eq!(claims.email().unwrap().as_str(), EMAIL);
    assert_eq!(claims.email_verified(), Some(true));
}

#[tokio::test]
async fn refuses_a_wrong_verifier() {
    let provider = Provider::start(ROOT_PATH).await;
    let sign_in = provider.begin_sign_in();
    let code = provider.approve_with_login_hint(&sign_in, EMAIL).await;
    let (_, wrong_verifier) = PkceCodeChallenge::new_random_sha256();

    let result = provider.exchange(&code, wrong_verifier).await;

    let Err(RequestTokenError::ServerResponse(error)) = result else {
        panic!("expected the provider to refuse the exchange");
    };
    assert_eq!(*error.error(), CoreErrorResponseType::InvalidGrant);
}

#[tokio::test]
async fn refuses_a_code_spent_twice() {
    let provider = Provider::start(ROOT_PATH).await;
    let sign_in = provider.begin_sign_in();
    let code = provider.approve_with_login_hint(&sign_in, EMAIL).await;
    let same_verifier = PkceCodeVerifier::new(sign_in.verifier.secret().clone());

    provider.exchange(&code, sign_in.verifier).await.unwrap();
    let second_exchange = provider.exchange(&code, same_verifier).await;

    assert!(second_exchange.is_err());
}

#[tokio::test]
async fn signs_in_through_the_form() {
    let provider = Provider::start(ROOT_PATH).await;
    let sign_in = provider.begin_sign_in();
    let authorization_url = sign_in.authorization_url.clone();

    let page = provider
        .http
        .get(authorization_url.clone())
        .send()
        .await
        .unwrap();
    let page_html = page.text().await.unwrap();
    assert!(page_html.contains(r#"name="login_hint" aria-label="Email""#));
    assert!(page_html.contains(&format!(r#"value="{CONFIGURED_EMAIL}""#)));

    let fields = form_fields_for(&sign_in, EMAIL);
    let response = provider
        .http
        .post(authorization_url)
        .form(&fields)
        .send()
        .await
        .unwrap();
    let callback = callback_from(&response);

    assert!(!query_value(&callback, "code").is_empty());
}

#[tokio::test]
async fn signs_in_a_configured_user_with_its_id_and_name() {
    let provider = Provider::start(ROOT_PATH).await;
    let sign_in = provider.begin_sign_in();
    let code = provider
        .approve_with_login_hint(&sign_in, CONFIGURED_EMAIL)
        .await;

    let claims = provider.verified_claims(&code, sign_in).await;

    assert_eq!(claims.subject().as_str(), CONFIGURED_ID);
    assert_eq!(name_claim(&claims), CONFIGURED_NAME);
}
