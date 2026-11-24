use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::ast::{Expression, FunctionDefinition, FunctionParameterKind, Program, Statement, UnaryOp, BinaryOp, VariableKind};
use crate::entry::registry::{
    event_spec, input_count, reporter_spec, required_input_count, statement_spec, ParamSlot,
};
use crate::error::CompileError;

pub struct Analyzer;

impl Analyzer {
    pub fn analyze(program: &Program, project: &Value) -> Result<(), CompileError> {
        Self::validate_project_collections(project)?;
        Self::validate_declarations(program, project)?;

        let objects = project
            .get("objects")
            .and_then(Value::as_array)
            .ok_or_else(|| CompileError::Project("Project must contain an 'objects' array".into()))?;

        let mut declared_sections = HashSet::new();
        for section in &program.objects {
            if !declared_sections.insert(section.name.clone()) {
                return Err(CompileError::Semantic(format!(
                    "Object '@{}' is declared more than once", section.name
                )));
            }

            let matches = objects.iter()
                .filter(|object| object.get("name").and_then(Value::as_str) == Some(section.name.as_str()))
                .count();
            match matches {
                0 => return Err(CompileError::Semantic(format!(
                    "Object '@{}' does not exist in the Entry project", section.name
                ))),
                1 => {}
                _ => return Err(CompileError::Semantic(format!(
                    "Object name '{}' is ambiguous", section.name
                ))),
            }

            let empty_parameters = HashMap::new();
            for handler in &section.handlers {
                if let Some(spec) = event_spec(&handler.event) {
                    Self::check_argument_count(&handler.event, &handler.args, &spec.params)?;
                } else {
                    Self::ensure_builtin_type(&handler.event)?;
                }
                for argument in &handler.args {
                    Self::analyze_expression(
                        argument, project, program, &empty_parameters, Some(&section.name),
                    )?;
                }
                Self::analyze_statements(
                    &handler.body, project, program, &empty_parameters, Some(&section.name),
                )?;
            }
        }

        for function in &program.functions {
            let param_kinds = function.params.iter()
                .map(|param| (param.name.clone(), param.kind))
                .collect::<HashMap<_, _>>();
            if let Some(value) = &function.return_value {
                Self::analyze_expression(value, project, program, &param_kinds, None)?;
                if !Self::is_value_expression(value, program) {
                    return Err(CompileError::Semantic(format!(
                        "Return value of function '{}' must be a reporter expression", function.name
                    )));
                }
            }
            Self::analyze_statements(&function.body, project, program, &param_kinds, None)?;

            // Prevent parameter names from shadowing each other or creating
            // ambiguous variable references in the generated Entry script.
            let mut seen = HashSet::new();
            for param in &function.params {
                if !seen.insert(param.name.as_str()) {
                    return Err(CompileError::Semantic(format!(
                        "Function '{}' has duplicate parameter '{}'", function.name, param.name
                    )));
                }
            }
        }
        Ok(())
    }

    fn validate_declarations(program: &Program, project: &Value) -> Result<(), CompileError> {
        let mut variable_names = HashSet::new();
        for declaration in &program.variables {
            if !variable_names.insert(declaration.name.as_str()) {
                return Err(CompileError::Semantic(format!(
                    "Variable '{}' is declared more than once", declaration.name
                )));
            }
            let collision = project.get("variables").and_then(Value::as_array).is_some_and(|items| {
                items.iter().any(|variable| {
                    variable.get("name").and_then(Value::as_str) == Some(declaration.name.as_str())
                })
            });
            if collision {
                return Err(CompileError::Semantic(format!(
                    "Variable '{}' already exists in the Entry project", declaration.name
                )));
            }
            match declaration.kind {
                VariableKind::Scalar => {
                    if !Self::is_literal(&declaration.initial_value) {
                        return Err(CompileError::Semantic(format!(
                            "Initial value of variable '{}' must be a literal", declaration.name
                        )));
                    }
                }
                VariableKind::List => {
                    for item in &declaration.initial_items {
                        if !Self::is_literal(item) {
                            return Err(CompileError::Semantic(format!(
                                "Initial items of list '{}' must be literals", declaration.name
                            )));
                        }
                    }
                }
            }
        }

        let existing_function_names = Self::project_function_names(project);
        let mut function_names = HashSet::new();
        for function in &program.functions {
            if existing_function_names.contains(&function.name) {
                return Err(CompileError::Semantic(format!(
                    "Function '{}' already exists in the Entry project", function.name
                )));
            }
            if !function_names.insert(function.name.as_str()) {
                return Err(CompileError::Semantic(format!(
                    "Function '{}' is declared more than once", function.name
                )));
            }
            if statement_spec(&function.name).is_some() || reporter_spec(&function.name).is_some()
                || matches!(function.name.as_str(), "set" | "set_variable" | "change" | "change_variable"
                    | "show_variable" | "hide_variable" | "get_variable" | "raw" | "field"
                    | "raw_block" | "entry_block")
                || function.name.starts_with("list_") || function.name.starts_with("func_")
            {
                return Err(CompileError::Semantic(format!(
                    "Function name '{}' conflicts with a built-in block", function.name
                )));
            }
            let mut parameters = HashSet::new();
            for parameter in &function.params {
                if !parameters.insert(parameter.name.as_str()) {
                    return Err(CompileError::Semantic(format!(
                        "Function '{}' has duplicate parameter '{}'", function.name, parameter.name
                    )));
                }
            }
        }
        Ok(())
    }

