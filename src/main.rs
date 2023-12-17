use actix_web::web::Data;
use actix_web::{guard, web, App, HttpResponse, HttpServer, Result};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql::{EmptyMutation, EmptySubscription, Object, Schema};
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};

// Query構造体を定義
struct Query;

// async-graphqlクレートによって提供されたもので、レスポンス(42)を返す
#[Object]
impl Query {
    async fn total_photos(&self) -> usize {
        42
    }
}

type ApiSchema = Schema<Query, EmptyMutation, EmptySubscription>;

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
    let schema = Schema::build(Query, EmptyMutation, EmptySubscription).finish();

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
