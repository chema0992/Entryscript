use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamEncoding {
    /// An Entry reporter/value block is stored at this params index.
    ValueBlock,
    /// A raw field/dropdown value is stored at this params index.
    Raw,
}

#[derive(Debug, Clone)]
pub enum ParamSlot {
    Input {
        encoding: ParamEncoding,
        default: Option<Value>,
    },
    Fixed(Value),
}

#[derive(Debug, Clone)]
pub struct BlockSpec {
    pub type_id: &'static str,
    pub params: Vec<ParamSlot>,
    pub statement_count: usize,
    pub x: i64,
    pub y: i64,
}

fn input(encoding: ParamEncoding) -> ParamSlot {
    ParamSlot::Input { encoding, default: None }
}

fn input_default(encoding: ParamEncoding, value: Value) -> ParamSlot {
    ParamSlot::Input { encoding, default: Some(value) }
}

fn fixed(value: Value) -> ParamSlot {
    ParamSlot::Fixed(value)
}

fn spec(type_id: &'static str, params: Vec<ParamSlot>, statement_count: usize) -> BlockSpec {
    BlockSpec { type_id, params, statement_count, x: 0, y: 0 }
}

fn event_spec_by_name(name: &str) -> Option<BlockSpec> {
    let null = || fixed(Value::Null);
    match name {
        "start" => {
            let mut value = spec("when_run_button_click", vec![null()], 0);
            value.x = 50;
            value.y = 30;
            Some(value)
        }
        "key" | "key_pressed" => Some(spec(
            "when_some_key_pressed",
            vec![null(), input_default(ParamEncoding::Raw, json!("81"))],
            0,
        )),
        "mouse_click" => Some(spec("mouse_clicked", vec![null()], 0)),
        "mouse_release" => Some(spec("mouse_click_cancled", vec![null()], 0)),
        "object_click" => Some(spec("when_object_click", vec![null()], 0)),
        "object_release" => Some(spec("when_object_click_canceled", vec![null()], 0)),
        "message" => Some(spec(
            "when_message_cast",
            vec![null(), input(ParamEncoding::Raw)],
            0,
        )),
        "scene_start" => Some(spec("when_scene_start", vec![null()], 0)),
        "clone_start" => Some(spec("when_clone_start", vec![null()], 0)),
        _ => None,
    }
}

pub fn event_spec(name: &str) -> Option<BlockSpec> {
    event_spec_by_name(name).or_else(|| {
        let null = || fixed(Value::Null);
        match name {
            "when_run_button_click" => Some(spec("when_run_button_click", vec![null()], 0)),
            "when_some_key_pressed" => Some(spec(
                "when_some_key_pressed",
                vec![null(), input_default(ParamEncoding::Raw, json!("81"))],
                0,
            )),
            "mouse_clicked" => Some(spec("mouse_clicked", vec![null()], 0)),
            "mouse_click_cancled" => Some(spec("mouse_click_cancled", vec![null()], 0)),
            "when_object_click" => Some(spec("when_object_click", vec![null()], 0)),
            "when_object_click_canceled" => Some(spec("when_object_click_canceled", vec![null()], 0)),
            "when_scene_start" => Some(spec("when_scene_start", vec![null()], 0)),
            "when_clone_start" => Some(spec("when_clone_start", vec![null()], 0)),
            "when_message_cast" => Some(spec(
                "when_message_cast",
                vec![null(), input(ParamEncoding::Raw)],
                0,
            )),
            _ => None,
        }
    })
}