    fn analyze_statements(
        statements: &[Statement],
        project: &Value,
        program: &Program,
        parameters: &HashMap<String, FunctionParameterKind>,
        scope_object: Option<&str>,
    ) -> Result<(), CompileError> {
        for statement in statements {
            let Statement::BlockCall { name, args, body, else_body } = statement;

            if let Some(function) = program.functions.iter().find(|function| function.name == *name) {
                if function.return_value.is_some() {
                    return Err(CompileError::Semantic(format!(
                        "Value function '{}' must be used inside an expression", name
                    )));
                }
                if body.is_some() || else_body.is_some() {
                    return Err(CompileError::Semantic(format!(
                        "Function call '{}' cannot have a statement body", name
                    )));
                }
                Self::check_function_arguments(
                    function, args, project, program, parameters, scope_object,
                )?;
                continue;
            }

            if let Some(function_id) = name.strip_prefix("func_") {
                if Self::project_has_function(project, function_id) {
                    if Self::project_function_is_value(project, function_id) {
                        return Err(CompileError::Semantic(format!(
                            "Value function '{}' must be used inside an expression", name
                        )));
                    }
                    if body.is_some() || else_body.is_some() {
                        return Err(CompileError::Semantic("Function calls cannot have statement bodies".into()));
                    }
                    Self::check_project_function_arguments(
                        function_id, args, project, program, parameters,
                    )?;
                    for argument in args {
                        Self::analyze_expression(argument, project, program, parameters, scope_object)?;
                    }
                    continue;
                }
            }

            if matches!(name.as_str(), "set" | "set_variable" | "change" | "change_variable") {
                let (variable_name, value_expression) = match args.as_slice() {
                    [Expression::Identifier(variable_name), value]
                    | [Expression::String(variable_name), value] => (variable_name, value),
                    _ => return Err(CompileError::Semantic(
                        "set/change expects a variable name followed by a value".into(),
                    )),
                };
                Self::ensure_scalar_variable_exists(variable_name, project, program, scope_object)?;
                Self::analyze_expression(value_expression, project, program, parameters, scope_object)?;
                Self::ensure_no_statement_areas(name, body, else_body)?;
                continue;
            }

            if matches!(name.as_str(), "show_variable" | "hide_variable") {
                let variable_name = Self::one_identifier_argument(name, args)?;
                Self::ensure_scalar_variable_exists(variable_name, project, program, scope_object)?;
                Self::ensure_no_statement_areas(name, body, else_body)?;
                continue;
            }

            if let Some((list_name, expressions)) = Self::list_statement_arguments(name, args)? {
                Self::ensure_list_exists(list_name, project, program, scope_object)?;
                for expression in expressions {
                    Self::analyze_expression(expression, project, program, parameters, scope_object)?;
                }
                Self::ensure_no_statement_areas(name, body, else_body)?;
                continue;
            }

            if name == "raw_block" || name == "entry_block" {
                let Some(Expression::String(type_id)) = args.first() else {
                    return Err(CompileError::Semantic(
                        "raw_block requires the Entry block type as its first string argument".into(),
                    ));
                };
                Self::ensure_builtin_type(type_id)?;
                for argument in &args[1..] {
                    Self::analyze_expression(argument, project, program, parameters, scope_object)?;
                }
                if let Some(body) = body { Self::analyze_statements(body, project, program, parameters, scope_object)?; }
                if let Some(body) = else_body { Self::analyze_statements(body, project, program, parameters, scope_object)?; }
                continue;
            }

            let spec = if name == "if" && else_body.is_some() {
                statement_spec("if_else")
            } else {
                statement_spec(name)
            };
            let Some(spec) = spec else {
                Self::ensure_builtin_type(name)?;
                return Err(CompileError::Semantic(format!("Unknown block '{}'", name)));
            };
            Self::check_argument_count(name, args, &spec.params)?;
            if matches!(
                spec.type_id,
                "_if" | "if_else" | "repeat_while_true" | "wait_until_true"
            ) {
                if let Some(condition) = args.first() {
                    if Self::expression_is_boolean(
                        condition, program, parameters, &mut HashSet::new(),
                    ) == Some(false) {
                        return Err(CompileError::Semantic(format!(
                            "Condition of block '{}' must be a boolean expression", name
                        )));
                    }
                }
            }
            if (body.is_some() || else_body.is_some()) && spec.statement_count == 0 {
                return Err(CompileError::Semantic(format!("Block '{}' has no statement area", name)));
            }
            if else_body.is_some() && spec.statement_count < 2 {
                return Err(CompileError::Semantic(format!("Block '{}' has no else statement area", name)));
            }
            Self::ensure_builtin_type(name)?;
            for argument in args {
                Self::analyze_expression(argument, project, program, parameters, scope_object)?;
            }
            if let Some(body) = body { Self::analyze_statements(body, project, program, parameters, scope_object)?; }
            if let Some(body) = else_body { Self::analyze_statements(body, project, program, parameters, scope_object)?; }
        }
        Ok(())
    }

