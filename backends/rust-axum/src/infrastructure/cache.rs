use redis::aio::MultiplexedConnection;

pub async fn create_client(redis_url: &str) -> MultiplexedConnection {
    let client = redis::Client::open(redis_url).expect("Invalid Redis URL");
    client
        .get_multiplexed_async_connection()
        .await
        .expect("Failed to connect to Redis")
}