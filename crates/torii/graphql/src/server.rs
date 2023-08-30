use std::time::Duration;

use async_graphql::http::GraphiQLSource;
use async_graphql_poem::{GraphQL, GraphQLSubscription};
use async_recursion::async_recursion;
use poem::listener::TcpListener;
use poem::middleware::Cors;
use poem::web::Html;
use poem::{get, handler, EndpointExt, IntoResponse, Route, Server};
use sqlx::{Pool, Sqlite};
use tokio_stream::StreamExt;
use torii_core::simple_broker::SimpleBroker;
use torii_core::types::Component;

use super::schema::build_schema;

#[handler]
async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/").subscription_endpoint("/ws").finish())
}

#[async_recursion]
pub async fn start(pool: Pool<Sqlite>) -> anyhow::Result<()> {
    let schema = build_schema(&pool).await?;

    let app = Route::new()
        .at("/", get(graphiql).post(GraphQL::new(schema.clone())))
        .at("/ws", get(GraphQLSubscription::new(schema)))
        .with(Cors::new());

    println!("Open GraphiQL IDE: http://localhost:8080");
    Server::new(TcpListener::bind("0.0.0.0:8080"))
        .run_with_graceful_shutdown(
            app,
            async move {
                // HACK: catch component register event and shutdown server, start() is recursively
                // called so schema is rebuilt and server restarted
                let mut sub = SimpleBroker::<Component>::subscribe();
                while let Some(_) = sub.next().await {
                    return;
                }
            },
            Some(Duration::from_secs(1)),
        )
        .await?;

    start(pool).await?;

    Ok(())
}
