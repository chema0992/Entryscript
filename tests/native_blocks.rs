use entscript_converter::compile_to_json;
use serde_json::Value;

fn compiled_blocks(source: &str) -> Value {
    let project: Value = serde_json::from_str(&compile_to_json(source).expect("compile should succeed"))
        .expect("project JSON should be valid");
    let script = project["objects"][0]["script"]
        .as_str()
        .expect("object.script should be a JSON string");
    let threads: Value = serde_json::from_str(script).expect("object.script should contain valid JSON");
    threads[0].clone()
}

#[test]
fn loop_and_wait_use_entry_flow_layout() {
    let blocks = compiled_blocks(
        r#"
        @엔트리봇 {
            when start {
                loop {
                    wait(2)
                }
            }
        }
        "#,
    );

    assert_eq!(blocks[0]["type"], "when_run_button_click");
    assert_eq!(blocks[1]["type"], "repeat_inf");
    assert_eq!(blocks[1]["params"], serde_json::json!([null]));
    assert_eq!(blocks[1]["statements"].as_array().unwrap().len(), 1);
    assert_eq!(blocks[1]["statements"][0][0]["type"], "wait_second");
    assert_eq!(blocks[1]["statements"][0][0]["params"][0]["type"], "number");
}

#[test]
fn counted_loop_uses_repeat_basic() {
    let blocks = compiled_blocks(
        r#"
        @엔트리봇 {
            when start {
                for 5 {
                    say("반복")
                }
            }
        }
        "#,
    );

    assert_eq!(blocks[1]["type"], "repeat_basic");
    assert_eq!(blocks[1]["params"].as_array().unwrap().len(), 2);
    assert_eq!(blocks[1]["statements"][0][0]["type"], "dialog");
}

#[test]
fn conditional_else_and_comparison_are_serialized() {
    let blocks = compiled_blocks(
        r#"
        @엔트리봇 {
            when start {
                if 3 >= 2 {
                    say("yes")
                } else {
                    say("no")
                }
            }
        }
        "#,
    );

    assert_eq!(blocks[1]["type"], "if_else");
    assert_eq!(blocks[1]["params"][0]["type"], "boolean_basic_operator");
    assert_eq!(blocks[1]["params"][0]["params"][1], "GREATER_OR_EQUAL");
    assert_eq!(blocks[1]["statements"].as_array().unwrap().len(), 2);
    assert_eq!(blocks[1]["statements"][0][0]["type"], "dialog");
    assert_eq!(blocks[1]["statements"][1][0]["type"], "dialog");
}

#[test]
fn raw_block_preserves_native_parameter_slots() {
    let blocks = compiled_blocks(
        r#"
        @엔트리봇 {
            when start {
                raw_block("native_example", field("MODE_A"), raw(12), raw(-3), null)
            }
        }
        "#,
    );

    assert_eq!(blocks[1]["type"], "native_example");
    assert_eq!(blocks[1]["params"], serde_json::json!(["MODE_A", 12, -3, null]));
}

#[test]
fn variables_can_be_read_and_written() {
    let blocks = compiled_blocks(
        r#"
        @엔트리봇 {
            when start {
                set(초시계, 초시계 + 1)
            }
        }
        "#,
    );

    assert_eq!(blocks[1]["type"], "set_variable");
    assert_eq!(blocks[1]["params"][0], "brih");
    assert_eq!(blocks[1]["params"][1]["type"], "calc_basic");
}


#[test]
fn get_variable_reporter_resolves_variable_name_to_id() {
    let blocks = compiled_blocks(
        r#"
        @엔트리봇 {
            when start {
                say(get_variable(초시계))
            }
        }
        "#,
    );

    assert_eq!(blocks[1]["params"][0]["type"], "get_variable");
    assert_eq!(blocks[1]["params"][0]["params"][0], "brih");
}

#[test]
fn extension_and_ai_blocks_are_rejected() {
    let source = r#"
        @엔트리봇 {
            when start {
                block_expansion_weather()
            }
        }
    "#;
    let error = compile_to_json(source).expect_err("extension blocks must be excluded");
    assert!(error.to_string().contains("extension and AI"));
}


#[test]
fn native_type_ids_work_for_events_statements_and_reporters() {
    let blocks = compiled_blocks(
        r#"
        @엔트리봇 {
            when custom_native_event(null) {
                raw_block("custom_native_statement", field("MODE"), raw(7), null) {
                    say(raw_block("custom_native_reporter", field("VALUE")))
                }
            }
        }
        "#,
    );

    assert_eq!(blocks[0]["type"], "custom_native_event");
    assert_eq!(blocks[0]["params"], serde_json::json!([null]));
    assert_eq!(blocks[1]["type"], "custom_native_statement");
    assert_eq!(blocks[1]["params"], serde_json::json!(["MODE", 7, null]));
    assert_eq!(
        blocks[1]["statements"][0][0]["params"][0]["type"],
        "custom_native_reporter"
    );
}

#[test]
fn timed_dialog_matches_entry_parameter_positions() {
    let blocks = compiled_blocks(
        r#"
        @엔트리봇 {
            when start {
                say_for("잠깐", 2)
            }
        }
        "#,
    );

    assert_eq!(blocks[1]["type"], "dialog_time");
    assert_eq!(blocks[1]["params"].as_array().unwrap().len(), 4);
    assert_eq!(blocks[1]["params"][0]["type"], "text");
    assert_eq!(blocks[1]["params"][1]["type"], "number");
    assert_eq!(blocks[1]["params"][2], Value::Null);
    assert_eq!(blocks[1]["params"][3], Value::Null);
}

#[test]
fn unknown_statement_blocks_are_rejected() {
    let source = r#"
        @엔트리봇 {
            when start {
                fly(10)
            }
        }
    "#;
    let error = compile_to_json(source).expect_err("unknown blocks must be rejected");
    assert!(error.to_string().contains("Unknown block 'fly'"));
}

#[test]
fn project_json_starts_with_objects_key() {
    let output = compile_to_json(
        r#"
        @엔트리봇 {
            when start {
                say("안녕")
            }
        }
        "#,
    ).expect("compilation should succeed");
    assert!(output.starts_with("{\"objects\":"));
}
