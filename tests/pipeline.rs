use serde_json::Value;
use entscript_converter::{compile_to_json, compile_with_project_json};

const SOURCE: &str = r#"
@엔트리봇 {
    when start {
        say("안녕")
    }
}
"#;

#[test]
fn compiles_hello_script_into_entry_project_json() {
    let output = compile_to_json(SOURCE).expect("compilation should succeed");
    let project: Value = serde_json::from_str(&output).expect("outer JSON should parse");

    assert_eq!(project["objects"][0]["name"], "엔트리봇");
    assert_eq!(project["objects"][0]["sprite"]["pictures"][0]["name"], "모양이름");
    assert!(output.starts_with("{\"objects\":"));
    assert!(!output.contains('\n'));

    let script_string = project["objects"][0]["script"]
        .as_str()
        .expect("object.script should be a JSON string");
    let script: Value = serde_json::from_str(script_string)
        .expect("the embedded script JSON should parse");

    assert_eq!(script.as_array().unwrap().len(), 1);
    assert_eq!(script[0].as_array().unwrap().len(), 2);
    assert_eq!(script[0][0]["type"], "when_run_button_click");
    assert_eq!(script[0][0]["x"], 50);
    assert_eq!(script[0][0]["y"], 30);
    assert_eq!(script[0][0]["deletable"], 1);
    assert_eq!(script[0][1]["type"], "dialog");
    assert_eq!(script[0][1]["params"][1], "speak");
    assert_eq!(script[0][1]["params"][0]["type"], "text");
    assert_eq!(script[0][1]["params"][0]["params"][0], "안녕");
}

#[test]
fn generated_block_ids_are_unique() {
    let output = compile_to_json(SOURCE).expect("compilation should succeed");
    let project: Value = serde_json::from_str(&output).unwrap();
    let script: Value = serde_json::from_str(
        project["objects"][0]["script"].as_str().unwrap(),
    )
    .unwrap();

    let event_id = script[0][0]["id"].as_str().unwrap();
    let dialog_id = script[0][1]["id"].as_str().unwrap();
    let text_id = script[0][1]["params"][0]["id"].as_str().unwrap();

    assert_ne!(event_id, dialog_id);
    assert_ne!(event_id, text_id);
    assert_ne!(dialog_id, text_id);

    for id in [event_id, dialog_id, text_id] {
        assert_eq!(id.len(), 4);
        assert!(id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    }
}

#[test]
fn preserves_entry_block_field_order_inside_script_string() {
    let output = compile_to_json(SOURCE).expect("compilation should succeed");
    let project: Value = serde_json::from_str(&output).unwrap();
    let script = project["objects"][0]["script"].as_str().unwrap();

    assert!(script.starts_with("[[{\"id\":\""));
    assert!(script.contains("\",\"x\":50,\"y\":30,\"type\":\"when_run_button_click\""));
}

#[test]
fn rejects_unknown_objects() {
    let source = "@없는오브젝트 { when start { say(\"안녕\") } }";
    let error = compile_to_json(source).unwrap_err().to_string();
    assert!(error.contains("does not exist"));
}

#[test]
fn rejects_unknown_blocks() {
    let source = "@엔트리봇 { when start { fly(10) } }";
    let error = compile_to_json(source).unwrap_err().to_string();
    assert!(error.contains("Unknown block 'fly'"));
}

#[test]
fn keeps_unrelated_data_when_compiling_into_existing_project() {
    let existing = include_str!("../examples/expected-project.json");
    let output = compile_with_project_json(SOURCE, existing)
        .expect("existing project compilation should succeed");
    let project: Value = serde_json::from_str(&output).unwrap();

    assert_eq!(project["name"], "작품이름");
    assert_eq!(project["objects"][0]["id"], "7y0y");
    assert_eq!(project["scenes"][0]["id"], "7dwq");

    let script: Value = serde_json::from_str(
        project["objects"][0]["script"].as_str().unwrap(),
    )
    .unwrap();
    assert_eq!(script[0][0]["type"], "when_run_button_click");
}
