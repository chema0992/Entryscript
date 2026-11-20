use serde_json::{json, Value};

pub fn default_metadata() -> Value {
    json!({
        "messages": [],
        "speed": 60,
        "interface": {
            "canvasWidth": 640,
            "object": "7y0y"
        },
        "expansionBlocks": [],
        "aiUtilizeBlocks": [],
        "hardwareLiteBlocks": [],
        "externalModules": [],
        "externalModulesLite": [],
        "likeCnt": 0,
        "visit": 0,
        "isopen": true,
        "name": "작품이름",
        "isPracticalCourse": false,
        "user": "6344fa93a96216005729b2f2",
        "recentLikeCnt": 0,
        "childCnt": 0,
        "comment": 0,
        "tables": []
    })
}
