use serde_json::{json, Value};

use crate::error::CompileError;

pub fn default_objects() -> Value {
    json!([
        {
            "id": "7y0y",
            "name": "엔트리봇",
            "script": "[[]]",
            "objectType": "sprite",
            "rotateMethod": "free",
            "scene": "7dwq",
            "sprite": {
                "pictures": [{
                    "id": "vx80",
                    "dimension": {
                        "width": 144,
                        "height": 246,
                        "scaleX": 0.5128205128205128,
                        "scaleY": 0.5128205128205128
                    },
                    "fileurl": "./bower_components/entry-js/images/media/entrybot1.svg",
                    "thumbUrl": "./bower_components/entry-js/images/media/entrybot1.svg",
                    "name": "모양이름",
                    "imageType": "svg"
                }],
                "sounds": []
            },
            "selectedPictureId": "vx80",
            "lock": false,
            "entity": {
                "x": 0,
                "y": 0,
                "regX": 72,
                "regY": 123,
                "scaleX": 0.5128205128205128,
                "scaleY": 0.5128205128205128,
                "rotation": 0,
                "direction": 90,
                "width": 144,
                "height": 246,
                "font": "undefinedpx ",
                "visible": true
            }
        }
    ])
}

pub fn set_script_for_object(
    project: &mut Value,
    object_name: &str,
    script: String,
) -> Result<(), CompileError> {
    let objects = project
        .get_mut("objects")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| CompileError::Project("Project must contain an 'objects' array".into()))?;

    let mut matches = objects
        .iter_mut()
        .filter(|object| object.get("name").and_then(Value::as_str) == Some(object_name));
    let object = matches
        .next()
        .ok_or_else(|| CompileError::Project(format!("Object '{object_name}' does not exist")))?;
    if matches.next().is_some() {
        return Err(CompileError::Project(format!(
            "Object name '{object_name}' is ambiguous"
        )));
    }
    let map = object
        .as_object_mut()
        .ok_or_else(|| CompileError::Project("Object entry must be a JSON object".into()))?;
    map.insert("script".into(), Value::String(script));
    Ok(())
}