    fn analyze_expression(
        expression: &Expression,
        project: &Value,
        program: &Program,
        parameters: &HashMap<String, FunctionParameterKind>,
        scope_object: Option<&str>,
    ) -> Result<(), CompileError> {
        match expression {
            Expression::String(_) | Expression::Number(_) | Expression::Boolean(_) | Expression::Null => Ok(()),
            Expression::Identifier(name) => {
                if parameters.contains_key(name) { Ok(()) }
                else { Self::ensure_scalar_variable_exists(name, project, program, scope_object) }
            }
            Expression::Call { name, args } if name == "field" => match args.as_slice() {
                [Expression::String(_)] | [Expression::Identifier(_)] => Ok(()),
                _ => Err(CompileError::Semantic("field() expects one string or identifier".into())),
            },
            Expression::Call { name, args } if name == "raw" => match args.as_slice() {
                [Expression::String(_) | Expression::Number(_) | Expression::Boolean(_) | Expression::Null] => Ok(()),
                [Expression::Unary { op: crate::ast::UnaryOp::Negate, value }] if matches!(value.as_ref(), Expression::Number(_)) => Ok(()),
                _ => Err(CompileError::Semantic("raw() expects one string, number, boolean, or null literal".into())),
            },
            Expression::Call { name, args } if matches!(name.as_str(), "list_get" | "list_length" | "list_contains") => {
                match (name.as_str(), args.as_slice()) {
                    ("list_get", [Expression::Identifier(list_name), index])
                    | ("list_get", [Expression::String(list_name), index]) => {
                        Self::ensure_list_exists(list_name, project, program, scope_object)?;
                        Self::analyze_expression(index, project, program, parameters, scope_object)
                    }
                    ("list_length", [Expression::Identifier(list_name)])
                    | ("list_length", [Expression::String(list_name)]) => {
                        Self::ensure_list_exists(list_name, project, program, scope_object)
                    }
                    ("list_contains", [Expression::Identifier(list_name), value])
                    | ("list_contains", [Expression::String(list_name), value]) => {
                        Self::ensure_list_exists(list_name, project, program, scope_object)?;
                        Self::analyze_expression(value, project, program, parameters, scope_object)
                    }
                    _ => Err(CompileError::Semantic(format!("Invalid arguments for '{}'", name))),
                }
            }
            Expression::Call { name, args } if name == "raw_block" || name == "entry_block" => {
                let Some(Expression::String(type_id)) = args.first() else {
                    return Err(CompileError::Semantic("raw_block requires a block type string".into()));
                };
                Self::ensure_builtin_type(type_id)?;
                for arg in &args[1..] { Self::analyze_expression(arg, project, program, parameters, scope_object)?; }
                Ok(())
            }
            Expression::Call { name, args } => {
                if let Some(function_id) = name.strip_prefix("func_") {
                    if Self::project_function_is_value(project, function_id) {
                        Self::check_project_function_arguments(
                            function_id, args, project, program, parameters,
                        )?;
                        for argument in args {
                            Self::analyze_expression(argument, project, program, parameters, scope_object)?;
                        }
                        return Ok(());
                    }
                }
                if let Some(function) = program.functions.iter().find(|function| function.name == *name) {
                    if function.return_value.is_none() {
                        return Err(CompileError::Semantic(format!(
                            "Void function '{}' cannot be used as a value", name
                        )));
                    }
                    Self::check_function_arguments(function, args, project, program, parameters, scope_object)?;
                    return Ok(());
                }
                if statement_spec(name).is_some() {
                    return Err(CompileError::Semantic(format!(
                        "'{}' is a statement block and cannot be used as a value", name
                    )));
                }
                if let Some(spec) = reporter_spec(name) {
                    Self::check_argument_count(name, args, &spec.params)?;
                } else {
                    Self::ensure_builtin_type(name)?;
                    return Err(CompileError::Semantic(format!("Unknown block '{}'", name)));
                }
                for arg in args { Self::analyze_expression(arg, project, program, parameters, scope_object)?; }
                Ok(())
            }
            Expression::Unary { value, .. } => Self::analyze_expression(value, project, program, parameters, scope_object),
            Expression::Binary { left, right, .. } => {
                Self::analyze_expression(left, project, program, parameters, scope_object)?;
                Self::analyze_expression(right, project, program, parameters, scope_object)
            }
        }
    }

