pub mod analyzer;
pub mod ast;
pub mod entry;
pub mod error;
pub mod lexer;
pub mod parser;

use serde_json::Value;

use analyzer::Analyzer;
use entry::builder::EntryBuilder;
use entry::project::{default_project, object::set_script_for_object};
use entry::serializer::{serialize_script, IdAllocator};
use error::CompileError;
use lexer::Lexer;
use parser::Parser;

/// Compile Entscript into a complete Entry project JSON document.
pub fn compile_to_json(source: &str) -> Result<String, CompileError> {
    compile_with_project_value(source, default_project())
}

/// Compile Entscript into an existing Entry project JSON document.
/// Existing project data and assets are preserved; targeted scripts are replaced.
pub fn compile_with_project_json(source: &str, project_json: &str) -> Result<String, CompileError> {
    let project: Value = serde_json::from_str(project_json)?;
    compile_with_project_value(source, project)
}

/// Value-based variant for callers that already have parsed project JSON.
pub fn compile_with_project_value(
    source: &str,
    mut project: Value,
) -> Result<String, CompileError> {
    let tokens = Lexer::new(source).tokenize()?;
    let ast = Parser::new(tokens).parse()?;
    Analyzer::analyze(&ast, &project)?;

    let mut ids = IdAllocator::from_project(&project);
    let built = EntryBuilder::build(&ast, &mut project, &mut ids)?;

    for (object_name, script_ir) in built.scripts {
        let script_json = serialize_script(&script_ir, &mut ids)?;
        set_script_for_object(&mut project, &object_name, script_json)?;
    }

    if project.get("functions").is_none() {
        project["functions"] = serde_json::json!([]);
    }
    let project_functions = project.get_mut("functions")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| CompileError::Project("Project 'functions' must be an array".into()))?;
    project_functions.extend(built.functions);
    // Entry expects `objects` at the beginning of the project document.
    // Preserve the relative order of every other top-level field.
    let project = match project {
        Value::Object(mut fields) => {
            let mut ordered = serde_json::Map::new();
            if let Some(objects) = fields.remove("objects") {
                ordered.insert("objects".to_owned(), objects);
            }
            ordered.extend(fields);
            Value::Object(ordered)
        }
        other => other,
    };

    Ok(serde_json::to_string(&project)?)
}

#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
pub mod wasm_api {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub fn compile(source: &str) -> Result<String, JsValue> {
        crate::compile_to_json(source).map_err(|error| JsValue::from_str(&error.to_string()))
    }

    #[wasm_bindgen]
    pub fn compile_with_project(source: &str, project_json: &str) -> Result<String, JsValue> {
        crate::compile_with_project_json(source, project_json)
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }
}
