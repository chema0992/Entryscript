use entscript_converter::compile_to_json;
use serde_json::Value;

fn compile(source: &str) -> Value {
    serde_json::from_str(&compile_to_json(source).expect("Entscript should compile"))
        .expect("project JSON should parse")
}

fn object_blocks(project: &Value) -> Value {
    let script = project["objects"][0]["script"].as_str().unwrap();
    serde_json::from_str(script).unwrap()[0].clone()
}

#[test]
fn declares_global_variables_and_lists() {
    let project = compile(
        r#"
        var score = 42
        var player_name = "Ada"
        list inventory = ["coin", "key"]
        @엔트리봇 {
            when start {
                set(score, score + 1)
            }
        }
        "#,
    );

    let variables = project["variables"].as_array().unwrap();
    let score = variables.iter().find(|item| item["name"] == "score").unwrap();
    let player_name = variables.iter().find(|item| item["name"] == "player_name").unwrap();
    let inventory = variables.iter().find(|item| item["name"] == "inventory").unwrap();

    assert_eq!(score["variableType"], "variable");
    assert_eq!(score["value"], "42");
    assert_eq!(player_name["value"], "Ada");
    assert_eq!(inventory["variableType"], "list");
    assert_eq!(inventory["array"][0]["data"], "coin");
    assert_eq!(inventory["array"][1]["data"], "key");
}

