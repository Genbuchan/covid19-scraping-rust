use core::panic;
use std::{
    fs::File,
    io::{Read, Write}, path::Path,
};

use calamine::{open_workbook, DataType, Range, Reader, Xlsx};
use chrono::{DateTime, Duration, Local, TimeZone, Utc};
use clap::{Parser, ValueEnum};
use data_formats::structs::{
    last_update::LastUpdate,
    news::{NewsItem, NewsItems},
    status::{Attributes, Status},
    summary::{Summary, SummaryContent},
};
use imap::types::Fetch;
use mail_parser::{Message, MimeHeaders};
use regex::Regex;

use crate::structs::{
    args::Args,
    mail_client::{LoginMethod, MailClient},
};

/// メールサーバへのログインの方式を列挙しています。
#[derive(Clone, Debug, ValueEnum)]
pub enum AppMode {
    Remote,
    Local,
}

pub struct App;

const EXTRACT_DIR: &'static str = &"./data/";
const FILES_NAME: &'static [&'static str] = &[
    "inspections_summary.json",
    "last_update.json",
    "main_summary.json",
    "news.json",
    "patients_summary.json",
];
const TMP_DIR: &'static str = &"./tmp/";
const WORKSHEETS_NAME: &'static [&'static str] = &["日毎の陽性者数", "PCR検査件数", "最新の情報"];

impl App {
    pub fn new() -> Self {
        return App {};
    }

