use std::sync::Mutex; //相互排他機能：複数のスレッドが同時にデータにアクセスすることを防ぐ。データ整合性を保つ

use actix_web::web::Data;
use actix_web::{guard, web, App, HttpResponse, HttpServer, Result};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql::{EmptySubscription, Enum, InputObject, Object, Schema, SimpleObject};
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};
use once_cell::sync::Lazy; //global dataや静的リソースを遅延初期化(データ呼び出し時まで初期化を遅らせる)する。

// Query構造体を定義
struct Query;

// Lazyを使用してスレッドセーフ化することで、プログラムの実行開始時ではなく、
// 最初にSEQUENCE, PHOTOSが使用されるときに初期化する。外部リソースへの接続など（時間がかかるものに有効）
static SEQUENCE_ID: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));
// ||: クロージャ表現。空のベクタを初期値としている。複数スレッドからのアクセスを同期するために使用される。
static PHOTOS: Lazy<Mutex<Vec<Photo>>> = Lazy::new(|| Mutex::new(vec![]));
#[derive(SimpleObject, Clone)]
struct Photo {
    id: usize,
    name: String,
    description: String,
    category: PhotoCategory,
}
struct Mutation;

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
enum PhotoCategory {
    Selfie,
    Portrailt,
    Action,
    Landscape,
    Graphic,
}

impl Default for PhotoCategory {
    fn default() -> Self {
        PhotoCategory::Portrailt
    }
}
#[derive(InputObject)]
struct PostPhotoInput {
    name: String,
    description: String,
    #[graphql(default_with = "PhotoCategory::default()")]
    category: PhotoCategory,
}

#[Object]
impl Mutation {
    async fn post_photo(&self, input: PostPhotoInput) -> Photo {
        let mut id = SEQUENCE_ID.lock().unwrap();
        *id += 1;
        let photo = Photo {
            id: *id,
            name: input.name,
            description: input.description,
            category: input.category,
        };
        // PHOTOSグローバルMutext<Vec<Photo>>に新しいPhotoオブジェクトをpushする
        // .lock(): このMutexをロックし、保持しているデータへのアクセスを試みる
        // .unwrap(): ロックの取得が成功した場合、MutexGuardを取り出し、失敗した場合Panicを起こす
        // .clone(): Photoオブジェクトのディープコピーを作成。オリジナルのphotoオブジェクトは関数呼び出し元に返却
        PHOTOS.lock().unwrap().push(photo.clone());
        photo
    }
}

// async-graphqlクレートによって提供されたもので、レスポンス(42)を返す
#[Object]
impl Query {
    async fn total_photos(&self) -> usize {
        PHOTOS.lock().unwrap().len()
    }
    async fn all_photos(&self) -> Vec<Photo> {
        PHOTOS.lock().unwrap().clone()
    }
}

type ApiSchema = Schema<Query, Mutation, EmptySubscription>;

async fn index(schema: web::Data<ApiSchema>, req: GraphQLRequest) -> GraphQLResponse {
    // into(): 実行結果をGraphQLResponseに変換する
    schema.execute(req.into_inner()).await.into()
}

// GraphQL Playgroundを提供するためのエンドポイント
async fn index_playground() -> Result<HttpResponse> {
    // GraphQLPlaygroundConfig::new("/"): PlaygroundがGraphQLクエリを送信するエンドポイントを設定する
    let source = playground_source(GraphQLPlaygroundConfig::new("/").subscription_endpoint("/"));
    // HTMLソースを含むHTTPレスポンスを構築。ブラウザでPlaygroundを開いたとき、適切なHTMLコンテンツが表示される。
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(source))
}

// 非同期関数をmain関数として使用できるようにするためのマクロ。
// main関数内で.awaitを使用できる
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let schema = Schema::build(Query, Mutation, EmptySubscription).finish();

    println!("Playground: http://localhost:8000");

    HttpServer::new(move || {
        // App::new(): 新しいActix Webアプリのインスタンスを生成する。
        App::new()
            .app_data(Data::new(schema.clone())) // schema.clone(): 各ワーカーにスキーマのコピーを提供する
            .service(web::resource("/").guard(guard::Post()).to(index)) // Postリクエスト'/'パスで受け取り、indexにルーディング
            .service(web::resource("/").guard(guard::Get()).to(index_playground))
        // Postリクエスト'/'パスで受け取り、index_playgroundにルーディング
    })
    .bind("127.0.0.1:8000")?
    .run()
    .await
}