/// Friendly Entscript names and recognized native type IDs for statement blocks.
/// Unregistered native blocks must use raw_block("type_id", ...).
pub fn statement_spec(name: &str) -> Option<BlockSpec> {
    use ParamEncoding::{Raw, ValueBlock};
    let null = || fixed(Value::Null);

    let result = match name {
        // Flow: https://github.com/entrylabs/entryjs/.../block_flow.js
        "wait" | "wait_second" => spec("wait_second", vec![input(ValueBlock), null()], 0),
        "for" | "repeat" | "repeat_basic" => {
            spec("repeat_basic", vec![input(ValueBlock), null()], 1)
        }
        // Entry's current default definition has one params slot for repeat_inf.
        "loop" | "repeat_inf" => spec("repeat_inf", vec![null()], 1),
        "repeat_until" | "repeat_until_true" | "repeat_while_true" => spec(
            "repeat_while_true",
            vec![input(ValueBlock), input_default(Raw, json!("until")), null()],
            1,
        ),
        "repeat_while" => spec(
            "repeat_while_true",
            vec![input(ValueBlock), input_default(Raw, json!("while")), null()],
            1,
        ),
        "break" | "stop_repeat" => spec("stop_repeat", vec![null()], 0),
        "continue" | "continue_repeat" => spec("continue_repeat", vec![null()], 0),
        "if" | "_if" => spec("_if", vec![input(ValueBlock), null()], 1),
        "if_else" => spec("if_else", vec![input(ValueBlock), null()], 2),
        "wait_until" | "wait_until_true" => {
            spec("wait_until_true", vec![input(ValueBlock), null()], 0)
        }
        "stop" | "stop_object" => spec(
            "stop_object",
            vec![input_default(Raw, json!("all")), null()],
            0,
        ),
        "stop_all" | "stop_all_scripts" => spec(
            "stop_object", vec![fixed(json!("all")), null()], 0,
        ),
        "stop_this_object" => spec(
            "stop_object", vec![fixed(json!("thisOnly")), null()], 0,
        ),
        "stop_this_thread" => spec(
            "stop_object", vec![fixed(json!("thisThread")), null()], 0,
        ),
        "stop_other_threads" => spec(
            "stop_object", vec![fixed(json!("otherThread")), null()], 0,
        ),
        "stop_other_objects" => spec(
            "stop_object", vec![fixed(json!("other_objects")), null()], 0,
        ),
        "restart" | "restart_project" => spec("restart_project", vec![null()], 0),
        "send_message" | "message_cast" => {
            spec("message_cast", vec![input(Raw), null()], 0)
        }
        "send_message_and_wait" | "message_cast_wait" => {
            spec("message_cast_wait", vec![input(Raw), null()], 0)
        }
        "start_scene" => spec("start_scene", vec![input(Raw), null()], 0),
        "start_neighbor_scene" => spec(
            "start_neighbor_scene",
            vec![input_default(Raw, Value::Null), null()],
            0,
        ),
        "create_clone" => spec(
            "create_clone",
            vec![input_default(Raw, Value::Null), null()],
            0,
        ),
        "delete_clone" => spec("delete_clone", vec![null()], 0),
        "delete_all_clones" | "remove_all_clones" => {
            spec("remove_all_clones", vec![null()], 0)
        }

        // Looks.
        "say" | "dialog" => spec(
            "dialog",
            vec![input(ValueBlock), fixed(json!("speak")), null()],
            0,
        ),
        "say_for" | "dialog_time" => spec(
            "dialog_time",
            vec![input(ValueBlock), input(ValueBlock), null(), null()],
            0,
        ),
        "think_for" => spec(
            "dialog_time",
            vec![input(ValueBlock), input(ValueBlock), fixed(json!("think")), null()],
            0,
        ),
        "think" => spec(
            "dialog",
            vec![input(ValueBlock), fixed(json!("think")), null()],
            0,
        ),
        "clear_dialog" | "remove_dialog" => spec("remove_dialog", vec![null()], 0),
        "show" => spec("show", vec![null()], 0),
        "hide" => spec("hide", vec![null()], 0),
        "ask" | "ask_and_wait" => spec("ask_and_wait", vec![input(ValueBlock), null()], 0),
        "show_answer" => spec(
            "set_visible_answer", vec![fixed(json!("SHOW")), null()], 0,
        ),
        "hide_answer" => spec(
            "set_visible_answer", vec![fixed(json!("HIDE")), null()], 0,
        ),
        "set_visible_answer" | "answer_visibility" => spec(
            "set_visible_answer", vec![input_default(Raw, json!("HIDE")), null()], 0,
        ),
        "next_shape" | "change_to_next_shape" => spec(
            "change_to_next_shape", vec![fixed(Value::Null), null()], 0,
        ),
        "previous_shape" | "prev_shape" => spec(
            "change_to_next_shape", vec![fixed(json!("prev")), null()], 0,
        ),
        "change_shape" | "change_to_some_shape" => spec(
            "change_to_some_shape",
            vec![input(ValueBlock), null()],
            0,
        ),
        "erase_effects" | "erase_all_effects" => {
            spec("erase_all_effects", vec![null()], 0)
        }
        "add_effect" | "add_effect_amount" => spec(
            "add_effect_amount", vec![input(Raw), input(ValueBlock), null()], 0,
        ),
        "set_effect" | "change_effect" | "change_effect_amount" => spec(
            "change_effect_amount", vec![input(Raw), input(ValueBlock), null()], 0,
        ),
        "change_size" | "add_size" | "change_scale_size" => {
            spec("change_scale_size", vec![input(ValueBlock), null()], 0)
        }
        "set_size" | "set_scale_size" => {
            spec("set_scale_size", vec![input(ValueBlock), null()], 0)
        }
        "stretch_size" | "stretch_scale_size" => {
            spec("stretch_scale_size", vec![input(Raw), input(ValueBlock), null()], 0)
        }
        "reset_size" | "reset_scale_size" => spec("reset_scale_size", vec![null()], 0),
        "flip_x" => spec("flip_x", vec![null()], 0),
        "flip_y" => spec("flip_y", vec![null()], 0),
        "change_layer" | "change_object_index" => spec(
            "change_object_index", vec![input(Raw), null()], 0,
        ),

        // Project timer and answer visibility.
        "timer_action" | "choose_project_timer_action" => spec(
            "choose_project_timer_action",
            vec![null(), input_default(Raw, json!("START")), null(), null()],
            0,
        ),
        "timer_start" | "start_timer" => spec(
            "choose_project_timer_action", vec![null(), fixed(json!("START")), null(), null()], 0,
        ),
        "timer_stop" | "pause_timer" => spec(
            "choose_project_timer_action", vec![null(), fixed(json!("STOP")), null(), null()], 0,
        ),
        "timer_reset" | "reset_timer" => spec(
            "choose_project_timer_action", vec![null(), fixed(json!("RESET")), null(), null()], 0,
        ),
        "set_project_timer_visibility" | "set_visible_project_timer" => spec(
            "set_visible_project_timer",
            vec![null(), input_default(Raw, json!("HIDE")), null(), null()],
            0,
        ),
        "show_project_timer" => spec(
            "set_visible_project_timer", vec![null(), fixed(json!("SHOW")), null(), null()], 0,
        ),
        "hide_project_timer" => spec(
            "set_visible_project_timer", vec![null(), fixed(json!("HIDE")), null(), null()], 0,
        ),

        // Motion: common, stable block mappings. Use raw_block for other slots.
        "move" | "move_direction" => spec("move_direction", vec![input(ValueBlock), null()], 0),
        "bounce" | "bounce_wall" => spec("bounce_wall", vec![null()], 0),
        "change_x" | "move_x" => spec("move_x", vec![input(ValueBlock), null()], 0),
        "change_y" | "move_y" => spec("move_y", vec![input(ValueBlock), null()], 0),
        "set_x" | "locate_x" => spec("locate_x", vec![input(ValueBlock), null()], 0),
        "set_y" | "locate_y" => spec("locate_y", vec![input(ValueBlock), null()], 0),
        "go_to_xy" | "locate_xy" => {
            spec("locate_xy", vec![input(ValueBlock), input(ValueBlock), null()], 0)
        }
        "go_to" | "locate" | "move_to_object" => spec("locate", vec![input(Raw), null()], 0),
        "go_to_mouse" | "move_to_mouse" => spec(
            "locate", vec![fixed(json!("mouse")), null()], 0,
        ),
        "set_direction" | "direction_absolute" => {
            spec("direction_absolute", vec![input(ValueBlock), null()], 0)
        }
        "turn" | "rotate_relative" => {
            spec("rotate_relative", vec![input(ValueBlock), null()], 0)
        }
        "turn_direction" | "change_direction" | "direction_relative" => {
            spec("direction_relative", vec![input(ValueBlock), null()], 0)
        }
        "rotate_absolute" | "set_rotation" => {
            spec("rotate_absolute", vec![input(ValueBlock), null()], 0)
        }
        // Entry has two distinct timed-rotation blocks: one rotates the
        // sprite itself, the other changes its movement direction.
        "rotate_by_time" => spec(
            "rotate_by_time",
            vec![input(ValueBlock), input(ValueBlock), null()],
            0,
        ),
        "direction_relative_duration" | "rotate_direction_by_time" => spec(
            "direction_relative_duration",
            vec![input(ValueBlock), input(ValueBlock), null()],
            0,
        ),
        "move_xy_time" | "move_xy_for" => spec(
            "move_xy_time",
            vec![input(ValueBlock), input(ValueBlock), input(ValueBlock), null()],
            0,
        ),
        "locate_xy_time" | "locate_xy_for" => spec(
            "locate_xy_time",
            vec![input(ValueBlock), input(ValueBlock), input(ValueBlock), null()],
            0,
        ),
        "locate_object_time" | "go_to_object_for" => spec(
            "locate_object_time",
            vec![input(ValueBlock), input(Raw), null()],
            0,
        ),
        "see_angle_object" | "point_towards" => spec(
            "see_angle_object", vec![input(Raw), null()], 0,
        ),
        "point_towards_mouse" => spec(
            "see_angle_object", vec![fixed(json!("mouse")), null()], 0,
        ),
        "move_to_angle" | "move_direction_distance" => spec(
            "move_to_angle",
            vec![input(ValueBlock), input(ValueBlock), null()],
            0,
        ),

        // Brush.
        "stamp" | "brush_stamp" => spec("brush_stamp", vec![null()], 0),
        "start_drawing" => spec("start_drawing", vec![null()], 0),
        "stop_drawing" => spec("stop_drawing", vec![null()], 0),
        "start_fill" => spec("start_fill", vec![null()], 0),
        "stop_fill" => spec("stop_fill", vec![null()], 0),
        "set_brush_color" | "set_color" => {
            spec("set_color", vec![input(ValueBlock), null()], 0)
        }
        "set_random_color" | "random_brush_color" => {
            spec("set_random_color", vec![null()], 0)
        }
        "set_fill_color" | "set_paint_color" => {
            spec("set_fill_color", vec![input(ValueBlock), null()], 0)
        }
        "change_thickness" | "change_brush_thickness" => {
            spec("change_thickness", vec![input(ValueBlock), null()], 0)
        }
        "set_thickness" | "set_brush_thickness" => {
            spec("set_thickness", vec![input(ValueBlock), null()], 0)
        }
        "change_brush_transparency" | "change_brush_opacity" => {
            spec("change_brush_transparency", vec![input(ValueBlock), null()], 0)
        }
        // The native Entry type ID contains this spelling (tranparency).
        "set_brush_transparency" | "set_brush_tranparency" => {
            spec("set_brush_tranparency", vec![input(ValueBlock), null()], 0)
        }
        "clear_brush" | "erase_all_brush" | "brush_erase_all" => {
            spec("brush_erase_all", vec![null()], 0)
        }

        // Text box objects only.
        "text_write" | "write_text" => {
            spec("text_write", vec![input(ValueBlock), null()], 0)
        }
        "text_append" | "append_text" => {
            spec("text_append", vec![input(ValueBlock), null()], 0)
        }
        "text_prepend" | "prepend_text" => {
            spec("text_prepend", vec![input(ValueBlock), null()], 0)
        }
        "text_change_effect" | "set_text_effect" => spec(
            "text_change_effect",
            vec![input_default(Raw, json!("strike")), input_default(Raw, json!("on"))],
            0,
        ),
        "text_change_font" | "set_text_font" => {
            // Entry fills this dynamic font dropdown from its font menu.
            spec("text_change_font", vec![input_default(Raw, Value::Null)], 0)
        }
        "text_change_font_color" | "set_text_color" => {
            spec("text_change_font_color", vec![input(ValueBlock), null()], 0)
        }
        "text_change_bg_color" | "set_text_background" => {
            spec("text_change_bg_color", vec![input(ValueBlock), null()], 0)
        }
        "text_flush" | "flush_text" => spec("text_flush", vec![null()], 0),

        // Sound.
        "play_sound" | "sound_something_with_block" => spec(
            "sound_something_with_block",
            vec![input(ValueBlock), null()],
            0,
        ),
        "play_sound_for" | "sound_something_second_with_block" => spec(
            "sound_something_second_with_block",
            vec![input(ValueBlock), input(ValueBlock), null()],
            0,
        ),
        "play_sound_from_to" | "sound_from_to" => spec(
            "sound_from_to",
            vec![input(ValueBlock), input(ValueBlock), input(ValueBlock), null()],
            0,
        ),
        "play_sound_until_done" | "sound_something_wait_with_block" => spec(
            "sound_something_wait_with_block",
            vec![input(ValueBlock), null()],
            0,
        ),
        "play_sound_for_and_wait" | "sound_something_second_wait_with_block" => spec(
            "sound_something_second_wait_with_block",
            vec![input(ValueBlock), input(ValueBlock), null()],
            0,
        ),
        "play_sound_from_to_and_wait" | "sound_from_to_and_wait" => spec(
            "sound_from_to_and_wait",
            vec![input(ValueBlock), input(ValueBlock), input(ValueBlock), null()],
            0,
        ),
        "change_sound_volume" | "sound_volume_change" => {
            spec("sound_volume_change", vec![input(ValueBlock), null()], 0)
        }
        "set_sound_volume" | "sound_volume_set" => {
            spec("sound_volume_set", vec![input(ValueBlock), null()], 0)
        }
        "stop_all_sounds" => spec(
            "sound_silent_all", vec![fixed(json!("all"))], 0,
        ),
        "stop_sounds" | "sound_silent_all" => spec(
            "sound_silent_all", vec![input_default(Raw, json!("all"))], 0,
        ),
        "stop_this_object_sounds" => spec(
            "sound_silent_all", vec![fixed(json!("thisOnly"))], 0,
        ),
        "stop_other_objects_sounds" => spec(
            "sound_silent_all", vec![fixed(json!("other_objects"))], 0,
        ),
        "play_bgm" => spec("play_bgm", vec![input(ValueBlock), null()], 0),
        "stop_bgm" => spec("stop_bgm", vec![null()], 0),
        "change_sound_speed" | "sound_speed_change" => {
            spec("sound_speed_change", vec![input(ValueBlock), null()], 0)
        }
        "set_sound_speed" | "sound_speed_set" => {
            spec("sound_speed_set", vec![input(ValueBlock), null()], 0)
        }

        // Data analysis tables. The first argument is the Entry table ID from
        // the source project's `tables` metadata. Column selectors are value
        // inputs because Entry supplies them through get_table_fields blocks.
        "table_append" | "append_table_row" | "append_row_to_table" => spec(
            "append_row_to_table",
            vec![input(Raw), input_default(Raw, json!("ROW")), null()],
            0,
        ),
        "table_insert" | "insert_table_row" | "insert_row_to_table" => spec(
            "insert_row_to_table",
            vec![
                input(Raw),
                input_default(ValueBlock, json!({"type": "text", "params": ["2"]})),
                input_default(Raw, json!("ROW")),
                null(),
            ],
            0,
        ),
        "table_delete" | "delete_table_row" | "delete_row_from_table" => spec(
            "delete_row_from_table",
            vec![
                input(Raw),
                input_default(ValueBlock, json!({"type": "text", "params": ["2"]})),
                input_default(Raw, json!("ROW")),
                null(),
            ],
            0,
        ),
        "table_set" | "set_table_value" | "set_value_from_table" => spec(
            "set_value_from_table",
            vec![input(Raw), input(ValueBlock), input(ValueBlock), input(ValueBlock), null()],
            0,
        ),
        "table_save" | "preserve_table" | "save_current_table" => spec(
            "save_current_table", vec![input(Raw), null()], 0,
        ),
        "table_open" | "open_table" => spec(
            "open_table", vec![input(Raw), null(), null()], 0,
        ),
        "table_open_for" | "open_table_wait" => spec(
            "open_table_wait",
            vec![
                input(Raw),
                input_default(ValueBlock, json!({"type": "number", "params": ["4"]})),
                null(),
            ],
            0,
        ),
        "table_open_chart" | "open_table_chart" => spec(
            "open_table_chart", vec![input(Raw), input_default(Raw, json!("0")), null()], 0,
        ),
        "table_close_chart" | "close_table_chart" => spec(
            "close_table_chart", vec![null(), null()], 0,
        ),
        "table_set_cell" | "set_value_from_cell" => spec(
            "set_value_from_cell", vec![input(Raw), input(ValueBlock), input(ValueBlock), null()], 0,
        ),

        _ => return None,
    };
    Some(result)
}