    pub fn run(&self) {
        // プロセスに渡された引数をパース
        let args = Args::parse();

        // データの最終更新日時を把握するため、./data/last_update.jsonの取得を試みる
        let mut last_update: Option<LastUpdate<Local>> =
            match File::open([EXTRACT_DIR, "last_update.json"].concat()) {
                Ok(mut f) => {
                    let mut buffer = String::new();
                    match f.read_to_string(&mut buffer) {
                        Ok(_) => match serde_json::from_str::<LastUpdate<Local>>(&buffer) {
                            Ok(l) => Some(l),
                            Err(_) => {
                                println!("last_update.jsonのデータフォーマットに誤りがあります。");
                                None
                            }
                        },
                        Err(_) => {
                            println!("last_update.jsonはテキストデータでない可能性があります。");
                            None
                        }
                    }
                }
                Err(_) => {
                    println!("last_update.jsonが存在しません。");
                    None
                }
            };

        // スプレッドシートのファイルパスを格納する変数を用意
        let mut spreadsheet_path: Option<String> = None;

        match args.mode {
            // IMAPサーバからデータを取得するモードの処理
            AppMode::Remote => {
                // 一時ディレクトリを作成
                if !Path::new(TMP_DIR).is_dir() {
                    println!("一時ディレクトリを作成します。");
                    std::fs::create_dir(TMP_DIR).unwrap();
                }

                let login_method = args
                    .login_type
                    .expect("リモートからのデータの取得には、login_typeの指定が必要です。");

                // タイムゾーンを取得
                let timezone_offset = Local::now().offset().local_minus_utc();
                // 添付ファイル名のマッチングの用意
                let regex = Regex::new("[0-9]{8}data.xlsx").unwrap();
                // IMAPクライアントを作成
                let mut client = MailClient::new(
                    login_method.clone(),
                    args.server.unwrap_or_else(|| {
                        panic!("IMAPサーバへのログインには、サーバアドレスの指定が必要です。")
                    }),
                    args.port,
                );

                let user = args.user.unwrap_or_else(|| {
                    panic!("IMAPサーバへのログインには、ユーザ名の指定が必要です。")
                });

                match &login_method {
                    LoginMethod::OAuth2 => {
                        client
                    .set_oauth2_account(
                        args.auth_url.unwrap_or_else(|| {
                            panic!("IMAPサーバへのログインには、認証URLの指定が必要です。")
                        }),
                        args.client_id.unwrap_or_else(|| {
                            panic!("IMAPサーバへのログインには、クライアントIDの指定が必要です。")
                        }),
                        args.client_secret.unwrap_or_else(|| {
                            panic!("IMAPサーバへのログインには、クライアントシークレットの指定が必要です。")
                        }),
                        args.refresh_token.unwrap_or_else(|| {
                            panic!("IMAPサーバへのログインには、リフレッシュトークンの指定が必要です。")
                        }),
                        args.token_url.unwrap_or_else(|| {
                            panic!("IMAPサーバへのログインには、トークンURLの指定が必要です。")
                        }),
                        user,
                    )
                    .unwrap();
                    }
                    LoginMethod::Password => client.set_password_account(
                        args.password.unwrap_or_else(|| {
                            panic!("IMAPサーバへのログインには、パスワードの指定が必要です。")
                        }),
                        user,
                    ),
                }

                println!("IMAPサーバとのセッションの確立を試みます。");
                let mut session = client.establish_imap_session().unwrap_or_else(|e| {
                    panic!("メールサーバとのセッションの確立に失敗しました。: {}", e)
                });

                println!("IMAPサーバからメールを取得します。");
                // 受信ボックスを選択
                session.select("INBOX").unwrap();
                // クエリで指定した条件に合致するメールのインデックスを列挙
                let mut indexes = session
                    .search(args.query.unwrap())
                    .unwrap()
                    .into_iter()
                    .collect::<Vec<u32>>();

                // 値が大きい順にソート
                indexes.sort();
                indexes.reverse();
                // メッセージのインデックスを、引数で指定された数で分割する
                let chunks: Vec<&[u32]> = indexes.chunks(args.fetch_size).collect();

                // チャンクを取り出す
                for chunk in chunks {
                    // IMAPサーバからメールをフェッチする
                    let result = session
                        .fetch(
                            chunk
                                .iter()
                                .map(|x| x.to_string())
                                .collect::<Vec<String>>()
                                .join(","),
                            "RFC822",
                        )
                        .unwrap();

                    // メールのインデックス順でソート
                    let fetched = {
                        let mut fetched: Vec<&Fetch> =
                            result.iter().map(|m| m.to_owned()).collect::<Vec<&Fetch>>();
                        fetched.sort_by(|a, b| a.message.cmp(&b.message));
                        fetched.reverse();
                        fetched
                    };

                    // メッセージの内容を読み取る
                    for raw_message in fetched {
                        // メッセージをパース
                        let message = Message::parse(raw_message.body().unwrap()).unwrap();
                        // メッセージから日付・時刻を取得
                        let parsed_datetime = message.get_date().unwrap();
                        // chronoのDateTime<Local>型に変換
                        let mut datetime = Local
                            .ymd(
                                parsed_datetime.year.try_into().unwrap(),
                                parsed_datetime.month,
                                parsed_datetime.day,
                            )
                            .and_hms(
                                parsed_datetime.hour,
                                parsed_datetime.minute,
                                parsed_datetime.second,
                            );
                        {
                            let hour: i64 = parsed_datetime.tz_hour.try_into().unwrap();
                            let minute: i64 = parsed_datetime.tz_minute.try_into().unwrap();
                            // 時刻をグリニッジ標準時に補正
                            match parsed_datetime.tz_before_gmt {
                                true => {
                                    datetime =
                                        datetime + Duration::hours(hour) + Duration::minutes(minute)
                                }
                                false => {
                                    datetime =
                                        datetime - Duration::hours(hour) - Duration::minutes(minute)
                                }
                            };
                        }
                        // 時刻をグリニッジ標準時からローカル時刻に補正
                        datetime = datetime + Duration::seconds(timezone_offset as i64);

                        // メッセージに含まれる添付ファイルの一覧を列挙
                        let attachments = message.get_attachments();

                        // 添付ファイルを取り出す
                        for attachment in attachments {
                            // 添付ファイル名を取得
                            let attachment_name = attachment.get_attachment_name().unwrap();

                            // 添付ファイルの名前が一致するかどうかを判定
                            if regex.is_match(attachment_name) {
                                let path = [TMP_DIR, attachment_name].concat();
                                let mut file = File::create(&path).unwrap();
                                file.write_all(attachment.get_contents()).unwrap();
                                spreadsheet_path = Some(path);

                                // last_updateの有無を判定し、最終更新日時が受信したメールの日時以上かどうかを判定
                                match last_update.clone() {
                                    Some(l) => {
                                        if datetime >= l.datetime {
                                            // 最終更新日時を更新する
                                            last_update = Some(LastUpdate { datetime: datetime });
                                            break;
                                        } else {
                                            delete_tmp_dir();
                                            panic!("データは最新です。更新する必要はありません。");
                                        }
                                    }
                                    None => {
                                        // 最終更新日時を更新する
                                        last_update = Some(LastUpdate { datetime: datetime });
                                        break;
                                    }
                                }
                            }
                        }

                        if spreadsheet_path.is_some() {
                            break;
                        };
                    }

                    if spreadsheet_path.is_some() {
                        break;
                    };
                }

                println!("IMAPサーバからログアウトします。");
                session.logout().unwrap();
            }
            // ローカルのスプレッドシートを利用するモードの処理
            AppMode::Local => {
                println!("ローカルにあるスプレッドシートを使用します。");
                spreadsheet_path = match args.file_path {
                    Some(p) => Some(p),
                    None => panic!("スプレッドシートの場所が指定されていません。"),
                };
                last_update = Some(LastUpdate {
                    datetime: Local::now(),
                });
            }
        }

        let last_update = last_update.unwrap();

        // スプレッドシートを読み込む
        let mut spreadsheet: Xlsx<_> = match spreadsheet_path {
            Some(p) => match open_workbook(p) {
                Ok(w) => w,
                Err(_) => {
                    delete_tmp_dir();
                    panic!("スプレッドシートの読み込みに失敗しました。")
                }
            },
            None => panic!("スプレッドシートが見つかりませんでした。"),
        };

        // 日毎の陽性者数を格納する可変長配列を用意
        let mut patients_summary: Summary = Summary {
            data: Vec::new(),
            last_update: last_update.datetime,
        };
        // 日毎のPCR検査件数を格納する可変長配列を用意
        let mut inspections_summary: Summary = Summary {
            data: Vec::new(),
            last_update: last_update.datetime,
        };
        // 陽性者の状況を格納する構造体を用意
        let mut main_summary: Status = Status {
            attr: Attributes::Inspections,
            value: 0,
            children: None,
            last_update: None,
        };
        let mut news: NewsItems = NewsItems {
            news_items: Vec::new(),
        };

        // シートごとに構造体へ変換する
        for worksheet in WORKSHEETS_NAME {
            if let Some(Ok(range)) = spreadsheet.worksheet_range(worksheet) {
                println!("「{}」シートを処理します。", worksheet);
                match worksheet {
                    &"日毎の陽性者数" => {
                        for row in range.rows().rev() {
                            patients_summary.data.push(SummaryContent {
                                // 日付を取得する
                                date: DateTime::<Utc>::from_utc(row[0].as_datetime().unwrap(), Utc),
                                // 当日の陽性者数を取得する
                                sum: row[1].get_float().unwrap() as u32,
                            });
                        }
                    }
                    &"PCR検査件数" => {
                        // 最後の小計を格納する
                        let mut last_sum: u32 = 0;

                        for row in range.rows().rev() {
                            let sum = row[1].get_float().unwrap() as u32;
                            inspections_summary.data.push(SummaryContent {
                                // 日付を取得する
                                date: DateTime::<Utc>::from_utc(row[0].as_datetime().unwrap(), Utc),
                                // 現在の小計から前日の小計の差を求め、これを1日あたりの検査件数として計算する
                                sum: sum - last_sum,
                            });
                            // 小計を更新する
                            last_sum = sum;
                        }

                        // 陽性者の状況を取得する
                        // 最新の陽性者数を取得
                        main_summary.value = get_sum(&range, (0, 1)).unwrap();
                        let patients = Status {
                            attr: Attributes::Patients,
                            value: get_sum(&range, (0, 2)).unwrap(),
                            children: Some(vec![
                                Status {
                                    attr: Attributes::Hospitalizations,
                                    value: get_sum(&range, (0, 4)).unwrap(),
                                    children: None,
                                    last_update: None,
                                },
                                Status {
                                    attr: Attributes::SeverelyPatients,
                                    value: get_sum(&range, (0, 5)).unwrap(),
                                    children: None,
                                    last_update: None,
                                },
                                Status {
                                    attr: Attributes::Other,
                                    value: get_sum(&range, (0, 6)).unwrap(),
                                    children: None,
                                    last_update: None,
                                },
                                Status {
                                    attr: Attributes::Accommodations,
                                    value: get_sum(&range, (0, 7)).unwrap(),
                                    children: None,
                                    last_update: None,
                                },
                                Status {
                                    attr: Attributes::Home,
                                    value: get_sum(&range, (0, 8)).unwrap(),
                                    children: None,
                                    last_update: None,
                                },
                                Status {
                                    attr: Attributes::Dead,
                                    value: get_sum(&range, (0, 9)).unwrap(),
                                    children: None,
                                    last_update: None,
                                },
                                Status {
                                    attr: Attributes::Leave,
                                    value: get_sum(&range, (0, 3)).unwrap(),
                                    children: None,
                                    last_update: None,
                                },
                                Status {
                                    attr: Attributes::Coodinating,
                                    value: get_sum(&range, (0, 10)).unwrap(),
                                    children: None,
                                    last_update: None,
                                },
                            ]),
                            last_update: Some(last_update.datetime),
                        };
                        main_summary.children = Some(vec![patients]);
                    }
                    &"最新の情報" => {
                        for row in range.rows() {
                            news.news_items.push(NewsItem {
                                date: row[0].as_datetime().unwrap().date(),
                                text: row[1].get_string().unwrap().to_string(),
                                url: row[2].get_string().unwrap().to_string(),
                            });
                        }
                    }
                    _ => break,
                }
            } else {
                panic!("不明なシートが含まれています。");
            }
        }

        // 日毎の陽性者数のデータを出力
        let mut patients_summary_file =
            File::create([&EXTRACT_DIR, FILES_NAME[4]].concat()).unwrap();
        patients_summary_file
            .write_all(
                serde_json::to_string_pretty(&patients_summary)
                    .unwrap()
                    .as_bytes(),
            )
            .unwrap();
        // 1日あたりのPCR検査件数のデータを出力
        let mut inspections_summary_file =
            File::create([&EXTRACT_DIR, FILES_NAME[0]].concat()).unwrap();
        inspections_summary_file
            .write_all(
                serde_json::to_string_pretty(&inspections_summary)
                    .unwrap()
                    .as_bytes(),
            )
            .unwrap();
        // 陽性者の状況のデータを出力
        let mut main_summary_file = File::create([&EXTRACT_DIR, FILES_NAME[2]].concat()).unwrap();
        main_summary_file
            .write_all(
                serde_json::to_string_pretty(&main_summary)
                    .unwrap()
                    .as_bytes(),
            )
            .unwrap();
        // ニュースのデータを出力
        let mut news_file = File::create([&EXTRACT_DIR, FILES_NAME[3]].concat()).unwrap();
        news_file
            .write_all(serde_json::to_string_pretty(&news).unwrap().as_bytes())
            .unwrap();
        // 最終更新日時を出力
        let mut last_update_file = File::create([&EXTRACT_DIR, FILES_NAME[1]].concat()).unwrap();
        last_update_file
            .write_all(
                serde_json::to_string_pretty(&last_update)
                    .unwrap()
                    .as_bytes(),
            )
            .unwrap();

        println!("Done!");
    }
}

fn get_sum(range: &Range<DataType>, relative_position: (usize, usize)) -> Option<u32> {
    let cell: Option<&DataType> = range.get(relative_position);

    match cell {
        Some(cell) => {
            if cell.is_float() {
                Some(cell.get_float().unwrap() as u32)
            } else {
                Some(get_sum(range, (relative_position.0 + 1, relative_position.1)).unwrap())
            }
        }
        None => None,
    }
}

fn delete_tmp_dir() {
    // 一時ディレクトリを削除
    println!("一時ディレクトリを削除しています。");
    std::fs::remove_dir_all(TMP_DIR).unwrap();
}