    fn validate_project_collections(project: &Value) -> Result<(), CompileError> {
        let objects = project.get("objects").and_then(Value::as_array).ok_or_else(|| {
            CompileError::Project("Project must contain an 'objects' array".into())
        })?;
        let mut object_ids = HashSet::new();
        for object in objects {
            let Some(id) = object.get("id").and_then(Value::as_str) else {
                return Err(CompileError::Project(
                    "Every project object must have a string 'id'".into(),
                ));
            };
            if id.is_empty() || !object_ids.insert(id) {
                return Err(CompileError::Project(format!(
                    "Project contains an empty or duplicate object id '{id}'"
                )));
            }
            if object.get("name").and_then(Value::as_str).is_none() {
                return Err(CompileError::Project(format!(
                    "Project object '{id}' must have a string 'name'"
                )));
            }
        }

        if let Some(variables) = project.get("variables") {
            let variables = variables.as_array().ok_or_else(|| {
                CompileError::Project("Project 'variables' must be an array".into())
            })?;
            let mut variable_ids = HashSet::new();
            let mut scoped_names = HashSet::new();

            for variable in variables {
                let Some(id) = variable.get("id").and_then(Value::as_str) else {
                    return Err(CompileError::Project(
                        "Every project variable must have a string 'id'".into(),
                    ));
                };
                let Some(name) = variable.get("name").and_then(Value::as_str) else {
                    return Err(CompileError::Project(
                        format!("Project variable '{id}' must have a string 'name'"),
                    ));
                };
                if name.trim().is_empty() {
                    return Err(CompileError::Project(format!(
                        "Project variable '{id}' has an empty name"
                    )));
                }
                if id.is_empty() || !variable_ids.insert(id) {
                    return Err(CompileError::Project(format!(
                        "Project contains an empty or duplicate variable id '{id}'"
                    )));
                }
                if !matches!(
                    variable.get("variableType").and_then(Value::as_str),
                    Some("variable" | "list" | "timer" | "answer" | "slide")
                ) {
                    return Err(CompileError::Project(format!(
                        "Project variable '{name}' has an unsupported or missing variableType"
                    )));
                }
                let owner = match variable.get("object") {
                    None | Some(Value::Null) => None,
                    Some(Value::String(owner)) if object_ids.contains(owner.as_str()) => Some(owner.as_str()),
                    Some(Value::String(owner)) => {
                        return Err(CompileError::Project(format!(
                            "Project variable '{name}' refers to unknown object id '{owner}'"
                        )));
                    }
                    Some(_) => {
                        return Err(CompileError::Project(format!(
                            "Project variable '{name}' has an invalid 'object' owner"
                        )));
                    }
                };

                for (field, expected) in [
                    ("value", "string"),
                    ("visible", "boolean"),
                    ("isCloud", "boolean"),
                ] {
                    if let Some(value) = variable.get(field) {
                        let valid = match expected {
                            "string" => value.is_string(),
                            "boolean" => value.is_boolean(),
                            _ => false,
                        };
                        if !valid {
                            return Err(CompileError::Project(format!(
                                "Project variable '{name}' has an invalid '{field}' field"
                            )));
                        }
                    }
                }
                for field in ["x", "y", "width", "height", "minValue", "maxValue"] {
                    if variable.get(field).is_some_and(|value| !value.is_null() && !value.is_number()) {
                        return Err(CompileError::Project(format!(
                            "Project variable '{name}' has an invalid '{field}' field"
                        )));
                    }
                }
                if variable.get("variableType").and_then(Value::as_str) == Some("list") {
                    let items = variable.get("array").and_then(Value::as_array).ok_or_else(|| {
                        CompileError::Project(format!(
                            "Project list '{name}' must contain an 'array'"
                        ))
                    })?;
                    for (index, item) in items.iter().enumerate() {
                        if item.get("data").and_then(Value::as_str).is_none() {
                            return Err(CompileError::Project(format!(
                                "Project list '{name}' item {} must contain a string 'data' field",
                                index + 1
                            )));
                        }
                    }
                }
                if !scoped_names.insert((owner, name)) {
                    return Err(CompileError::Project(format!(
                        "Project contains duplicate variable name '{name}' in the same scope"
                    )));
                }
            }
        }
        if let Some(functions) = project.get("functions") {
            let functions = functions.as_array().ok_or_else(|| {
                CompileError::Project("Project 'functions' must be an array".into())
            })?;
            let mut ids = HashSet::new();
            let mut names = HashSet::new();
            for function in functions {
                let Some(id) = function.get("id").and_then(Value::as_str) else {
                    return Err(CompileError::Project(
                        "Every project function must have a string 'id'".into(),
                    ));
                };
                if id.is_empty() || !ids.insert(id) {
                    return Err(CompileError::Project(format!(
                        "Project contains an empty or duplicate function id '{id}'"
                    )));
                }
                if !matches!(function.get("type").and_then(Value::as_str), Some("normal" | "value")) {
                    return Err(CompileError::Project(format!(
                        "Project function '{id}' must have type 'normal' or 'value'"
                    )));
                }

                if function.get("useLocalVariables")
                    .is_some_and(|value| !value.is_null() && !value.is_boolean())
                {
                    return Err(CompileError::Project(format!(
                        "Project function '{id}' has an invalid 'useLocalVariables' field"
                    )));
                }
                if function.get("localVariables")
                    .is_some_and(|value| !value.is_null() && !value.is_array())
                {
                    return Err(CompileError::Project(format!(
                        "Project function '{id}' has an invalid 'localVariables' field"
                    )));
                }

                let Some(content) = function.get("content").and_then(Value::as_str) else {
                    return Err(CompileError::Project(format!(
                        "Project function '{id}' must have a string 'content' field"
                    )));
                };
                let content: Value = serde_json::from_str(content).map_err(|error| {
                    CompileError::Project(format!(
                        "Project function '{id}' has invalid JSON in 'content': {error}"
                    ))
                })?;
                let definition = Self::find_function_create_block(&content).ok_or_else(|| {
                    CompileError::Project(format!(
                        "Project function '{id}' does not contain a function definition block"
                    ))
                })?;
                let expected_type = if function.get("type").and_then(Value::as_str) == Some("value") {
                    "function_create_value"
                } else {
                    "function_create"
                };
                if definition.get("type").and_then(Value::as_str) != Some(expected_type) {
                    return Err(CompileError::Project(format!(
                        "Project function '{id}' has a definition block that does not match its type"
                    )));
                }
                let name = Self::find_function_label(&content).ok_or_else(|| {
                    CompileError::Project(format!(
                        "Project function '{id}' is missing its function name"
                    ))
                })?;
                if name.trim().is_empty() || !names.insert(name.clone()) {
                    return Err(CompileError::Project(format!(
                        "Project contains an empty or duplicate function name '{name}'"
                    )));
                }
                if Self::project_function_parameter_kinds(project, id).is_none() {
                    return Err(CompileError::Project(format!(
                        "Project function '{id}' has an invalid parameter definition"
                    )));
                }
            }
        }
        Ok(())
    }

