# Unofficial SurrealDB RPC Client


```rust
let mut connect = Client::builder("ws://127.0.0.1:8000/rpc")
    .with_err_handler(|e| {
        println!("{e}: {e:?}");
        ClientAction::IgnoreError
    })
    .build()
    .await?;

let res = connect
    .ping()?
    .send()
    .await?
    .response()
    .await?;
assert_eq!(res, Value::Bool(true));

let res = connect
    .sign_in("root".into(), "root".into())?
    .send()
    .await?
    .response()
    .await?;
assert_eq!(res, Value::String(String::new()));

let res = connect
    .use_ns_db("my_db".into(), "my_ns".into()?
    .send()
    .await?
    .response()
    .await?;
assert_eq!(res, Value::Null);

let res = connect
    .query("\
        SELECT bar FROM foo; \
        BEGIN TRANSACTION; \
        CREATE foo SET bar = 'bat'; \
        CANCEL TRANSACTION; \
        SELECT bar FROM foo \
    ".to_string(), BTreeMap::new())?
    .send()
    .await?
    .response()
    .await?;
assert_eq!(res[0]["result"], Value::Array(vec![]));
assert_eq!(res[2]["result"], Value::Array(vec![]));

let res = connect
    .query("\
        SELECT bar FROM foo; \
        BEGIN TRANSACTION; \
        CREATE foo SET bar = 'bat'; \
        COMMIT TRANSACTION; \
        SELECT bar FROM foo \
    ".to_string(), BTreeMap::new())?
    .send()
    .await?
    .response()
    .await?;
assert_eq!(res[0]["result"], Value::Array(vec![]));
assert_eq!(res[2]["result"][0]["bar"], Value::String("bat".into()));
```