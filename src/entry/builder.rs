use std::collections::HashMap;

use serde_json::{json, Value};

use crate::ast::{
    BinaryOp, EventHandler, Expression, FunctionDefinition, FunctionParameterKind,
    ObjectSection, Program, Statement, UnaryOp, VariableKind,
};
use crate::error::CompileError;

use super::input::EntryInput;
use super::ir::{EntryBlock, EntryScript, EntryThread};
use super::registry::{
    event_spec, input_count, reporter_spec, required_input_count, statement_spec, BlockSpec,
    ParamEncoding, ParamSlot,
};
use super::serializer::IdAllocator;

type ParameterBindings = HashMap<String, String>;

#[derive(Debug, Clone)]
struct FunctionParameterSymbol {
    name: String,
    block_type: String,
}

#[derive(Debug, Clone)]
struct FunctionSymbol {
    id: String,
    returns_value: bool,
    parameters: Vec<FunctionParameterSymbol>,
}

#[derive(Debug)]
pub struct BuildOutput {
    pub scripts: Vec<(String, EntryScript)>,
    pub functions: Vec<Value>,
}

struct BuildEnv<'a> {
    project: &'a Value,
    current_object: &'a str,
    functions: &'a HashMap<String, FunctionSymbol>,
    parameters: &'a ParameterBindings,
}

pub struct EntryBuilder;

impl EntryBuilder {
    /// Builds object scripts, new global variables/lists, and project-level functions.
    pub fn build(
        program: &Program,
        project: &mut Value,
        ids: &mut IdAllocator,
    ) -> Result<BuildOutput, CompileError> {
        Self::append_variables(program, project, ids)?;
        let symbols = Self::allocate_function_symbols(program, ids);

        let mut scripts = Vec::with_capacity(program.objects.len());
        for section in &program.objects {
            let empty_parameters = ParameterBindings::new();
            let env = BuildEnv {
                project: &*project,
                current_object: &section.name,
                functions: &symbols,
                parameters: &empty_parameters,
            };
            scripts.push((section.name.clone(), Self::build_object(section, &env)?));
        }

        let mut functions = Vec::with_capacity(program.functions.len());
        for definition in &program.functions {
            let symbol = symbols.get(&definition.name).expect("function symbol was allocated");
            let parameters = symbol.parameters.iter()
                .map(|parameter| (parameter.name.clone(), parameter.block_type.clone()))
                .collect::<ParameterBindings>();
            let env = BuildEnv {
                project: &*project,
                current_object: "",
                functions: &symbols,
                parameters: &parameters,
            };
            functions.push(Self::build_function(definition, symbol, &env, ids)?);
        }

        Ok(BuildOutput { scripts, functions })
    }

    fn append_variables(
        program: &Program,
        project: &mut Value,
        ids: &mut IdAllocator,
    ) -> Result<(), CompileError> {
        if project.get("variables").is_none() {
            project["variables"] = json!([]);
        }
        let variables = project.get_mut("variables")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| CompileError::Project("Project 'variables' must be an array".into()))?;

