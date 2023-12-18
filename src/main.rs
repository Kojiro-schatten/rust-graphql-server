use std::sync::Mutex;

use actix_web::web::Data;
use actix_web::{guard, web, App, HttpResponse, HttpServer, Result};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql::{EmptySubscription, Object, Schema, SimpleObject};
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};
use once_cell::sync::Lazy;

// Query構造体を定義
struct Query;

static SEQUENCE_ID: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));
static PHOTOS: Lazy<Mutex<Vec<Photo>>> = Lazy::new(|| Mutex::new(vec![]));
#[derive(SimpleObject, Clone)]
struct Photo {
    id: usize,
    name: String,
    description: String,
}
struct Mutation;

#[Object]
impl Mutation {
    async fn post_photo(&self, name: String, description: String) -> Photo {
        let mut id = SEQUENCE_ID.lock().unwrap();
        *id += 1;
        let photo = Photo {
            id: *id,
            name,
            description,
        };
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
