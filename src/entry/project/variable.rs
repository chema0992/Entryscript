use serde_json::{json, Value};

pub fn default_variables() -> Value {
    json!([
        {
            "name": "초시계",
            "id": "brih",
            "visible": false,
            "value": "0",
            "variableType": "timer",
            "isCloud": false,
            "isRealTime": false,
            "cloudDate": false,
            "object": null,
            "x": 134,
            "y": -70
        },
        {
            "name": "대답",
            "id": "1vu8",
            "visible": false,
            "value": "0",
            "variableType": "answer",
            "isCloud": false,
            "isRealTime": false,
            "cloudDate": false,
            "object": null,
            "x": 150,
            "y": -100
        }
    ])
}
