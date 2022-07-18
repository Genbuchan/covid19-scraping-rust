use super::{app::AppMode, mail_client::LoginMethod};
use clap::Parser;

#[derive(Parser)]
#[clap(author, version, about)]
pub struct Args {
    #[clap(
        long,
        value_parser,
        help = "OAuth2方式でログインする際の認証URLを指定します"
    )]
    pub auth_url: Option<String>,
    #[clap(
        long,
        value_parser,
        help = "OAuth2方式でログインする際のクライアントIDを指定します"
    )]
    pub client_id: Option<String>,
    #[clap(
        long,
        value_parser,
        help = "OAuth2方式でログインする際のクライアントシークレットを指定します"
    )]
    pub client_secret: Option<String>,
    #[clap(
        long,
        value_parser,
        help = "一度にメールを取得する際の最大メッセージ数を指定します",
        default_value_t = 4
    )]
    pub fetch_size: usize,
    #[clap(
        long,
        value_parser,
        help = "ローカルモードで使用するファイル名を指定します"
    )]
    pub file_path: Option<String>,
    #[clap(long, value_enum, help = "IMAPサーバへのログインの方式を指定します")]
    pub login_type: Option<LoginMethod>,
    #[clap(long, value_enum, help = "動作モードを指定します")]
    pub mode: AppMode,
    #[clap(
        long,
        value_parser,
        help = "パスワード認証方式でログインする際のパスワードを指定します"
    )]
    pub password: Option<String>,
    #[clap(
        long,
        value_parser,
        help = "IMAPサーバのポート番号を指定します",
        default_value_t = 993
    )]
    pub port: u16,
    #[clap(
        long,
        value_parser,
        help = "OAuth2方式でログインする際のリフレッシュトークンを指定します"
    )]
    pub refresh_token: Option<String>,
    #[clap(long, value_parser, help = "IMAPサーバのアドレスを指定します")]
    pub server: Option<String>,
    #[clap(
        long,
        value_parser,
        help = "OAuth2方式でログインする際のトークンURLを指定します"
    )]
    pub token_url: Option<String>,
    #[clap(
        long,
        value_parser,
        help = "メールボックスを検索する際のクエリ文字列を指定します"
    )]
    pub query: Option<String>,
    #[clap(
        long,
        value_parser,
        help = "IMAPサーバにログインする際のユーザ名を指定します"
    )]
    pub user: Option<String>,
}
