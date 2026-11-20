use entscript_converter::compile_to_json;
use serde_json::Value;

fn compile(source: &str) -> Value {
    serde_json::from_str(&compile_to_json(source).expect("Entscript should compile"))
        .expect("compiled project JSON should be valid")
}

fn blocks(project: &Value) -> Value {
    let script = project["objects"][0]["script"].as_str().unwrap();
    serde_json::from_str(script).unwrap()[0].clone()
}

#[test]
fn compiles_additional_motion_blocks() {
    let project = compile(
        r#"
        @엔트리봇 {
            when start {
                move_xy_time(1, 10, 20)
                locate_xy_time(2, 100, 200)
                locate_object_time(1, "mouse")
                turn_direction(30)
                rotate_absolute(90)
                rotate_by_time(1, 45)
                direction_relative_duration(1, 45)
                point_towards("mouse")
                move_to_angle(90, 20)
            }
        }
        "#,
    );
    let result = blocks(&project);
    let list = result.as_array().unwrap();
    let types = list.iter().map(|block| block["type"].as_str().unwrap()).collect::<Vec<_>>();
    assert_eq!(types[1..], [
        "move_xy_time", "locate_xy_time", "locate_object_time", "direction_relative",
        "rotate_absolute", "rotate_by_time", "direction_relative_duration", "see_angle_object", "move_to_angle",
    ]);
    assert_eq!(list[1]["params"].as_array().unwrap().len(), 4);
    assert_eq!(list[3]["params"].as_array().unwrap().len(), 3);
}

#[test]
fn compiles_brush_blocks() {
    let project = compile(
        r#"
        @엔트리봇 {
            when start {
                start_drawing()
                set_color("#ff0000")
                change_thickness(2)
                set_thickness(4)
                set_random_color()
                start_fill()
                set_fill_color("#00ff00")
                change_brush_transparency(10)
                set_brush_transparency(25)
                stop_fill()
                brush_stamp()
                clear_brush()
                stop_drawing()
            }
        }
        "#,
    );
    let result = blocks(&project);
    let types = result.as_array().unwrap().iter()
        .map(|block| block["type"].as_str().unwrap()).collect::<Vec<_>>();
    assert_eq!(types[1..], [
        "start_drawing", "set_color", "change_thickness", "set_thickness", "set_random_color",
        "start_fill", "set_fill_color", "change_brush_transparency", "set_brush_tranparency",
        "stop_fill", "brush_stamp", "brush_erase_all", "stop_drawing",
    ]);
}

#[test]
fn compiles_text_box_blocks_and_reporters() {
    let project = compile(
        r#"
        @엔트리봇 {
            when start {
                text_write("hello")
                text_append(" world")
                text_prepend("say: ")
                text_change_effect("underLine", "off")
                text_change_font("Arial")
                text_change_font_color("#ffffff")
                text_change_bg_color("#000000")
                text_flush()
                say(text_read("self"))
            }
        }
        "#,
    );
    let result = blocks(&project);
    let list = result.as_array().unwrap();
    assert_eq!(list[1]["type"], "text_write");
    assert_eq!(list[2]["type"], "text_append");
    assert_eq!(list[3]["type"], "text_prepend");
    assert_eq!(list[4]["type"], "text_change_effect");
    assert_eq!(list[4]["params"][0], "underLine");
    assert_eq!(list[4]["params"][1], "off");
    assert_eq!(list[5]["type"], "text_change_font");
    assert_eq!(list[6]["type"], "text_change_font_color");
    assert_eq!(list[7]["type"], "text_change_bg_color");
    assert_eq!(list[8]["type"], "text_flush");
    assert_eq!(list[9]["params"][0]["type"], "text_read");
}

#[test]
fn compiles_sound_speed_controls() {
    let project = compile(
        r#"
        @엔트리봇 {
            when start {
                change_sound_speed(0.1)
                set_sound_speed(1.5)
                say(get_sound_speed())
            }
        }
        "#,
    );
    let result = blocks(&project);
    assert_eq!(result[1]["type"], "sound_speed_change");
    assert_eq!(result[2]["type"], "sound_speed_set");
    assert_eq!(result[3]["params"][0]["type"], "get_sound_speed");
}

