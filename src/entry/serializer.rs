use std::collections::HashSet;

use rand::Rng;
use serde_json::{json, Value};

use crate::error::CompileError;

use super::input::EntryInput;
use super::ir::{EntryBlock, EntryScript};

/// Serializes Entry IR into the JSON string stored in object.script/function.content.
pub fn serialize_script(
    script: &EntryScript,
    ids: &mut IdAllocator,
) -> Result<String, CompileError> {
    let mut threads = Vec::with_capacity(script.threads.len());
    for thread in &script.threads {
        let mut blocks = Vec::with_capacity(thread.blocks.len());
        for block in &thread.blocks {
            blocks.push(serialize_block(block, ids)?);
        }
        threads.push(Value::Array(blocks));
    }
    Ok(serde_json::to_string(&Value::Array(threads))?)
}

fn serialize_block(block: &EntryBlock, ids: &mut IdAllocator) -> Result<Value, CompileError> {
    // Allocate the container block first, before any nested reporter IDs.
    let block_id = ids.next_id();
    let mut params = Vec::with_capacity(block.params.len());
    for input in &block.params {
        params.push(serialize_input(input, ids)?);
    }

    let mut statements = Vec::with_capacity(block.statements.len());
    for area in &block.statements {
        let mut nested = Vec::with_capacity(area.len());
        for child in area {
            nested.push(serialize_block(child, ids)?);
        }
        statements.push(Value::Array(nested));
    }

    Ok(make_block_json(
        block_id,
        &block.type_id,
        block.x,
        block.y,
        Value::Array(params),
        Value::Array(statements),
    ))
}

fn serialize_input(input: &EntryInput, ids: &mut IdAllocator) -> Result<Value, CompileError> {
    match input {
        EntryInput::Empty => Ok(Value::Null),
        EntryInput::Raw(value) => Ok(value.clone()),
        EntryInput::Field(value) => Ok(Value::String(value.clone())),
        EntryInput::Text(value) => Ok(make_value_block("text", json!([value]), ids)),
        EntryInput::Number(value) => Ok(make_value_block("number", json!([value]), ids)),
        EntryInput::Boolean(value) => Ok(make_value_block(
            if *value { "True" } else { "False" },
            json!([]),
            ids,
        )),
        EntryInput::Block(block) => serialize_block(block, ids),
    }
}

fn make_value_block(type_id: &str, params: Value, ids: &mut IdAllocator) -> Value {
    make_block_json(ids.next_id(), type_id, 0, 0, params, json!([]))
}

fn make_block_json(
    id: String,
    type_id: &str,
    x: i64,
    y: i64,
    params: Value,
    statements: Value,
) -> Value {
    json!({
        "id": id,
        "x": x,
        "y": y,
        "type": type_id,
        "params": params,
        "statements": statements,
        "movable": null,
        "deletable": 1,
        "emphasized": false,
        "readOnly": null,
        "copyable": true,
        "assemble": true,
        "extensions": []
    })
}

#[derive(Debug)]
pub struct IdAllocator {
    used: HashSet<String>,
}

impl IdAllocator {
    pub fn from_project(project: &Value) -> Self {
        let mut used = HashSet::new();
        collect_ids(project, &mut used);
        Self { used }
    }

    pub fn next_id(&mut self) -> String {
        const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
        loop {
            let mut rng = rand::thread_rng();
            let candidate: String = (0..4)
                .map(|_| DIGITS[rng.gen_range(0..DIGITS.len())] as char)
                .collect();
            if self.used.insert(candidate.clone()) {
                return candidate;
            }
        }
    }
}

fn collect_ids(value: &Value, used: &mut HashSet<String>) {
    match value {
        Value::Object(map) => {
            if let Some(id) = map.get("id").and_then(Value::as_str) {
                used.insert(id.to_string());
            }
            for (key, child) in map {
                if key == "script" || key == "content" {
                    if let Some(serialized) = child.as_str() {
                        if let Ok(parsed) = serde_json::from_str::<Value>(serialized) {
                            collect_ids(&parsed, used);
                        }
                    }
                }
                collect_ids(child, used);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_ids(item, used);
            }
        }
        _ => {}
    }
}