    /// Resolve a variable/list using Entry's object-local-first lookup rule,
    /// then fall back to the global namespace. Ambiguous matches are rejected.
    fn find_project_variable<'a>(
        name: &str,
        project: &'a Value,
        scope_object: Option<&str>,
    ) -> Result<Option<&'a Value>, CompileError> {
        let Some(variables_value) = project.get("variables") else {
            return Ok(None);
        };
        let variables = variables_value.as_array().ok_or_else(|| {
            CompileError::Project("Project 'variables' must be an array".into())
        })?;

        let object_id = scope_object.and_then(|object_name| {
            project.get("objects")
                .and_then(Value::as_array)
                .and_then(|objects| objects.iter().find(|object| {
                    object.get("name").and_then(Value::as_str) == Some(object_name)
                }))
                .and_then(|object| object.get("id"))
                .and_then(Value::as_str)
        });

        let mut local_matches = Vec::new();
        let mut global_matches = Vec::new();
        for variable in variables {
            if variable.get("name").and_then(Value::as_str) != Some(name) {
                continue;
            }
            if let (Some(owner_id), Some(variable_owner)) = (
                object_id,
                variable.get("object").and_then(Value::as_str),
            ) {
                if variable_owner == owner_id {
                    local_matches.push(variable);
                    continue;
                }
            }
            if variable.get("object").map_or(true, Value::is_null) {
                global_matches.push(variable);
            }
        }

        let matches = if local_matches.is_empty() { &global_matches } else { &local_matches };
        match matches.as_slice() {
            [] => Ok(None),
            [variable] => {
                let id = variable.get("id").and_then(Value::as_str).filter(|id| !id.is_empty());
                if id.is_none() {
                    return Err(CompileError::Project(format!(
                        "Project variable '{name}' is missing a valid string id"
                    )));
                }
                Ok(Some(*variable))
            }
            _ => Err(CompileError::Project(format!(
                "Variable name '{name}' is ambiguous in the selected scope"
            ))),
        }
    }

    fn project_function_names(project: &Value) -> HashSet<String> {
        let mut names = HashSet::new();
        let Some(functions) = project.get("functions").and_then(Value::as_array) else {
            return names;
        };
        for function in functions {
            let Some(content) = function.get("content").and_then(Value::as_str) else {
                continue;
            };
            let Ok(content) = serde_json::from_str::<Value>(content) else {
                continue;
            };
            if let Some(name) = Self::find_function_label(&content) {
                names.insert(name);
            }
        }
        names
    }

    fn find_function_label(value: &Value) -> Option<String> {
        match value {
            Value::Array(items) => items.iter().find_map(Self::find_function_label),
            Value::Object(map) => {
                if map.get("type").and_then(Value::as_str) == Some("function_field_label") {
                    return map.get("params")
                        .and_then(Value::as_array)
                        .and_then(|params| params.first())
                        .and_then(Value::as_str)
                        .map(str::to_owned);
                }
                map.values().find_map(Self::find_function_label)
            }
            _ => None,
        }
    }

    fn check_project_function_arguments(
        function_id: &str,
        args: &[Expression],
        project: &Value,
        program: &Program,
        caller_parameters: &HashMap<String, FunctionParameterKind>,
    ) -> Result<(), CompileError> {
        if let Some(parameter_kinds) = Self::project_function_parameter_kinds(project, function_id) {
            let expected = parameter_kinds.len();
            if args.len() != expected {
                return Err(CompileError::Semantic(format!(
                    "Entry function 'func_{function_id}' expects {expected} argument(s), but {} were provided",
                    args.len()
                )));
            }
            for (index, (kind, argument)) in parameter_kinds.iter().zip(args).enumerate() {
                let Some(kind) = kind else { continue };
                let argument_is_boolean = Self::expression_is_boolean(
                    argument, program, caller_parameters, &mut HashSet::new(),
                );
                let mismatch = matches!(
                    (*kind, argument_is_boolean),
                    (FunctionParameterKind::Boolean, Some(false))
                        | (FunctionParameterKind::Value, Some(true))
                );
                if mismatch {
                    let expected = match kind {
                        FunctionParameterKind::Boolean => "a boolean",
                        FunctionParameterKind::Value => "a value",
                    };
                    return Err(CompileError::Semantic(format!(
                        "Argument {} of Entry function 'func_{function_id}' must be {expected}",
                        index + 1
                    )));
                }
            }
        }
        Ok(())
    }

    fn project_function_parameter_kinds(
        project: &Value,
        function_id: &str,
    ) -> Option<Vec<Option<FunctionParameterKind>>> {
        let function = project.get("functions")?.as_array()?.iter().find(|function| {
            function.get("id").and_then(Value::as_str) == Some(function_id)
        })?;
        let content = serde_json::from_str::<Value>(function.get("content")?.as_str()?).ok()?;
        let definition = Self::find_function_create_block(&content)?;
        let header = definition.get("params")?.as_array()?.first()?;
        if header.get("type")?.as_str()? != "function_field_label" {
            return None;
        }
        let header_params = header.get("params")?.as_array()?;
        let Some(mut ribbon) = header_params.get(1) else {
            // Some older Entry projects omit the empty parameter-chain slot
            // entirely for functions that have no parameters.
            return Some(Vec::new());
        };
        if ribbon.is_null() {
            return Some(Vec::new());
        }

        let mut kinds = Vec::new();
        loop {
            let kind = ribbon.get("type")?.as_str()?;
            if !kind.starts_with("function_field_") {
                return None;
            }
            kinds.push(match kind {
                "function_field_string" => Some(FunctionParameterKind::Value),
                "function_field_boolean" => Some(FunctionParameterKind::Boolean),
                _ => None,
            });
            let params = ribbon.get("params")?.as_array()?;
            ribbon = params.get(1)?;
            if ribbon.is_null() {
                return Some(kinds);
            }
        }
    }

    fn find_function_create_block(value: &Value) -> Option<&Value> {
        match value {
            Value::Array(items) => items.iter().find_map(Self::find_function_create_block),
            Value::Object(map) => {
                if matches!(map.get("type").and_then(Value::as_str), Some("function_create" | "function_create_value")) {
                    return Some(value);
                }
                map.values().find_map(Self::find_function_create_block)
            }
            _ => None,
        }
    }

    fn ensure_scalar_variable_exists(
        name: &str,
        project: &Value,
        program: &Program,
        scope_object: Option<&str>,
    ) -> Result<(), CompileError> {
        if let Some(declaration) = program.variables.iter().find(|variable| variable.name == name) {
            return match declaration.kind {
                VariableKind::Scalar => Ok(()),
                VariableKind::List => Err(CompileError::Semantic(format!(
                    "'{name}' is a list, not a scalar variable"
                ))),
            };
        }

        let variable = Self::find_project_variable(name, project, scope_object)?;
        match variable {
            Some(variable) if variable.get("variableType").and_then(Value::as_str) == Some("list") => {
                Err(CompileError::Semantic(format!("'{name}' is a list, not a scalar variable")))
            }
            Some(_) => Ok(()),
            None => Err(CompileError::Semantic(format!("Unknown variable '{name}'"))),
        }
    }

    fn ensure_list_exists(
        name: &str,
        project: &Value,
        program: &Program,
        scope_object: Option<&str>,
    ) -> Result<(), CompileError> {
        if let Some(declaration) = program.variables.iter().find(|variable| variable.name == name) {
            return match declaration.kind {
                VariableKind::List => Ok(()),
                VariableKind::Scalar => Err(CompileError::Semantic(format!(
                    "'{name}' is a scalar variable, not a list"
                ))),
            };
        }

        let variable = Self::find_project_variable(name, project, scope_object)?;
        match variable {
            Some(variable) if variable.get("variableType").and_then(Value::as_str) == Some("list") => Ok(()),
            Some(_) => Err(CompileError::Semantic(format!("'{name}' is a scalar variable, not a list"))),
            None => Err(CompileError::Semantic(format!("Unknown list '{name}'"))),
        }
    }

    fn one_identifier_argument<'a>(name: &str, args: &'a [Expression]) -> Result<&'a str, CompileError> {
        match args {
            [Expression::Identifier(value)] | [Expression::String(value)] => Ok(value),
            _ => Err(CompileError::Semantic(format!("{name} expects one variable name"))),
        }
    }

    fn list_statement_arguments<'a>(name: &str, args: &'a [Expression]) -> Result<Option<(&'a str, Vec<&'a Expression>)>, CompileError> {
        let (list_name, expressions) = match (name, args) {
            ("list_add", [Expression::Identifier(list), value])
            | ("list_add", [Expression::String(list), value]) => (list.as_str(), vec![value]),
            ("list_remove", [Expression::Identifier(list), index])
            | ("list_remove", [Expression::String(list), index]) => (list.as_str(), vec![index]),
            ("list_insert", [Expression::Identifier(list), index, value])
            | ("list_insert", [Expression::String(list), index, value]) => (list.as_str(), vec![index, value]),
            ("list_set", [Expression::Identifier(list), index, value])
            | ("list_set", [Expression::String(list), index, value]) => (list.as_str(), vec![index, value]),
            ("list_show" | "list_hide", [Expression::Identifier(list)])
            | ("list_show" | "list_hide", [Expression::String(list)]) => (list.as_str(), vec![]),
            _ if name.starts_with("list_") => return Err(CompileError::Semantic(format!("Invalid arguments for list operation '{name}'"))),
            _ => return Ok(None),
        };
        Ok(Some((list_name, expressions)))
    }

    fn ensure_no_statement_areas(
        name: &str,
        body: &Option<Vec<Statement>>,
        else_body: &Option<Vec<Statement>>,
    ) -> Result<(), CompileError> {
        if body.is_some() || else_body.is_some() {
            Err(CompileError::Semantic(format!("'{name}' does not have a statement area")))
        } else { Ok(()) }
    }

    fn check_function_arguments(
        function: &FunctionDefinition,
        args: &[Expression],
        project: &Value,
        program: &Program,
        caller_parameters: &HashMap<String, FunctionParameterKind>,
        scope_object: Option<&str>,
    ) -> Result<(), CompileError> {
        if args.len() != function.params.len() {
            return Err(CompileError::Semantic(format!(
                "Function '{}' expects {} argument(s), but {} were provided",
                function.name, function.params.len(), args.len()
            )));
        }

        for (index, (parameter, argument)) in function.params.iter().zip(args).enumerate() {
            Self::analyze_expression(argument, project, program, caller_parameters, scope_object)?;
            let argument_is_boolean = Self::expression_is_boolean(
                argument, program, caller_parameters, &mut HashSet::new(),
            );
            let mismatch = match (parameter.kind, argument_is_boolean) {
                (FunctionParameterKind::Boolean, Some(false)) => true,
                (FunctionParameterKind::Value, Some(true)) => true,
                _ => false,
            };
            if mismatch {
                let expected = match parameter.kind {
                    FunctionParameterKind::Boolean => "a boolean",
                    FunctionParameterKind::Value => "a value",
                };
                return Err(CompileError::Semantic(format!(
                    "Argument {} ('{}') of function '{}' must be {expected}",
                    index + 1, parameter.name, function.name
                )));
            }
        }
        Ok(())
    }

    /// Returns Some(true) for a known boolean expression, Some(false) for a
    /// known value expression, and None when the expression's type is unknown.
    fn expression_is_boolean(
        expression: &Expression,
        program: &Program,
        parameters: &HashMap<String, FunctionParameterKind>,
        visiting_functions: &mut HashSet<String>,
    ) -> Option<bool> {
        match expression {
            Expression::Boolean(_) => Some(true),
            Expression::String(_) | Expression::Number(_) | Expression::Null => Some(false),
            // Entry variable reporters are value reporters, even when their
            // current stored text happens to be "true" or "false".
            Expression::Identifier(name) => Some(
                parameters.get(name).is_some_and(|kind| *kind == FunctionParameterKind::Boolean)
            ),
            Expression::Unary { op: UnaryOp::Not, .. } => Some(true),
            Expression::Unary { op: UnaryOp::Negate, .. } => Some(false),
            Expression::Binary { op, .. } => Some(matches!(
                op,
                BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Less | BinaryOp::LessEqual
                    | BinaryOp::Greater | BinaryOp::GreaterEqual | BinaryOp::And | BinaryOp::Or
            )),
            Expression::Call { name, args } if name == "raw" => match args.as_slice() {
                [Expression::Boolean(_)] => Some(true),
                [Expression::String(_) | Expression::Number(_) | Expression::Null] => Some(false),
                _ => None,
            },
            Expression::Call { name, .. } if matches!(
                name.as_str(),
                "list_contains" | "clicked" | "is_clicked" | "is_boost_mode"
                    | "is_object_clicked" | "key_down" | "is_press_some_key"
                    | "touching" | "reach_something" | "boolean_basic_operator"
                    | "boolean_and_or" | "boolean_not"
            ) => Some(true),
            Expression::Call { name, .. } => {
                if let Some(function) = program.functions.iter().find(|function| function.name == *name) {
                    let return_value = function.return_value.as_ref()?;
                    if !visiting_functions.insert(name.clone()) {
                        return None;
                    }
                    let function_parameters = function.params.iter()
                        .map(|parameter| (parameter.name.clone(), parameter.kind))
                        .collect::<HashMap<_, _>>();
                    let result = Self::expression_is_boolean(
                        return_value, program, &function_parameters, visiting_functions,
                    );
                    visiting_functions.remove(name);
                    result
                } else if let Some(spec) = reporter_spec(name) {
                    Some(matches!(
                        spec.type_id,
                        "boolean_basic_operator" | "boolean_and_or" | "boolean_not"
                            | "is_clicked" | "is_boost_mode" | "is_object_clicked"
                            | "is_press_some_key" | "reach_something"
                    ))
                } else {
                    None
                }
            }
        }
    }

    fn is_value_expression(expression: &Expression, program: &Program) -> bool {
        match expression {
            Expression::String(_) | Expression::Number(_) | Expression::Boolean(_) | Expression::Null
            | Expression::Identifier(_) | Expression::Unary { .. } | Expression::Binary { .. } => true,
            Expression::Call { name, .. } => {
                program.functions.iter().find(|function| function.name == *name)
                    .map_or(true, |function| function.return_value.is_some())
            }
        }
    }

    fn is_literal(expression: &Expression) -> bool {
        matches!(expression, Expression::String(_) | Expression::Number(_) | Expression::Boolean(_) | Expression::Null)
    }

    fn project_function_is_value(project: &Value, function_id: &str) -> bool {
        project.get("functions").and_then(Value::as_array).is_some_and(|functions| {
            functions.iter().any(|function| {
                function.get("id").and_then(Value::as_str) == Some(function_id)
                    && function.get("type").and_then(Value::as_str) == Some("value")
            })
        })
    }

    fn project_has_function(project: &Value, function_id: &str) -> bool {
        project.get("functions").and_then(Value::as_array).is_some_and(|functions| {
            functions.iter().any(|function| function.get("id").and_then(Value::as_str) == Some(function_id))
        })
    }

    fn check_argument_count(name: &str, args: &[Expression], params: &[ParamSlot]) -> Result<(), CompileError> {
        let minimum = required_input_count(params);
        let maximum = input_count(params);
        if args.len() < minimum || args.len() > maximum {
            let expected = if minimum == maximum { minimum.to_string() } else { format!("{minimum} to {maximum}") };
            return Err(CompileError::Semantic(format!(
                "'{}' expects {} argument(s), but {} were provided", name, expected, args.len()
            )));
        }
        Ok(())
    }

    fn ensure_builtin_type(type_id: &str) -> Result<(), CompileError> {
        let lower = type_id.to_ascii_lowercase();
        if lower.starts_with("block_expansion_") || lower.starts_with("expansion_")
            || lower.starts_with("block_ai_") || lower.starts_with("ai_") || lower.contains("ai_utilize")
        {
            Err(CompileError::Semantic(format!(
                "Entry extension and AI blocks are excluded: '{type_id}'"
            )))
        } else { Ok(()) }
    }
}
