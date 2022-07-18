# covid19-scraping-rust

## 使用方法

```
covid19-scraping-rust [OPTIONS] --mode <MODE>

OPTIONS:
        --auth-url <AUTH_URL>
            OAuth2方式でログインする際の認証URLを指定します

        --client-id <CLIENT_ID>
            OAuth2方式でログインする際のクライアントIDを指定します

        --client-secret <CLIENT_SECRET>
            OAuth2方式でログインする際のクライアントシークレットを指定します

        --fetch-size <FETCH_SIZE>
            一度にメールを取得する際の最大メッセージ数を指定します [default: 4]

        --file-path <FILE_PATH>
            ローカルモードで使用するファイル名を指定します

    -h, --help
            Print help information

        --login-type <LOGIN_TYPE>
            IMAPサーバへのログインの方式を指定します [possible values: oauth2, password]

        --mode <MODE>
            動作モードを指定します [possible values: remote, local]

        --password <PASSWORD>
            パスワード認証方式でログインする際のパスワードを指定します

        --port <PORT>
            IMAPサーバのポート番号を指定します [default: 993]

        --query <QUERY>
            メールボックスを検索する際のクエリ文字列を指定します

        --refresh-token <REFRESH_TOKEN>
            OAuth2方式でログインする際のリフレッシュトークンを指定します

        --server <SERVER>
            IMAPサーバのアドレスを指定します

        --token-url <TOKEN_URL>
            OAuth2方式でログインする際のトークンURLを指定します

        --user <USER>
            IMAPサーバにログインする際のユーザ名を指定します

    -V, --version
            Print version information
```

## ライセンス

本ソフトウェアは、MIT Licenseでライセンスされています。条文は[こちら](LICENSE)です。

また、ソフトウェアの開発に使用したライブラリとそのライセンスは、[こちら](docs/credits.md)で紹介しています。
