use std::collections::BTreeMap;

use serde_json::Value;
use surrealdb_rpc_async::{Client, ClientAction};

#[tokio::test]
async fn main() {
    let mut connect = Client::builder("ws://127.0.0.1:8000/rpc".into())
        .with_err_handler(|e| {
            println!("{e}: {e:?}");
            ClientAction::IgnoreError
        })
        .build()
        .await
        .unwrap();

    let res = connect
        .ping()
        .unwrap()
        .send()
        .await
        .unwrap()
        .response()
        .await
        .unwrap();
    assert_eq!(res, Value::Bool(true));

    let res = connect
        .sign_in("root".into(), "root".into())
        .unwrap()
        .send()
        .await
        .unwrap()
        .response()
        .await
        .unwrap();
    assert_eq!(res, Value::String(String::new()));

    let ns = base64::encode(&rand::Rng::gen::<u128>(&mut rand::thread_rng()).to_le_bytes());

    let res = connect
        .use_ns_db("client_test".into(), ns)
        .unwrap()
        .send()
        .await
        .unwrap()
        .response()
        .await
        .unwrap();
    assert_eq!(res, Value::Null);

    let res = connect
        .query(
            "\
            SELECT bar FROM foo; \
            BEGIN TRANSACTION; \
            CREATE foo SET bar = 'bat'; \
            CANCEL TRANSACTION; \
            SELECT bar FROM foo \
        "
            .to_string(),
            BTreeMap::new(),
        )
        .unwrap()
        .send()
        .await
        .unwrap()
        .response()
        .await
        .unwrap();
    assert_eq!(res[0]["result"], Value::Array(vec![]));
    assert_eq!(res[2]["result"], Value::Array(vec![]));

    let res = connect
        .query(
            "\
            SELECT bar FROM foo; \
            BEGIN TRANSACTION; \
            CREATE foo SET bar = 'bat'; \
            COMMIT TRANSACTION; \
            SELECT bar FROM foo \
        "
            .to_string(),
            BTreeMap::new(),
        )
        .unwrap()
        .send()
        .await
        .unwrap()
        .response()
        .await
        .unwrap();
    assert_eq!(res[0]["result"], Value::Array(vec![]));
    assert_eq!(res[2]["result"][0]["bar"], Value::String("bat".into()));
}