#[test]
fn compiles_string_date_distance_and_user_reporters() {
    let project = compile(
        r#"
        @엔트리봇 {
            when start {
                say(string_length("abc"))
                say(reverse_string("abc"))
                say(combine("hello", "world"))
                say(char_at("hello", 2))
                say(substring("hello", 2, 4))
                say(index_of("hello", "l"))
                say(replace_string("hello", "l", "x"))
                say(uppercase("hello"))
                say(lowercase("HELLO"))
                say(get_date("YEAR"))
                say(distance_to("mouse"))
                say(get_user_name())
                say(get_nickname())
                say(quotient(7, 2))
            }
        }
        "#,
    );
    let result = blocks(&project);
    let reporters = result.as_array().unwrap().iter().skip(1)
        .map(|block| block["params"][0]["type"].as_str().unwrap()).collect::<Vec<_>>();
    assert_eq!(reporters, [
        "length_of_string", "reverse_of_string", "combine_something", "char_at", "substring",
        "index_of_string", "replace_string", "change_string_case", "change_string_case",
        "get_date", "distance_something", "get_user_name", "get_nickname", "quotient_and_mod",
    ]);
    assert_eq!(result[1]["params"][0]["params"][3], "toUpperCase");
    assert_eq!(result[2]["params"][0]["params"][3], "toLowerCase");
}

