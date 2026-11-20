use serde_json::{json, Value};

pub fn default_scenes() -> Value {
    json!([
        {
            "id": "7dwq",
            "name": "장면이름"
        }
    ])
}