/// Friendly reporter aliases. The complete native reporter set is also
/// available through its Entry type ID and raw_block(...).
pub fn reporter_spec(name: &str) -> Option<BlockSpec> {
    use ParamEncoding::{Raw, ValueBlock};
    match name {
        "random" | "rand" | "calc_rand" => Some(spec(
            "calc_rand",
            vec![
                fixed(Value::Null),
                input(ValueBlock),
                fixed(Value::Null),
                input(ValueBlock),
                fixed(Value::Null),
            ],
            0,
        )),
        "mouse_x" => Some(spec(
            "coordinate_mouse",
            vec![fixed(Value::Null), fixed(json!("x")), fixed(Value::Null)],
            0,
        )),
        "mouse_y" => Some(spec(
            "coordinate_mouse",
            vec![fixed(Value::Null), fixed(json!("y")), fixed(Value::Null)],
            0,
        )),
        "clicked" | "is_clicked" => Some(spec("is_clicked", vec![fixed(Value::Null)], 0)),
        "is_boost_mode" => Some(spec("is_boost_mode", vec![fixed(Value::Null)], 0)),
        "get_project_timer_value" | "timer" | "stopwatch" => Some(spec(
            "get_project_timer_value", vec![fixed(Value::Null), fixed(Value::Null)], 0,
        )),
        "is_object_clicked" => Some(spec("is_object_clicked", vec![fixed(Value::Null)], 0)),
        "key_down" | "is_press_some_key" => Some(spec(
            "is_press_some_key",
            vec![input_default(Raw, json!("81")), fixed(Value::Null)],
            0,
        )),
        "touching" | "reach_something" => Some(spec(
            "reach_something",
            vec![fixed(Value::Null), input(Raw), fixed(Value::Null)],
            0,
        )),
        "object_x" | "x_of_object" => Some(spec(
            "coordinate_object",
            vec![fixed(Value::Null), input_default(Raw, json!("self")), fixed(Value::Null), fixed(json!("x"))],
            0,
        )),
        "object_y" | "y_of_object" => Some(spec(
            "coordinate_object",
            vec![fixed(Value::Null), input_default(Raw, json!("self")), fixed(Value::Null), fixed(json!("y"))],
            0,
        )),
        "object_rotation" | "rotation_of_object" => Some(spec(
            "coordinate_object",
            vec![fixed(Value::Null), input_default(Raw, json!("self")), fixed(Value::Null), fixed(json!("rotation"))],
            0,
        )),
        "object_direction" | "direction_of_object" => Some(spec(
            "coordinate_object",
            vec![fixed(Value::Null), input_default(Raw, json!("self")), fixed(Value::Null), fixed(json!("direction"))],
            0,
        )),
        "object_size" | "size_of_object" => Some(spec(
            "coordinate_object",
            vec![fixed(Value::Null), input_default(Raw, json!("self")), fixed(Value::Null), fixed(json!("size"))],
            0,
        )),
        "object_picture_index" | "picture_index_of_object" => Some(spec(
            "coordinate_object",
            vec![fixed(Value::Null), input_default(Raw, json!("self")), fixed(Value::Null), fixed(json!("picture_index"))],
            0,
        )),
        "object_picture_name" | "picture_name_of_object" => Some(spec(
            "coordinate_object",
            vec![fixed(Value::Null), input_default(Raw, json!("self")), fixed(Value::Null), fixed(json!("picture_name"))],
            0,
        )),
        "object_property" | "coordinate_object" => Some(spec(
            "coordinate_object",
            vec![fixed(Value::Null), input_default(Raw, json!("self")), fixed(Value::Null), input(Raw)],
            0,
        )),
        "calc_basic" => Some(spec(
            "calc_basic",
            vec![input(ValueBlock), input_default(Raw, json!("PLUS")), input(ValueBlock)],
            0,
        )),
        "boolean_basic_operator" => Some(spec(
            "boolean_basic_operator",
            vec![input(ValueBlock), input_default(Raw, json!("EQUAL")), input(ValueBlock)],
            0,
        )),
        "boolean_and_or" => Some(spec(
            "boolean_and_or",
            vec![input(ValueBlock), input_default(Raw, json!("AND")), input(ValueBlock)],
            0,
        )),
        "boolean_not" => Some(spec(
            "boolean_not",
            vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null)],
            0,
        )),
        "sqrt" | "root" | "math_sqrt" => Some(spec(
            "calc_operation", vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), fixed(json!("root"))], 0,
        )),
        "square" => Some(spec(
            "calc_operation", vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), fixed(json!("square"))], 0,
        )),
        "abs" | "absolute" => Some(spec(
            "calc_operation", vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), fixed(json!("abs"))], 0,
        )),
        "floor" => Some(spec(
            "calc_operation", vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), fixed(json!("floor"))], 0,
        )),
        "ceil" => Some(spec(
            "calc_operation", vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), fixed(json!("ceil"))], 0,
        )),
        "round" => Some(spec(
            "calc_operation", vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), fixed(json!("round"))], 0,
        )),
        "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "log10" | "ln" | "factorial" => {
            let operation = match name {
                "log10" => "log",
                "asin" => "asin_radian",
                "acos" => "acos_radian",
                "atan" => "atan_radian",
                other => other,
            };
            Some(spec(
                "calc_operation", vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), fixed(json!(operation))], 0,
            ))
        }
        "calc_operation" => Some(spec(
            "calc_operation",
            vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), input(Raw)],
            0,
        )),
        // String and text-box reporters.
        "text_read" | "read_text" | "text_box_value" => Some(spec(
            "text_read",
            vec![input_default(Raw, Value::Null), fixed(Value::Null)],
            0,
        )),
        "string_length" | "length_of_string" => Some(spec(
            "length_of_string",
            vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null)],
            0,
        )),
        "reverse_string" | "reverse_of_string" => Some(spec(
            "reverse_of_string",
            vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null)],
            0,
        )),
        "combine" | "combine_text" | "combine_something" => Some(spec(
            "combine_something",
            vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), input(ValueBlock), fixed(Value::Null)],
            0,
        )),
        "char_at" | "string_char_at" => Some(spec(
            "char_at",
            vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), input(ValueBlock), fixed(Value::Null)],
            0,
        )),
        "substring" | "string_slice" => Some(spec(
            "substring",
            vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), input(ValueBlock), fixed(Value::Null), input(ValueBlock), fixed(Value::Null)],
            0,
        )),
        "index_of" | "index_of_string" | "string_index_of" => Some(spec(
            "index_of_string",
            vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), input(ValueBlock), fixed(Value::Null)],
            0,
        )),
        "replace_string" | "string_replace" => Some(spec(
            "replace_string",
            vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), input(ValueBlock), fixed(Value::Null), input(ValueBlock), fixed(Value::Null)],
            0,
        )),
        "uppercase" => Some(spec(
            "change_string_case",
            vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), fixed(json!("toUpperCase")), fixed(Value::Null)],
            0,
        )),
        "lowercase" => Some(spec(
            "change_string_case",
            vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), fixed(json!("toLowerCase")), fixed(Value::Null)],
            0,
        )),
        "change_string_case" => Some(spec(
            "change_string_case",
            vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), input_default(Raw, json!("toUpperCase")), fixed(Value::Null)],
            0,
        )),
        "get_user_name" | "username" | "user_id" => Some(spec("get_user_name", vec![], 0)),
        "block_count" | "get_block_count" => Some(spec(
            "get_block_count", vec![input(Raw)], 0,
        )),
        "rgb_to_hex" | "change_rgb_to_hex" => Some(spec(
            "change_rgb_to_hex", vec![input(ValueBlock), input(ValueBlock), input(ValueBlock)], 0,
        )),
        "hex_to_rgb" | "change_hex_to_rgb" => Some(spec(
            "change_hex_to_rgb", vec![input(ValueBlock), input_default(Raw, json!("r"))], 0,
        )),
        "boolean_value" | "get_boolean_value" => Some(spec(
            "get_boolean_value", vec![input(ValueBlock)], 0,
        )),
        "current_device_type" | "is_current_device_type" => Some(spec(
            "is_current_device_type", vec![input_default(Raw, json!("desktop"))], 0,
        )),
        "get_nickname" | "nickname" => Some(spec("get_nickname", vec![], 0)),
        "get_date" | "date_part" => Some(spec(
            "get_date",
            vec![fixed(Value::Null), input_default(Raw, json!("YEAR")), fixed(Value::Null)],
            0,
        )),
        "distance" | "distance_to" | "distance_something" => Some(spec(
            "distance_something",
            vec![fixed(Value::Null), input(Raw), fixed(Value::Null)],
            0,
        )),
        "quotient" => Some(spec(
            "quotient_and_mod",
            vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), input(ValueBlock), fixed(Value::Null), input_default(Raw, json!("QUOTIENT"))],
            0,
        )),
        "remainder" | "modulo" => Some(spec(
            "quotient_and_mod",
            vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), input(ValueBlock), fixed(Value::Null), input_default(Raw, json!("MOD"))],
            0,
        )),
        "fractional_part" | "decimal_part" => Some(spec(
            "calc_operation", vec![fixed(Value::Null), input(ValueBlock), fixed(Value::Null), fixed(json!("unnatural"))], 0,
        )),
        "table_count" | "get_table_count" => Some(spec(
            "get_table_count", vec![input(Raw), input_default(Raw, json!("ROW"))], 0,
        )),
        "table_value" | "get_value_from_table" => Some(spec(
            "get_value_from_table", vec![input(Raw), input(ValueBlock), input(ValueBlock)], 0,
        )),
        "table_last_value" | "get_value_from_last_row" => Some(spec(
            "get_value_from_last_row", vec![input(Raw), input(ValueBlock)], 0,
        )),
        "table_stat" | "table_statistics" | "calc_values_from_table" => Some(spec(
            "calc_values_from_table", vec![input(Raw), input(ValueBlock), input_default(Raw, json!("SUM"))], 0,
        )),
        "table_correlation" | "get_coefficient" => Some(spec(
            "get_coefficient", vec![input(Raw), input(ValueBlock), input(ValueBlock)], 0,
        )),
        "table_cell" | "get_value_from_cell" => Some(spec(
            "get_value_from_cell", vec![input(Raw), input(ValueBlock)], 0,
        )),
        "table_lookup" | "get_value_v_lookup" => Some(spec(
            "get_value_v_lookup", vec![input(Raw), input(ValueBlock), input(ValueBlock), input(ValueBlock)], 0,
        )),
        "get_variable" => Some(spec("get_variable", vec![input_default(Raw, Value::Null)], 0)),
        "sound" | "get_sounds" => Some(spec("get_sounds", vec![input(Raw)], 0)),
        "picture" | "get_pictures" => Some(spec("get_pictures", vec![input(Raw)], 0)),
        "get_canvas_input_value" | "get_answer" | "answer" => Some(spec(
            "get_canvas_input_value", vec![fixed(Value::Null)], 0,
        )),
        "get_sound_volume" => Some(spec("get_sound_volume", vec![fixed(Value::Null)], 0)),
        "get_sound_speed" => Some(spec("get_sound_speed", vec![fixed(Value::Null)], 0)),
        "get_sound_duration" => Some(spec(
            "get_sound_duration",
            vec![fixed(Value::Null), input_default(Raw, Value::Null), fixed(Value::Null)],
            0,
        )),
        "is_type" | "is_value_type" => Some(spec(
            "is_type",
            vec![input(ValueBlock), fixed(Value::Null), input_default(Raw, json!("number")), fixed(Value::Null)],
            0,
        )),
        "count_match_string" | "count_substring" => Some(spec(
            "count_match_string",
            vec![input(ValueBlock), fixed(Value::Null), input(ValueBlock), fixed(Value::Null)],
            0,
        )),
        _ => None,
    }
}

