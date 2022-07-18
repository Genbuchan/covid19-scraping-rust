use std::{
    error::Error,
    fmt::{self, Display},
    net::TcpStream,
};

use clap::ValueEnum;
use imap::{Client, Session};
use native_tls::{TlsConnector, TlsStream};
use oauth2::{
    basic::BasicClient, reqwest::http_client, AuthUrl, ClientId, ClientSecret, RefreshToken,
    TokenResponse, TokenUrl,
};

#[derive(Debug)]
pub struct MailClient {
    client: Option<Client<TlsStream<TcpStream>>>,
    pub login_method: LoginMethod,
    pub oauth2: Option<OAuth2>,
    pub password: Option<Password>,
    pub server: Server,
    tls: TlsConnector,
}

/// MailClientのエラーを列挙しています。
#[derive(Debug)]
pub enum MailClientError {
    /// IMAPサーバとの認証に失敗した際のエラーです。
    EstablishmentError,
    FailedToGetTokenError,
}

/// メールサーバへのログインの方式を列挙しています。
#[derive(Clone, Debug, ValueEnum)]
pub enum LoginMethod {
    #[clap(name = "oauth2")]
    OAuth2,
    Password,
}

/// OAuth2方式のアカウントの情報を格納する構造体です。
#[derive(Debug)]
pub struct OAuth2 {
    pub user: String,
    pub access_token: String,
}

/// パスワード認証方式のアカウントの情報を格納する構造体です。
#[derive(Clone, Debug)]
pub struct Password {
    pub user: String,
    pub password: String,
}

#[derive(Debug)]
pub struct Server {
    pub address: String,
    pub port: u16,
}

impl Display for MailClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MailClientError::EstablishmentError => write!(f, "EstablishmentError"),
            MailClientError::FailedToGetTokenError => write!(f, "FailedToGetTokenError"),
        }
    }
}

impl Error for MailClientError {}

impl MailClient {
    /// IMAPサーバとのセッションを確立するメソッドです。
    pub fn establish_imap_session(self) -> Result<Session<TlsStream<TcpStream>>, MailClientError> {
        match self.login_method {
            LoginMethod::OAuth2 => {
                return match self
                    .client
                    .unwrap()
                    .authenticate("XOAUTH2", &self.oauth2.unwrap())
                {
                    Ok(s) => Ok(s),
                    Err(_) => Err(MailClientError::EstablishmentError),
                }
            }
            LoginMethod::Password => {
                let username = self.password.clone().unwrap().user;
                let password = self.password.clone().unwrap().password;
                return match self.client.unwrap().login(username, password) {
                    Ok(s) => Ok(s),
                    Err(_) => Err(MailClientError::EstablishmentError),
                };
            }
        }
    }

    pub fn new(method: LoginMethod, address: String, port: u16) -> Self {
        return MailClient {
            client: None,
            login_method: method,
            oauth2: None,
            password: None,
            server: Server {
                address: address,
                port: port,
            },
            tls: native_tls::TlsConnector::builder().build().unwrap(),
        };
    }

    pub fn set_oauth2_account(
        &mut self,
        auth_url: String,
        client_id: String,
        client_secret: String,
        refresh_token: String,
        token_url: String,
        username: String,
    ) -> Result<(), MailClientError> {
        let oauth2_client = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new(auth_url).unwrap(),
            Some(TokenUrl::new(token_url).unwrap()),
        );

        let token_result = match oauth2_client
            .exchange_refresh_token(&RefreshToken::new(refresh_token))
            .request(http_client)
        {
            Ok(c) => c,
            Err(_) => return Err(MailClientError::FailedToGetTokenError),
        };
        self.client = Some(
            imap::connect(
                (self.server.address.clone(), self.server.port),
                self.server.address.clone(),
                &self.tls,
            )
            .unwrap(),
        );

        self.oauth2 = Some(OAuth2 {
            user: username,
            access_token: token_result.access_token().secret().to_owned(),
        });

        return Ok(());
    }

    pub fn set_password_account(&mut self, password: String, username: String) {
        self.password = Some(Password {
            password: password,
            user: username,
        })
    }
}

impl imap::Authenticator for OAuth2 {
    type Response = String;
    fn process(&self, _: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}