        for declaration in &program.variables {
            let id = ids.next_id();
            let value = match declaration.kind {
                VariableKind::Scalar => Self::literal_to_entry_string(&declaration.initial_value)?,
                VariableKind::List => String::new(),
            };
            let mut variable = json!({
                "name": declaration.name,
                "id": id,
                "visible": false,
                "value": value,
                "variableType": match declaration.kind {
                    VariableKind::Scalar => "variable",
                    VariableKind::List => "list",
                },
                "isCloud": false,
                "isRealTime": false,
                "cloudDate": false,
                "object": null,
                "x": 150,
                "y": -100
            });
            if declaration.kind == VariableKind::List {
                variable["array"] = Value::Array(
                    declaration.initial_items.iter().map(|item| {
                        Self::literal_to_entry_string(item).map(|data| json!({"data": data}))
                    }).collect::<Result<Vec<_>, _>>()?,
                );
            }
            variables.push(variable);
        }
        Ok(())
    }

    fn literal_to_entry_string(expression: &Expression) -> Result<String, CompileError> {
        match expression {
            Expression::String(value) => Ok(value.clone()),
            Expression::Number(value) => Ok(value.clone()),
            Expression::Boolean(value) => Ok(value.to_string()),
            Expression::Null => Ok(String::new()),
            _ => Err(CompileError::Semantic("Variable initializers must be literal values".into())),
        }
    }

    fn allocate_function_symbols(
        program: &Program,
        ids: &mut IdAllocator,
    ) -> HashMap<String, FunctionSymbol> {
        let mut symbols = HashMap::new();
        for function in &program.functions {
            let function_id = ids.next_id();
            let parameters = function.params.iter().map(|parameter| {
                let parameter_id = ids.next_id();
                let prefix = match parameter.kind {
                    FunctionParameterKind::Value => "stringParam",
                    FunctionParameterKind::Boolean => "booleanParam",
                };
                FunctionParameterSymbol {
                    name: parameter.name.clone(),
                    block_type: format!("{prefix}_{parameter_id}"),
                }
            }).collect();
            symbols.insert(function.name.clone(), FunctionSymbol {
                id: function_id,
                returns_value: function.return_value.is_some(),
                parameters,
            });
        }
        symbols
    }

    fn build_function(
        definition: &FunctionDefinition,
        symbol: &FunctionSymbol,
        env: &BuildEnv<'_>,
        ids: &mut IdAllocator,
    ) -> Result<Value, CompileError> {
        let mut chain = EntryBlock::new("function_field_label");
        chain.params.push(EntryInput::Field(definition.name.clone()));
        let mut next = EntryInput::Empty;
        for parameter in symbol.parameters.iter().rev() {
            let mut ribbon = EntryBlock::new(if parameter.block_type.starts_with("booleanParam_") {
                "function_field_boolean"
            } else {
                "function_field_string"
            });
            ribbon.params.push(EntryInput::Block(Box::new(EntryBlock::new(parameter.block_type.clone()))));
            ribbon.params.push(next);
            next = EntryInput::Block(Box::new(ribbon));
        }
        chain.params.push(next);

        let mut root = EntryBlock::new(if symbol.returns_value {
            "function_create_value"
        } else {
            "function_create"
        });
        root.params.push(EntryInput::Block(Box::new(chain)));
        if symbol.returns_value {
            root.params.push(EntryInput::Empty);
            root.params.push(EntryInput::Empty);
            root.params.push(Self::build_expression(
                definition.return_value.as_ref().expect("value function has return expression"), env,
            )?);
        } else {
            root.params.push(EntryInput::Empty);
        }
        root.statements = vec![Self::build_statements(&definition.body, env)?];

        let content = super::serializer::serialize_script(
            &EntryScript { threads: vec![EntryThread { blocks: vec![root] }] }, ids,
        )?;
        Ok(json!({
            "id": symbol.id,
            "type": if symbol.returns_value { "value" } else { "normal" },
            "localVariables": [],
            "useLocalVariables": false,
            "content": content
        }))
    }

    fn build_object(section: &ObjectSection, env: &BuildEnv<'_>) -> Result<EntryScript, CompileError> {
        let mut script = EntryScript::default();
        for handler in &section.handlers {
            script.threads.push(Self::build_handler(handler, env)?);
        }
        Ok(script)
    }

    fn build_handler(handler: &EventHandler, env: &BuildEnv<'_>) -> Result<EntryThread, CompileError> {
        let event_block = if let Some(spec) = event_spec(&handler.event) {
            Self::build_spec_block(&spec, &handler.args, None, None, env)?
        } else {
            Self::ensure_builtin_type(&handler.event)?;
            let mut block = EntryBlock::new(handler.event.clone());
            block.params = handler.args.iter()
                .map(|argument| Self::build_expression(argument, env))
                .collect::<Result<Vec<_>, _>>()?;
            block
        };
        let mut blocks = vec![event_block];
        blocks.extend(Self::build_statements(&handler.body, env)?);
        Ok(EntryThread { blocks })
    }

    fn build_statement(statement: &Statement, env: &BuildEnv<'_>) -> Result<EntryBlock, CompileError> {
        let Statement::BlockCall { name, args, body, else_body } = statement;

        if let Some(function) = env.functions.get(name) {
            if function.returns_value {
                return Err(CompileError::Semantic(format!(
                    "Value function '{}' must be used inside an expression", name
                )));
            }
            let mut block = EntryBlock::new(format!("func_{}", function.id));
            block.params = args.iter().map(|argument| Self::build_expression(argument, env))
                .collect::<Result<Vec<_>, _>>()?;
            block.params.push(EntryInput::Empty);
            return Ok(block);
        }

        if matches!(name.as_str(), "set" | "set_variable" | "change" | "change_variable") {
            let (variable_name, value_expression) = match args.as_slice() {
                [Expression::Identifier(variable_name), value]
                | [Expression::String(variable_name), value] => (variable_name.as_str(), value),
                _ => return Err(CompileError::Semantic(
                    "set/change expects a variable name followed by a value".into(),
                )),
            };
            let variable_id = Self::find_variable_id(variable_name, env)?;
            let mut block = EntryBlock::new(if matches!(name.as_str(), "set" | "set_variable") {
                "set_variable"
            } else { "change_variable" });
            block.params = vec![
                EntryInput::Field(variable_id),
                Self::build_expression(value_expression, env)?,
                EntryInput::Empty,
            ];
            return Ok(block);
        }

        if matches!(name.as_str(), "show_variable" | "hide_variable") {
            let variable_name = Self::one_identifier_argument(name, args)?;
            let variable_id = Self::find_variable_id(variable_name, env)?;
            let mut block = EntryBlock::new(name.as_str());
            block.params = vec![EntryInput::Field(variable_id), EntryInput::Empty];
            return Ok(block);
        }

        if name.starts_with("list_") {
            return Self::build_list_statement(name, args, env);
        }

        if name == "raw_block" || name == "entry_block" {
            let (type_id, params) = Self::split_raw_block_args(args, env)?;
            let mut block = EntryBlock::new(type_id);
            block.params = params;
            if let Some(body) = body { block.statements.push(Self::build_statements(body, env)?); }
            if let Some(else_body) = else_body {
                if block.statements.is_empty() { block.statements.push(Vec::new()); }
                block.statements.push(Self::build_statements(else_body, env)?);
            }
            return Ok(block);
        }

        if let Some(function_id) = name.strip_prefix("func_") {
            if Self::project_has_function(env.project, function_id) {
                let mut block = EntryBlock::new(name.clone());
                block.params = args.iter().map(|argument| Self::build_expression(argument, env))
                    .collect::<Result<Vec<_>, _>>()?;
                block.params.push(EntryInput::Empty);
                return Ok(block);
            }
        }

        if let Some(spec) = if name == "if" && else_body.is_some() {
            statement_spec("if_else")
        } else { statement_spec(name) } {
            return Self::build_spec_block(&spec, args, body.as_deref(), else_body.as_deref(), env);
        }
        Self::ensure_builtin_type(name)?;
        Err(CompileError::Semantic(format!("Unknown block '{}'", name)))
    }

    fn build_list_statement(name: &str, args: &[Expression], env: &BuildEnv<'_>) -> Result<EntryBlock, CompileError> {
        let block = match (name, args) {
            ("list_add", [Expression::Identifier(list), value])
            | ("list_add", [Expression::String(list), value]) => {
                let id = Self::find_list_id(list, env)?;
                EntryBlock::new_with_params("add_value_to_list", vec![
                    Self::build_expression(value, env)?, EntryInput::Field(id), EntryInput::Empty,
                ])
            }
            ("list_remove", [Expression::Identifier(list), index])
            | ("list_remove", [Expression::String(list), index]) => {
                let id = Self::find_list_id(list, env)?;
                EntryBlock::new_with_params("remove_value_from_list", vec![
                    Self::build_expression(index, env)?, EntryInput::Field(id), EntryInput::Empty,
                ])
            }
            ("list_insert", [Expression::Identifier(list), index, value])
            | ("list_insert", [Expression::String(list), index, value]) => {
                let id = Self::find_list_id(list, env)?;
                EntryBlock::new_with_params("insert_value_to_list", vec![
                    Self::build_expression(value, env)?, EntryInput::Field(id),
                    Self::build_expression(index, env)?, EntryInput::Empty,
                ])
            }
            ("list_set", [Expression::Identifier(list), index, value])
            | ("list_set", [Expression::String(list), index, value]) => {
                let id = Self::find_list_id(list, env)?;
                EntryBlock::new_with_params("change_value_list_index", vec![
                    EntryInput::Field(id), Self::build_expression(index, env)?,
                    Self::build_expression(value, env)?, EntryInput::Empty,
                ])
            }
            ("list_show" | "list_hide", [Expression::Identifier(list)])
            | ("list_show" | "list_hide", [Expression::String(list)]) => {
                let id = Self::find_list_id(list, env)?;
                EntryBlock::new_with_params(if name == "list_show" { "show_list" } else { "hide_list" }, vec![
                    EntryInput::Field(id), EntryInput::Empty,
                ])
            }
            _ => return Err(CompileError::Semantic(format!("Invalid arguments for list operation '{name}'"))),
        };
        Ok(block)
    }

    fn build_statements(statements: &[Statement], env: &BuildEnv<'_>) -> Result<Vec<EntryBlock>, CompileError> {
        statements.iter().map(|statement| Self::build_statement(statement, env)).collect()
    }

    fn build_spec_block(
        spec: &BlockSpec,
        args: &[Expression],
        body: Option<&[Statement]>,
        else_body: Option<&[Statement]>,
        env: &BuildEnv<'_>,
    ) -> Result<EntryBlock, CompileError> {
        let minimum = required_input_count(&spec.params);
        let maximum = input_count(&spec.params);
        if args.len() < minimum || args.len() > maximum {
            let expected = if minimum == maximum { minimum.to_string() } else { format!("{minimum} to {maximum}") };
            return Err(CompileError::Semantic(format!(
                "Block '{}' expects {expected} argument(s), but {} were provided", spec.type_id, args.len()
            )));
        }
        if (body.is_some() || else_body.is_some()) && spec.statement_count == 0 {
            return Err(CompileError::Semantic(format!("Block '{}' does not have a statement area", spec.type_id)));
        }
        if else_body.is_some() && spec.statement_count < 2 {
            return Err(CompileError::Semantic(format!("Block '{}' does not have an else statement area", spec.type_id)));
        }

        let mut block = EntryBlock::new(spec.type_id);
        block.x = spec.x;
        block.y = spec.y;
        let mut argument_index = 0;
        for slot in &spec.params {
            match slot {
                ParamSlot::Fixed(value) => block.params.push(Self::fixed_input(value)),
                ParamSlot::Input { encoding, default } => {
                    if let Some(argument) = args.get(argument_index) {
                        block.params.push(Self::build_expression_for_encoding(argument, *encoding, env)?);
                    } else if let Some(default) = default {
                        block.params.push(Self::default_input(*encoding, default));
                    } else {
                        block.params.push(EntryInput::Empty);
                    }
                    argument_index += 1;
                }
            }
        }
        block.statements = vec![Vec::new(); spec.statement_count];
        if let Some(body) = body { block.statements[0] = Self::build_statements(body, env)?; }
        if let Some(else_body) = else_body { block.statements[1] = Self::build_statements(else_body, env)?; }
        Ok(block)
    }

    fn split_raw_block_args(args: &[Expression], env: &BuildEnv<'_>) -> Result<(String, Vec<EntryInput>), CompileError> {
        let Some(Expression::String(type_id)) = args.first() else {
            return Err(CompileError::Semantic("raw_block requires the Entry block type as its first string argument".into()));
        };
        Self::ensure_builtin_type(type_id)?;
        let params = args[1..].iter().map(|argument| Self::build_expression(argument, env))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((type_id.clone(), params))
    }

    fn build_expression_for_encoding(
        expression: &Expression,
        encoding: ParamEncoding,
        env: &BuildEnv<'_>,
    ) -> Result<EntryInput, CompileError> {
        if encoding == ParamEncoding::Raw {
            return match expression {
                Expression::String(value) => Ok(EntryInput::Field(value.clone())),
                Expression::Number(value) => Ok(EntryInput::Raw(number_to_json(value)?)),
                Expression::Boolean(value) => Ok(EntryInput::Raw(Value::Bool(*value))),
                Expression::Null => Ok(EntryInput::Empty),
                Expression::Call { name, args } if name == "field" => Self::field_argument(args),
                Expression::Call { name, args } if name == "raw" => Self::raw_argument(args),
                Expression::Identifier(value) => Ok(EntryInput::Field(value.clone())),
                _ => Err(CompileError::Semantic(
                    "This Entry parameter is a field/dropdown slot. Use field(\"value\") for an explicit field value.".into(),
                )),
            };
        }
        Self::build_expression(expression, env)
    }

    fn build_expression(expression: &Expression, env: &BuildEnv<'_>) -> Result<EntryInput, CompileError> {
        match expression {
            Expression::String(value) => Ok(EntryInput::Text(value.clone())),
            Expression::Number(value) => Ok(EntryInput::Number(value.clone())),
            Expression::Boolean(value) => Ok(EntryInput::Boolean(*value)),
            Expression::Null => Ok(EntryInput::Empty),
            Expression::Identifier(name) => {
                if let Some(block_type) = env.parameters.get(name) {
                    Ok(EntryInput::Block(Box::new(EntryBlock::new(block_type.clone()))))
                } else {
                    Self::build_variable_reporter(name, env)
                }
            }
            Expression::Call { name, args } if name == "field" => Self::field_argument(args),
            Expression::Call { name, args } if name == "raw" => Self::raw_argument(args),
            Expression::Call { name, args } if name == "raw_block" || name == "entry_block" => {
                let (type_id, params) = Self::split_raw_block_args(args, env)?;
                Ok(EntryInput::Block(Box::new(EntryBlock::new_with_params(type_id, params))))
            }
            Expression::Call { name, args } if matches!(name.as_str(), "list_get" | "list_length" | "list_contains") => {
                Self::build_list_reporter(name, args, env)
            }
            Expression::Call { name, args } if name == "get_variable" => {
                match args.as_slice() {
                    [Expression::Identifier(variable_name)] | [Expression::String(variable_name)] => {
                        let id = Self::find_variable_id(variable_name, env)?;
                        Ok(EntryInput::Block(Box::new(EntryBlock::new_with_params("get_variable", vec![EntryInput::Field(id)]))))
                    }
                    _ => {
                        let spec = reporter_spec(name).expect("get_variable has a reporter spec");
                        Ok(EntryInput::Block(Box::new(Self::build_spec_block(&spec, args, None, None, env)?)))
                    }
                }
            }
            Expression::Call { name, args } => {
                if let Some(function_id) = name.strip_prefix("func_") {
                    if Self::project_function_is_value(env.project, function_id) {
                        let mut block = EntryBlock::new(name.clone());
                        block.params = args.iter().map(|argument| Self::build_expression(argument, env))
                            .collect::<Result<Vec<_>, _>>()?;
                        return Ok(EntryInput::Block(Box::new(block)));
                    }
                }
                if let Some(function) = env.functions.get(name) {
                    if !function.returns_value {
                        return Err(CompileError::Semantic(format!("Void function '{}' cannot be used as a value", name)));
                    }
                    if args.len() != function.parameters.len() {
                        return Err(CompileError::Semantic(format!("Function '{}' expects {} argument(s), but {} were provided", name, function.parameters.len(), args.len())));
                    }
                    let mut block = EntryBlock::new(format!("func_{}", function.id));
                    block.params = args.iter().map(|argument| Self::build_expression(argument, env))
                        .collect::<Result<Vec<_>, _>>()?;
                    return Ok(EntryInput::Block(Box::new(block)));
                }
                if statement_spec(name).is_some() {
                    return Err(CompileError::Semantic(format!("'{}' is a statement block and cannot be used as a value", name)));
                }
                if let Some(spec) = reporter_spec(name) {
                    if spec.statement_count != 0 {
                        return Err(CompileError::Semantic(format!("'{}' is a statement block, not a reporter block", name)));
                    }
                    Ok(EntryInput::Block(Box::new(Self::build_spec_block(&spec, args, None, None, env)?)))
                } else {
                    Self::ensure_builtin_type(name)?;
                    Err(CompileError::Semantic(format!("Unknown block '{}'", name)))
                }
            }
            Expression::Unary { op, value } => {
                Ok(EntryInput::Block(Box::new(Self::build_unary_reporter(*op, value, env)?)))
            }
            Expression::Binary { left, op, right } => {
                Ok(EntryInput::Block(Box::new(Self::build_binary_reporter(left, *op, right, env)?)))
            }
        }
    }

    fn build_list_reporter(name: &str, args: &[Expression], env: &BuildEnv<'_>) -> Result<EntryInput, CompileError> {
        let block = match (name, args) {
            ("list_get", [Expression::Identifier(list), index])
            | ("list_get", [Expression::String(list), index]) => {
                let id = Self::find_list_id(list, env)?;
                EntryBlock::new_with_params("value_of_index_from_list", vec![
                    EntryInput::Empty, EntryInput::Field(id), EntryInput::Empty,
                    Self::build_expression(index, env)?, EntryInput::Empty,
                ])
            }
            ("list_length", [Expression::Identifier(list)])
            | ("list_length", [Expression::String(list)]) => {
                let id = Self::find_list_id(list, env)?;
                EntryBlock::new_with_params("length_of_list", vec![
                    EntryInput::Empty, EntryInput::Field(id), EntryInput::Empty,
                ])
            }
            ("list_contains", [Expression::Identifier(list), value])
            | ("list_contains", [Expression::String(list), value]) => {
                let id = Self::find_list_id(list, env)?;
                EntryBlock::new_with_params("is_included_in_list", vec![
                    EntryInput::Empty, EntryInput::Field(id), EntryInput::Empty,
                    Self::build_expression(value, env)?, EntryInput::Empty,
                ])
            }
            _ => return Err(CompileError::Semantic(format!("Invalid arguments for '{name}'"))),
        };
        Ok(EntryInput::Block(Box::new(block)))
    }

    fn field_argument(args: &[Expression]) -> Result<EntryInput, CompileError> {
        match args {
            [Expression::String(value)] | [Expression::Identifier(value)] => Ok(EntryInput::Field(value.clone())),
            _ => Err(CompileError::Semantic("field() expects exactly one string or identifier".into())),
        }
    }

    fn raw_argument(args: &[Expression]) -> Result<EntryInput, CompileError> {
        match args {
            [Expression::String(value)] => Ok(EntryInput::Raw(Value::String(value.clone()))),
            [Expression::Number(value)] => Ok(EntryInput::Raw(number_to_json(value)?)),
            [Expression::Boolean(value)] => Ok(EntryInput::Raw(Value::Bool(*value))),
            [Expression::Null] => Ok(EntryInput::Empty),
            [Expression::Unary { op: UnaryOp::Negate, value }] if matches!(value.as_ref(), Expression::Number(_)) => {
                if let Expression::Number(number) = value.as_ref() {
                    Ok(EntryInput::Raw(number_to_json(&format!("-{number}"))?))
                } else { unreachable!() }
            }
            _ => Err(CompileError::Semantic("raw() expects exactly one string, number, boolean, or null literal".into())),
        }
    }

    fn current_object_id<'a>(env: &BuildEnv<'a>) -> Option<&'a str> {
        env.project.get("objects").and_then(Value::as_array)
            .and_then(|objects| objects.iter().find(|object| {
                object.get("name").and_then(Value::as_str) == Some(env.current_object)
            }))
            .and_then(|object| object.get("id")).and_then(Value::as_str)
    }

    fn find_variable_id(name: &str, env: &BuildEnv<'_>) -> Result<String, CompileError> {
        Self::find_scoped_variable_id(name, env, false)
    }

    fn find_list_id(name: &str, env: &BuildEnv<'_>) -> Result<String, CompileError> {
        Self::find_scoped_variable_id(name, env, true)
    }

    fn find_scoped_variable_id(
        name: &str,
        env: &BuildEnv<'_>,
        expect_list: bool,
    ) -> Result<String, CompileError> {
        let variables = env.project.get("variables")
            .and_then(Value::as_array)
            .ok_or_else(|| CompileError::Project("Project 'variables' must be an array".into()))?;
        let owner_id = Self::current_object_id(env);
        let mut local_matches = Vec::new();
        let mut global_matches = Vec::new();

        for variable in variables {
            if variable.get("name").and_then(Value::as_str) != Some(name) {
                continue;
            }
            let is_list = variable.get("variableType").and_then(Value::as_str) == Some("list");
            if is_list != expect_list {
                continue;
            }

            if let (Some(current_owner), Some(variable_owner)) = (
                owner_id,
                variable.get("object").and_then(Value::as_str),
            ) {
                if variable_owner == current_owner {
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
            [variable] => variable.get("id")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| CompileError::Project(format!(
                    "Project variable '{name}' is missing a valid string id"
                ))),
            [] => Err(CompileError::Semantic(if expect_list {
                format!("Unknown list '{name}'")
            } else {
                format!("Unknown variable '{name}'")
            })),
            _ => Err(CompileError::Project(format!(
                "Variable name '{name}' is ambiguous in the selected scope"
            ))),
        }
    }

    fn build_variable_reporter(name: &str, env: &BuildEnv<'_>) -> Result<EntryInput, CompileError> {
        let id = Self::find_variable_id(name, env)?;
        Ok(EntryInput::Block(Box::new(EntryBlock::new_with_params("get_variable", vec![EntryInput::Field(id)]))))
    }

    fn one_identifier_argument<'a>(name: &str, args: &'a [Expression]) -> Result<&'a str, CompileError> {
        match args {
            [Expression::Identifier(value)] | [Expression::String(value)] => Ok(value),
            _ => Err(CompileError::Semantic(format!("{name} expects one variable name"))),
        }
    }

    fn build_unary_reporter(op: UnaryOp, value: &Expression, env: &BuildEnv<'_>) -> Result<EntryBlock, CompileError> {
        match op {
            UnaryOp::Not => Ok(EntryBlock::new_with_params("boolean_not", vec![
                EntryInput::Empty, Self::build_expression(value, env)?, EntryInput::Empty,
            ])),
            UnaryOp::Negate => Ok(EntryBlock::new_with_params("calc_basic", vec![
                EntryInput::Number("0".into()), EntryInput::Field("MINUS".into()),
                Self::build_expression(value, env)?,
            ])),
        }
    }

    fn build_binary_reporter(
        left: &Expression,
        op: BinaryOp,
        right: &Expression,
        env: &BuildEnv<'_>,
    ) -> Result<EntryBlock, CompileError> {
        let left = Self::build_expression(left, env)?;
        let right = Self::build_expression(right, env)?;
        let (type_id, operator) = match op {
            BinaryOp::Add => ("calc_basic", "PLUS"),
            BinaryOp::Subtract => ("calc_basic", "MINUS"),
            BinaryOp::Multiply => ("calc_basic", "MULTI"),
            BinaryOp::Divide => ("calc_basic", "DIVIDE"),
            BinaryOp::Modulo => ("quotient_and_mod", "MOD"),
            BinaryOp::Equal => ("boolean_basic_operator", "EQUAL"),
            BinaryOp::NotEqual => ("boolean_basic_operator", "NOT_EQUAL"),
            BinaryOp::Less => ("boolean_basic_operator", "LESS"),
            BinaryOp::LessEqual => ("boolean_basic_operator", "LESS_OR_EQUAL"),
            BinaryOp::Greater => ("boolean_basic_operator", "GREATER"),
            BinaryOp::GreaterEqual => ("boolean_basic_operator", "GREATER_OR_EQUAL"),
            BinaryOp::And => ("boolean_and_or", "AND"),
            BinaryOp::Or => ("boolean_and_or", "OR"),
        };
        let params = if op == BinaryOp::Modulo {
            vec![EntryInput::Empty, left, EntryInput::Empty, right,
                EntryInput::Empty, EntryInput::Field(operator.into())]
        } else {
            vec![left, EntryInput::Field(operator.into()), right]
        };
        Ok(EntryBlock::new_with_params(type_id, params))
    }

    fn fixed_input(value: &Value) -> EntryInput {
        match value {
            Value::Null => EntryInput::Empty,
            Value::String(text) => EntryInput::Field(text.clone()),
            _ => EntryInput::Raw(value.clone()),
        }
    }

    /// Turns a registry default into the same Entry input representation used
    /// for an explicitly supplied argument. Value-block defaults are described
    /// as {"type":"text"|"number", "params":[...]} and must receive a real
    /// Entry block ID during serialization, rather than being copied as raw
    /// JSON into the parent block's params array.
    fn default_input(encoding: ParamEncoding, value: &Value) -> EntryInput {
        if encoding == ParamEncoding::ValueBlock {
            if let (Some(block_type), Some(argument)) = (
                value.get("type").and_then(Value::as_str),
                value.get("params").and_then(Value::as_array).and_then(|params| params.first()),
            ) {
                match block_type {
                    "text" => {
                        if let Some(text) = argument.as_str() {
                            return EntryInput::Text(text.to_owned());
                        }
                    }
                    "number" => {
                        if let Some(number) = argument.as_str() {
                            return EntryInput::Number(number.to_owned());
                        }
                        if let Some(number) = argument.as_number() {
                            return EntryInput::Number(number.to_string());
                        }
                    }
                    "True" => return EntryInput::Boolean(true),
                    "False" => return EntryInput::Boolean(false),
                    _ => {}
                }
            }
        }

        Self::fixed_input(value)
    }

    fn ensure_builtin_type(type_id: &str) -> Result<(), CompileError> {
        let lower = type_id.to_ascii_lowercase();
        if lower.starts_with("block_expansion_") || lower.starts_with("expansion_")
            || lower.starts_with("block_ai_") || lower.starts_with("ai_") || lower.contains("ai_utilize")
        {
            Err(CompileError::Semantic(format!("Entry extension and AI blocks are not included: '{type_id}'")))
        } else { Ok(()) }
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
}

impl EntryBlock {
    fn new_with_params(type_id: impl Into<String>, params: Vec<EntryInput>) -> Self {
        let mut block = Self::new(type_id);
        block.params = params;
        block
    }
}

fn number_to_json(text: &str) -> Result<Value, CompileError> {
    if let Ok(value) = text.parse::<i64>() {
        Ok(json!(value))
    } else if let Ok(value) = text.parse::<f64>() {
        serde_json::Number::from_f64(value).map(Value::Number)
            .ok_or_else(|| CompileError::Semantic(format!("Invalid number '{text}'")))
    } else {
        Err(CompileError::Semantic(format!("Invalid number '{text}'")))
    }
}