/// Known blocks for which Entscript can construct the standard params layout.
/// Unknown native types remain available through raw_block with exact params.
pub fn block_spec_by_type(type_id: &str) -> Option<BlockSpec> {
    event_spec(type_id)
        .or_else(|| statement_spec(type_id))
        .or_else(|| reporter_spec(type_id))
        .or_else(|| match type_id {
            "calc_basic" => Some(spec(
                "calc_basic",
                vec![
                    input(ParamEncoding::ValueBlock),
                    input_default(ParamEncoding::Raw, json!("PLUS")),
                    input(ParamEncoding::ValueBlock),
                ],
                0,
            )),
            "boolean_basic_operator" => Some(spec(
                "boolean_basic_operator",
                vec![
                    input(ParamEncoding::ValueBlock),
                    input_default(ParamEncoding::Raw, json!("EQUAL")),
                    input(ParamEncoding::ValueBlock),
                ],
                0,
            )),
            "boolean_and_or" => Some(spec(
                "boolean_and_or",
                vec![
                    input(ParamEncoding::ValueBlock),
                    input_default(ParamEncoding::Raw, json!("AND")),
                    input(ParamEncoding::ValueBlock),
                ],
                0,
            )),
            "boolean_not" => Some(spec(
                "boolean_not",
                vec![fixed(Value::Null), input(ParamEncoding::ValueBlock), fixed(Value::Null)],
                0,
            )),
            _ => None,
        })
}

pub fn is_known_statement(name: &str) -> bool {
    statement_spec(name).is_some()
}

/// Number of required caller-supplied arguments. Inputs with defaults are
/// optional; they are useful for fields such as the default stop target.
pub fn required_input_count(params: &[ParamSlot]) -> usize {
    params
        .iter()
        .filter(|param| matches!(param, ParamSlot::Input { default: None, .. }))
        .count()
}

/// Maximum caller-supplied argument count for a spec.
pub fn input_count(params: &[ParamSlot]) -> usize {
    params.iter().filter(|param| matches!(param, ParamSlot::Input { .. })).count()
}