#[test]
fn compiles_answer_and_project_timer_controls() {
    let project = compile(
        r#"
        @엔트리봇 {
            when start {
                show_answer()
                hide_answer()
                timer_start()
                timer_stop()
                timer_reset()
                timer_action("START")
                show_project_timer()
                hide_project_timer()
                stop_this_thread()
                stop_other_objects()
                stop_all_sounds()
                stop_sounds("thisOnly")
            }
        }
        "#,
    );
    let result = blocks(&project);
    let list = result.as_array().unwrap();
    let types = list.iter().skip(1)
        .map(|block| block["type"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(types, [
        "set_visible_answer", "set_visible_answer",
        "choose_project_timer_action", "choose_project_timer_action",
        "choose_project_timer_action", "choose_project_timer_action",
        "set_visible_project_timer", "set_visible_project_timer",
        "stop_object", "stop_object", "sound_silent_all", "sound_silent_all",
    ]);
    assert_eq!(list[1]["params"][0], "SHOW");
    assert_eq!(list[2]["params"][0], "HIDE");
    assert_eq!(list[3]["params"].as_array().unwrap().len(), 4);
    assert_eq!(list[3]["params"][1], "START");
    assert_eq!(list[4]["params"][1], "STOP");
    assert_eq!(list[5]["params"][1], "RESET");
    assert_eq!(list[7]["params"][1], "SHOW");
    assert_eq!(list[8]["params"][1], "HIDE");
    assert_eq!(list[9]["params"][0], "thisThread");
    assert_eq!(list[10]["params"][0], "other_objects");
    assert_eq!(list[11]["params"].as_array().unwrap().len(), 1);
    assert_eq!(list[11]["params"][0], "all");
    assert_eq!(list[12]["params"].as_array().unwrap().len(), 1);
    assert_eq!(list[12]["params"][0], "thisOnly");
}

#[test]
fn compiles_additional_decision_and_calculation_reporters() {
    let project = compile(
        r#"
        @엔트리봇 {
            when start {
                say(is_type("123", "number"))
                say(count_match_string("banana", "a"))
                say(object_x())
                say(object_y("mouse"))
                say(object_rotation())
                say(object_direction())
                say(object_size())
                say(object_picture_index())
                say(object_picture_name())
                say(timer())
                say(answer())
            }
        }
        "#,
    );
    let result = blocks(&project);
    let list = result.as_array().unwrap();
    let reporters = list.iter().skip(1)
        .map(|block| block["params"][0]["type"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(reporters, [
        "is_type", "count_match_string", "coordinate_object", "coordinate_object",
        "coordinate_object", "coordinate_object", "coordinate_object", "coordinate_object",
        "coordinate_object", "get_project_timer_value", "get_canvas_input_value",
    ]);
    assert_eq!(list[1]["params"][2], "number");
    assert_eq!(list[2]["params"].as_array().unwrap().len(), 4);
    assert_eq!(list[3]["params"][1], "self");
    assert_eq!(list[4]["params"][1], "mouse");
    assert_eq!(list[5]["params"][3], "rotation");
    assert_eq!(list[6]["params"][3], "direction");
    assert_eq!(list[7]["params"][3], "size");
    assert_eq!(list[8]["params"][3], "picture_index");
    assert_eq!(list[9]["params"][3], "picture_name");
    assert_eq!(list[10]["params"].as_array().unwrap().len(), 2);
}

#[test]
fn uses_entry_defaults_for_dynamic_textbox_fields() {
    let project = compile(
        r#"
        @엔트리봇 {
            when start {
                text_change_font("NanumGothic")
                say(text_read())
            }
        }
        "#,
    );
    let result = blocks(&project);
    let list = result.as_array().unwrap();
    assert_eq!(list[1]["type"], "text_change_font");
    assert_eq!(list[1]["params"][0], "NanumGothic");
    assert_eq!(list[2]["params"][0]["type"], "text_read");
    assert_eq!(list[2]["params"][0]["params"][0], Value::Null);
}

#[test]
fn compiles_data_analysis_table_blocks() {
    let project = compile(
        r#"
        @엔트리봇 {
            when start {
                append_row_to_table("table-id", "ROW")
                insert_row_to_table("table-id", 2, "COL")
                delete_row_from_table("table-id", 3, "ROW")
                set_value_from_table("table-id", 2, "점수", 99)
                save_current_table("table-id")
                open_table("table-id")
                open_table_wait("table-id", 1.5)
                open_table_chart("table-id", "0")
                close_table_chart()
                set_value_from_cell("table-id", "A2", "hello")
                say(table_count("table-id", "ROW"))
                say(table_value("table-id", 2, "점수"))
                say(table_last_value("table-id", "점수"))
                say(table_stat("table-id", "점수", "AVG"))
                say(table_correlation("table-id", "점수", "시간"))
                say(table_cell("table-id", "A2"))
                say(table_lookup("table-id", "이름", "Ada", "점수"))
            }
        }
        "#,
    );

    let list = blocks(&project).as_array().unwrap().clone();
    let types = list.iter().skip(1)
        .map(|block| block["type"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(types, [
        "append_row_to_table", "insert_row_to_table", "delete_row_from_table",
        "set_value_from_table", "save_current_table", "open_table", "open_table_wait",
        "open_table_chart", "close_table_chart", "set_value_from_cell",
        "get_table_count", "get_value_from_table", "get_value_from_last_row",
        "calc_values_from_table", "get_coefficient", "get_value_from_cell",
        "get_value_v_lookup",
    ]);

    assert_eq!(list[1]["params"].as_array().unwrap().len(), 3);
    assert_eq!(list[1]["params"][0], "table-id");
    assert_eq!(list[1]["params"][1], "ROW");
    assert_eq!(list[2]["params"].as_array().unwrap().len(), 4);
    assert_eq!(list[2]["params"][0], "table-id");
    assert_eq!(list[2]["params"][2], "COL");
    assert_eq!(list[4]["params"][2]["type"], "text");
    assert_eq!(list[7]["params"][1]["params"][0], "1.5");
    assert_eq!(list[8]["params"][1], "0");
    assert_eq!(list[9]["params"].as_array().unwrap().len(), 2);
    assert_eq!(list[10]["params"][1]["params"][0], "A2");
    assert_eq!(list[11]["params"][0]["params"][1], "ROW");
    assert_eq!(list[12]["params"][0]["params"][2]["type"], "text");
    assert_eq!(list[14]["params"][0]["params"][2], "AVG");
}

#[test]
fn compiles_remaining_calculation_reporters() {
    let project = compile(
        r#"
        @엔트리봇 {
            when start {
                say(user_id())
                say(block_count("all"))
                say(rgb_to_hex(255, 128, 0))
                say(hex_to_rgb("#ff8000", "g"))
                say(boolean_value(1 < 2))
                say(fractional_part(3.14))
            }
        }
        "#,
    );

    let list = blocks(&project).as_array().unwrap();
    let reporters = list.iter().skip(1)
        .map(|block| block["params"][0]["type"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(reporters, [
        "get_user_name", "get_block_count", "change_rgb_to_hex", "change_hex_to_rgb",
        "get_boolean_value", "calc_operation",
    ]);
    assert_eq!(list[2]["params"][0]["params"][0], "all");
    assert_eq!(list[3]["params"][0]["params"].as_array().unwrap().len(), 3);
    assert_eq!(list[4]["params"][0]["params"][1], "g");
    assert_eq!(list[5]["params"][0]["params"][0]["type"], "boolean_basic_operator");
    assert_eq!(list[6]["params"][0]["params"][3], "unnatural");
}


#[test]
fn maps_sprite_rotation_and_movement_direction_rotation_separately() {
    let project = compile(
        r#"
        @엔트리봇 {
            when start {
                rotate_by_time(1, 45)
                direction_relative_duration(1, 45)
            }
        }
        "#,
    );
    let list = blocks(&project);
    let list = list.as_array().unwrap();
    assert_eq!(list[1]["type"], "rotate_by_time");
    assert_eq!(list[2]["type"], "direction_relative_duration");
}

#[test]
fn table_blocks_emit_entry_defaults_when_optional_arguments_are_omitted() {
    let project = compile(
        r#"
        @엔트리봇 {
            when start {
                insert_row_to_table("table-id")
                delete_row_from_table("table-id")
                open_table_wait("table-id")
            }
        }
        "#,
    );
    let list = blocks(&project);
    let list = list.as_array().unwrap();

    assert_eq!(list[1]["type"], "insert_row_to_table");
    assert_eq!(list[1]["params"][1]["type"], "text");
    assert!(list[1]["params"][1]["id"].is_string());
    assert_eq!(list[1]["params"][1]["params"][0], "2");
    assert_eq!(list[1]["params"][1]["statements"], serde_json::json!([]));
    assert_eq!(list[1]["params"][2], "ROW");

    assert_eq!(list[2]["type"], "delete_row_from_table");
    assert_eq!(list[2]["params"][1]["params"][0], "2");
    assert_eq!(list[2]["params"][2], "ROW");

    assert_eq!(list[3]["type"], "open_table_wait");
    assert_eq!(list[3]["params"][1]["type"], "number");
    assert!(list[3]["params"][1]["id"].is_string());
    assert_eq!(list[3]["params"][1]["params"][0], "4");
    assert_eq!(list[3]["params"][1]["statements"], serde_json::json!([]));
}


#[test]
fn repeat_until_and_repeat_while_use_the_correct_entry_dropdown() {
    let project = compile(
        r#"
        @엔트리봇 {
            when start {
                repeat_until(1 < 2) { say("until") }
                repeat_while(1 < 2) { say("while") }
                repeat_while_true(1 < 2) { say("native default") }
            }
        }
        "#,
    );
    let list = blocks(&project);
    let list = list.as_array().unwrap();
    assert_eq!(list[1]["type"], "repeat_while_true");
    assert_eq!(list[1]["params"][1], "until");
    assert_eq!(list[2]["type"], "repeat_while_true");
    assert_eq!(list[2]["params"][1], "while");
    assert_eq!(list[3]["type"], "repeat_while_true");
    assert_eq!(list[3]["params"][1], "until");
}
