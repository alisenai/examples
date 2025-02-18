use async_graphql::http::{playground_source, GraphQLPlaygroundConfig, ALL_WEBSOCKET_PROTOCOLS};
use async_graphql::{EmptyMutation, Schema};
use async_graphql_poem::{GraphQLProtocol, GraphQLRequest, GraphQLResponse, GraphQLWebSocket};
use poem::http::HeaderMap;
use poem::listener::TcpListener;
use poem::web::{websocket::WebSocket, Data, Html};
use poem::{get, handler, EndpointExt, IntoResponse, Route, Server};
use token::{on_connection_init, QueryRoot, SubscriptionRoot, Token, TokenSchema};

fn get_token_from_headers(headers: &HeaderMap) -> Option<Token> {
    headers
        .get("Token")
        .and_then(|value| value.to_str().map(|s| Token(s.to_string())).ok())
}

#[handler]
async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(
        GraphQLPlaygroundConfig::new("/").subscription_endpoint("/ws"),
    ))
}

#[handler]
async fn index(
    schema: Data<&TokenSchema>,
    headers: &HeaderMap,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let mut req = req.0;
    if let Some(token) = get_token_from_headers(headers) {
        req = req.data(token);
    }
    schema.execute(req).await.into()
}

#[handler]
async fn ws(
    schema: Data<&TokenSchema>,
    headers: &HeaderMap,
    protocol: GraphQLProtocol,
    websocket: WebSocket,
) -> impl IntoResponse {
    let mut data = async_graphql::Data::default();
    if let Some(token) = get_token_from_headers(headers) {
        data.insert(token);
    }

    let schema = schema.0.clone();
    websocket
        .protocols(ALL_WEBSOCKET_PROTOCOLS)
        .on_upgrade(move |stream| {
            GraphQLWebSocket::new(stream, schema, protocol)
                .with_data(data)
                .on_connection_init(on_connection_init)
                .serve()
        })
}

#[tokio::main]
async fn main() {
    let schema = Schema::new(QueryRoot, EmptyMutation, SubscriptionRoot);

    let app = Route::new()
        .at("/", get(graphql_playground).post(index))
        .at("/ws", get(ws))
        .data(schema);

    println!("Playground: http://localhost:8000");
    Server::new(TcpListener::bind("0.0.0.0:8000"))
        .run(app)
        .await
        .unwrap();
}
