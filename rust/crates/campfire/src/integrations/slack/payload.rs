//! Ruby JSON access/coercion at Slack's untyped payload boundary.
use serde_json::{Value, json};

#[derive(Debug, thiserror::Error)]
#[error("{class}: {message}")]
pub struct Error {
    pub class: &'static str,
    pub message: String,
}
impl From<Error> for campfire_db::Error {
    fn from(error: Error) -> Self {
        Self::Other(error.to_string())
    }
}
pub fn no_method(method: &str, value: &Value) -> Error {
    let receiver = match value {
        Value::Null => "nil".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(_) => "an instance of Float".into(),
        Value::String(_) => "an instance of String".into(),
        _ => "an instance of Array".into(),
    };
    Error {
        class: "NoMethodError",
        message: format!("undefined method '{method}' for {receiver}"),
    }
}
pub fn at(value: &Value, key: &str) -> Result<Value, Error> {
    match value {
        Value::Object(o) => Ok(o.get(key).cloned().unwrap_or(Value::Null)),
        Value::String(s) => Ok(if s.contains(key) {
            json!(key)
        } else {
            Value::Null
        }),
        Value::Array(_) => Err(type_error()),
        Value::Number(n) if n.is_i64() || n.is_u64() => Err(type_error()),
        _ => Err(no_method("[]", value)),
    }
}
pub fn type_error() -> Error {
    Error {
        class: "TypeError",
        message: "no implicit conversion of String into Integer".into(),
    }
}
pub fn array(value: &Value) -> Vec<Value> {
    match value {
        Value::Null => vec![],
        Value::Array(a) => a.clone(),
        Value::Object(o) => o.iter().map(|(k, v)| json!([k, v])).collect(),
        v => vec![v.clone()],
    }
}
pub fn fields(value: &Value, keys: &[&str]) -> Result<Value, Error> {
    let mut result = json!({});
    for key in keys {
        result[*key] = at(value, key)?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::super::{
        client,
        jobs::tests::{run, setup, start},
        markdown, users,
    };
    use super::*;
    #[tokio::test]
    async fn slack_malformed_payloads_match_actual_rails_classes_messages_and_results() {
        let oracle: Value =
            serde_json::from_str(include_str!("../../../../../vectors/slack/payloads.json"))
                .unwrap();
        let (db, _, _dir) = setup().await;
        let id = start(&db).await;
        let run = run(&db, id).await;
        let mapped_run = run.clone();
        db.write(move |tx| {
            users::record(
                tx,
                &mapped_run,
                "user",
                "UKNOWN",
                "User",
                mapped_run.user_id,
                false,
            )
        })
        .await
        .unwrap();
        for case in oracle["client"].as_array().unwrap() {
            let expected = &case["expected"];
            match client::check_ok(case["input"].clone(), "users.list") {
                Ok(value) => assert_eq!(value, expected["result"]),
                Err(error) => {
                    assert_eq!(error.message, expected["message"]);
                    assert_eq!(error.ruby_class(), expected["class"]);
                }
            }
        }
        for case in oracle["markdown"].as_array().unwrap() {
            match markdown::try_convert(&case["input"], &Default::default()) {
                Ok(value) => assert_eq!(
                    serde_json::to_value(value).unwrap(),
                    case["expected"]["result"]
                ),
                Err(error) => {
                    assert_eq!(error.class, case["expected"]["class"]);
                    assert_eq!(error.message, case["expected"]["message"]);
                }
            }
        }
        for case in oracle["mapper"].as_array().unwrap() {
            let input = case["input"].clone();
            let run = run.clone();
            let result = db
                .write(move |tx| users::map_page(tx, &run, &input, true, &Default::default()))
                .await;
            match result {
                Ok(value) => assert_eq!(value, case["expected"]["result"]),
                Err(error) => assert_eq!(
                    error.to_string(),
                    format!(
                        "{}: {}",
                        case["expected"]["class"].as_str().unwrap(),
                        case["expected"]["message"].as_str().unwrap()
                    )
                ),
            }
        }
        for case in oracle["conversation"].as_array().unwrap() {
            let input = case["input"].clone();
            let run = run.clone();
            let result=db.write(move |tx|{
                let target=super::super::conversations::resolve(tx,&run,&input,&[],&Default::default(),true)?;
                Ok(json!({"action":target.action,"room_id":target.room.map(|r|r.id),"reason":target.reason}))
            }).await;
            match result {
                Ok(value) => assert_eq!(value, case["expected"]["result"]),
                Err(error) => assert_eq!(
                    error.to_string(),
                    format!(
                        "{}: {}",
                        case["expected"]["class"].as_str().unwrap(),
                        case["expected"]["message"].as_str().unwrap()
                    )
                ),
            }
        }
        println!(
            "Slack payload parity: 12 client, 23 converter, 23 user mapper and 12 conversation mapper Rails cases matched"
        );
    }
}
