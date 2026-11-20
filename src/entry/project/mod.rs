pub mod function;
pub mod metadata;
pub mod object;
pub mod scene;
pub mod variable;

use serde_json::Value;

pub use object::default_objects;

pub fn default_project() -> Value {
    let mut project = metadata::default_metadata();
    project["objects"] = object::default_objects();
    project["scenes"] = scene::default_scenes();
    project["variables"] = variable::default_variables();
    project["functions"] = function::default_functions();
    project
}