#[test]
fn builds_list_statement_and_reporter_blocks() {
    let project = compile(
        r#"
        list inventory = ["coin"]
        @엔트리봇 {
            when start {
                list_add(inventory, "potion")
                list_insert(inventory, 1, "map")
                list_set(inventory, 2, "key")
                list_remove(inventory, 1)
                list_show(inventory)
                say(list_get(inventory, 1))
                say(list_length(inventory))
                if list_contains(inventory, "key") {
                    say("있음")
                }
            }
        }
        "#,
    );
    let blocks = object_blocks(&project);
    let types = blocks.as_array().unwrap().iter()
        .map(|block| block["type"].as_str().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(types[1], "add_value_to_list");
    assert_eq!(types[2], "insert_value_to_list");
    assert_eq!(types[3], "change_value_list_index");
    assert_eq!(types[4], "remove_value_from_list");
    assert_eq!(types[5], "show_list");
    assert_eq!(blocks[6]["params"][0]["type"], "value_of_index_from_list");
    assert_eq!(blocks[7]["params"][0]["type"], "length_of_list");
    assert_eq!(blocks[8]["params"][0]["type"], "is_included_in_list");
}

#[test]
fn creates_normal_functions_with_value_and_boolean_parameters() {
    let project = compile(
        r#"
        fn announce(message, condition: bool) {
            if condition {
                say(message)
            }
        }
        @엔트리봇 {
            when start {
                announce("ready", true)
            }
        }
        "#,
    );

    let function = &project["functions"][0];
    assert_eq!(function["type"], "normal");
    let content: Value = serde_json::from_str(function["content"].as_str().unwrap()).unwrap();
    let definition = &content[0][0];
    assert_eq!(definition["type"], "function_create");

    let body = &definition["statements"][0][0];
    assert_eq!(body["type"], "_if");
    assert!(function["content"].as_str().unwrap().contains("stringParam_"));
    assert!(function["content"].as_str().unwrap().contains("booleanParam_"));

    let blocks = object_blocks(&project);
    assert_eq!(blocks[1]["type"], serde_json::json!(format!("func_{}", function["id"].as_str().unwrap())));
    assert_eq!(blocks[1]["params"].as_array().unwrap().len(), 3);
}

#[test]
fn creates_value_functions_and_embeds_their_call_as_reporters() {
    let project = compile(
        r#"
        fn add(a, b) -> a + b {
        }
        var result = 0
        @엔트리봇 {
            when start {
                set(result, add(2, 3))
            }
        }
        "#,
    );

    let function = &project["functions"][0];
    assert_eq!(function["type"], "value");
    let content: Value = serde_json::from_str(function["content"].as_str().unwrap()).unwrap();
    assert_eq!(content[0][0]["type"], "function_create_value");
    assert_eq!(content[0][0]["params"][3]["type"], "calc_basic");

    let blocks = object_blocks(&project);
    assert_eq!(blocks[1]["type"], "set_variable");
    assert_eq!(blocks[1]["params"][1]["type"], serde_json::json!(format!("func_{}", function["id"].as_str().unwrap())));
}

#[test]
fn rejects_wrong_function_argument_count() {
    let error = compile_to_json(
        r#"
        fn greet(name) { say(name) }
        @엔트리봇 { when start { greet() } }
        "#,
    ).unwrap_err();
    assert!(error.to_string().contains("expects 1 argument(s)"));
}

#[test]
fn rejects_unknown_list_references() {
    let error = compile_to_json(
        r#"
        list items = []
        @엔트리봇 { when start { list_add(missing, 1) } }
        "#,
    ).unwrap_err();
    assert!(error.to_string().contains("Unknown list 'missing'"));
}


#[test]
fn rejects_using_a_list_as_a_scalar_variable() {
    let error = compile_to_json(
        r#"
        list inventory = ["coin"]
        @엔트리봇 { when start { say(inventory) } }
        "#,
    ).unwrap_err();
    assert!(error.to_string().contains("'inventory' is a list, not a scalar variable"));
}

#[test]
fn rejects_using_a_scalar_variable_as_a_list() {
    let error = compile_to_json(
        r#"
        var score = 0
        @엔트리봇 { when start { list_add(score, 1) } }
        "#,
    ).unwrap_err();
    assert!(error.to_string().contains("'score' is a scalar variable, not a list"));
}

#[test]
fn checks_boolean_function_parameter_arguments() {
    let error = compile_to_json(
        r#"
        fn announce(condition: bool) { if condition { say("yes") } }
        @엔트리봇 { when start { announce(123) } }
        "#,
    ).unwrap_err();
    assert!(error.to_string().contains("Argument 1 ('condition') of function 'announce' must be a boolean"));
}


#[test]
fn rejects_value_parameters_in_boolean_conditions() {
    let error = compile_to_json(
        r#"
        fn announce(message) {
            if message { say("yes") }
        }
        @엔트리봇 { when start { announce("hello") } }
        "#,
    ).unwrap_err();
    assert!(error.to_string().contains("Condition of block 'if' must be a boolean expression"));
}

#[test]
fn allows_boolean_parameters_to_be_forwarded_to_other_functions() {
    let project = compile(
        r#"
        fn inner(condition: bool) { if condition { say("inner") } }
        fn outer(condition: bool) { inner(condition) }
        @엔트리봇 { when start { outer(true) } }
        "#,
    );
    assert_eq!(project["functions"].as_array().unwrap().len(), 2);
}

#[test]
fn supports_forward_function_calls() {
    let project = compile(
        r#"
        fn first(value) { second(value) }
        fn second(value) { say(value) }
        @엔트리봇 { when start { first("ok") } }
        "#,
    );
    assert_eq!(project["functions"].as_array().unwrap().len(), 2);
}

#[test]
fn maps_additional_looks_and_math_blocks() {
    let project = compile(
        r#"
        @엔트리봇 {
            when start {
                add_effect("brightness", 10)
                set_effect("color", 20)
                change_size(5)
                set_size(120)
                stretch_size("WIDTH", 10)
                reset_size()
                flip_x()
                flip_y()
                say(sqrt(9) + abs(-2) + round(1.2))
            }
        }
        "#,
    );
    let blocks = object_blocks(&project);
    assert_eq!(blocks[1]["type"], "add_effect_amount");
    assert_eq!(blocks[2]["type"], "change_effect_amount");
    assert_eq!(blocks[3]["type"], "change_scale_size");
    assert_eq!(blocks[4]["type"], "set_scale_size");
    assert_eq!(blocks[5]["type"], "stretch_scale_size");
    assert_eq!(blocks[6]["type"], "reset_scale_size");
    assert_eq!(blocks[7]["type"], "flip_x");
    assert_eq!(blocks[8]["type"], "flip_y");
    assert_eq!(blocks[9]["params"][0]["type"], "calc_basic");
}


#[test]
fn function_body_can_read_and_update_global_variables() {
    let project = compile(
        r#"
        var score = 0
        fn add_score(amount) {
            change(score, amount)
            say(score)
        }
        @엔트리봇 { when start { add_score(5) } }
        "#,
    );

    let function = &project["functions"][0];
    let content: Value = serde_json::from_str(function["content"].as_str().unwrap()).unwrap();
    let body = &content[0][0]["statements"][0];
    assert_eq!(body[0]["type"], "change_variable");
    let score = project["variables"].as_array().unwrap().iter()
        .find(|variable| variable["name"] == "score").unwrap();
    assert_eq!(body[0]["params"][0], score["id"]);
    assert_eq!(body[1]["type"], "dialog");
    assert_eq!(body[1]["params"][0]["type"], "get_variable");
}

#[test]
fn boolean_value_function_can_be_passed_to_boolean_parameter() {
    let project = compile(
        r#"
        fn is_ready() -> 3 >= 2 { }
        fn show_if(condition: bool) {
            if condition { say("ready") }
        }
        @엔트리봇 { when start { show_if(is_ready()) } }
        "#,
    );
    assert_eq!(project["functions"].as_array().unwrap().len(), 2);
}

#[test]
fn rejects_value_function_used_as_a_statement() {
    let error = compile_to_json(
        r#"
        fn answer() -> 42 { }
        @엔트리봇 { when start { answer() } }
        "#,
    ).unwrap_err();
    assert!(error.to_string().contains("must be used inside an expression"));
}

#[test]
fn rejects_boolean_variable_as_a_boolean_parameter_input() {
    let error = compile_to_json(
        r#"
        var ready = true
        fn show_if(condition: bool) { if condition { say("ready") } }
        @엔트리봇 { when start { show_if(ready) } }
        "#,
    ).unwrap_err();
    assert!(error.to_string().contains("must be a boolean"));
}
