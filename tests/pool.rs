
#[cfg(feature="pool")]
#[tokio::test]
async fn main() {
    use serde_json::Value;
    use surrealdb_ws_async::{Client, ClientAction};
    
    let pool = Client::builder("ws://127.0.0.1:8000/rpc")
        .with_err_handler(|e| {
            println!("{e}: {e:?}");
            ClientAction::IgnoreError
        }).build_pool();

    let mut tasks = Vec::new();
    for _ in 1..100 {
        let pool = pool.clone();
        tasks.push(tokio::task::spawn(async move {
            let mut conn = pool.get().await.unwrap();
            let res = conn.ping().unwrap().send().await.unwrap().response().await.unwrap();
            assert_eq!(res, Value::Bool(true));
        }));
    }
    let _ = futures::future::select_all(tasks);
}

#[cfg(not(feature="pool"))]
async fn main() {
    // this test only runs with the pool feature enabled
}