//! OpenAPI 3.x → [`ApiDefinition`] import (nested commands from URL path segments).

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use serde_json::Value;

use crate::error::{Result, YcallrError};
use crate::models::{
    ApiDefinition, ApiErrorConfig, ApiKeyLocation, AuthConfig, BodyConfig, Command, EnvVar,
    HttpMethod, ParamType, Parameter, ResponseConfig, ResponseEntry,
};

const HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete"];

/// How imported operations are grouped into nested ycallr commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OpenApiNestBy {
    /// Nest by static URL path segments (`repos.issues.list`).
    #[default]
    Path,
    /// Nest by the first OpenAPI tag on each operation (`issues.list`).
    Tag,
}

impl OpenApiNestBy {
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "path" | "paths" | "url" => Ok(Self::Path),
            "tag" | "tags" => Ok(Self::Tag),
            other => Err(YcallrError::OpenApiParse(format!(
                "Invalid nest-by value '{other}' (expected 'path' or 'tag')"
            ))),
        }
    }
}

/// Vendor-specific import defaults (headers, auth fallbacks).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OpenApiPreset {
    /// Detect GitHub/GHES from the document (default).
    #[default]
    Auto,
    /// Force GitHub REST headers and auth scaffold.
    GitHub,
    /// Do not apply vendor presets.
    None,
}

impl OpenApiPreset {
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "github" | "gh" | "ghes" => Ok(Self::GitHub),
            "none" | "off" => Ok(Self::None),
            other => Err(YcallrError::OpenApiParse(format!(
                "Invalid preset value '{other}' (expected 'auto', 'github', or 'none')"
            ))),
        }
    }
}

/// Options when converting OpenAPI to a ycallr profile scaffold.
#[derive(Debug, Clone)]
pub struct OpenApiImportOptions {
    /// Profile `name:` override (must be alphanumeric + dash). Defaults from `info.title`.
    pub name: Option<String>,
    /// Import only operations that include this OpenAPI tag.
    pub tag: Option<String>,
    /// Override `base_url:` (e.g. `https://github.mycompany.com/api/v3`).
    pub base_url: Option<String>,
    /// Group commands by URL path or OpenAPI tag.
    pub nest_by: OpenApiNestBy,
    /// Shorten leaf names by removing tag/path redundancy (`list-issues` → `list`).
    pub short_names: bool,
    /// Vendor preset for headers/auth (`auto` detects GitHub/GHES).
    pub preset: OpenApiPreset,
}

impl Default for OpenApiImportOptions {
    fn default() -> Self {
        Self {
            name: None,
            tag: None,
            base_url: None,
            nest_by: OpenApiNestBy::Path,
            short_names: false,
            preset: OpenApiPreset::Auto,
        }
    }
}

/// Parse OpenAPI JSON or YAML text into a JSON value tree.
pub fn parse_openapi_content(content: &str) -> Result<Value> {
    let trimmed = content.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        serde_json::from_str(content)
            .map_err(|e| YcallrError::OpenApiParse(format!("Invalid OpenAPI JSON: {e}")))
    } else {
        serde_yaml::from_str(content)
            .map_err(|e| YcallrError::OpenApiParse(format!("Invalid OpenAPI YAML: {e}")))
    }
}

/// Convert a parsed OpenAPI document into a ycallr [`ApiDefinition`].
pub fn import_openapi(doc: &Value, options: &OpenApiImportOptions) -> Result<ApiDefinition> {
    let openapi_version = doc
        .get("openapi")
        .or_else(|| doc.get("swagger"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if !openapi_version.starts_with("3.") && !openapi_version.starts_with("3") {
        return Err(YcallrError::OpenApiParse(format!(
            "Unsupported OpenAPI version '{openapi_version}' (expected 3.x)"
        )));
    }

    let info = doc.get("info").and_then(Value::as_object);
    let title = info
        .and_then(|i| i.get("title"))
        .and_then(Value::as_str)
        .unwrap_or("api");
    let version = info
        .and_then(|i| i.get("version"))
        .and_then(Value::as_str)
        .unwrap_or("1.0.0");
    let description = info
        .and_then(|i| i.get("description"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();

    let name = options.name.clone().unwrap_or_else(|| slugify_name(title));

    let base_url = options
        .base_url
        .clone()
        .unwrap_or_else(|| extract_base_url(doc));

    let (auth, env) = extract_auth(doc);
    let api_errors = extract_api_errors(doc);

    let mut operations = Vec::new();
    if let Some(paths) = doc.get("paths").and_then(Value::as_object) {
        for (path, path_item) in paths {
            let path_item = resolve_value(path_item, doc);
            collect_path_operations(&path_item, doc, path, api_errors.as_ref(), &mut operations);
        }
    }

    if let Some(tag) = &options.tag {
        operations.retain(|op| op.tags.iter().any(|t| t == tag));
    }

    if operations.is_empty() {
        return Err(YcallrError::OpenApiParse(
            "No operations found to import (check paths and --tag filter)".into(),
        ));
    }

    let use_short_names = options.short_names || options.nest_by == OpenApiNestBy::Tag;
    for op in &mut operations {
        apply_nesting(op, options.nest_by, use_short_names);
    }

    let commands = build_command_tree(operations);

    let mut api = ApiDefinition {
        name,
        version: version.to_string(),
        description,
        base_url: base_url.clone(),
        env,
        auth,
        commands,
        errors: api_errors,
    };

    apply_vendor_presets(doc, title, &base_url, options.preset, &mut api);

    api.validate_for_client()?;
    Ok(api)
}

/// Parse OpenAPI text and serialize the result as ycallr profile YAML.
pub fn import_openapi_to_yaml(content: &str, options: &OpenApiImportOptions) -> Result<String> {
    let doc = parse_openapi_content(content)?;
    let api = import_openapi(&doc, options)?;
    serde_yaml::to_string(&api)
        .map_err(|e| YcallrError::Serialization(format!("Failed to serialize YAML: {e}")))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn import_openapi_file(
    path: &std::path::Path,
    options: &OpenApiImportOptions,
) -> Result<String> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        YcallrError::OpenApiParse(format!("Failed to read {}: {e}", path.display()))
    })?;
    import_openapi_to_yaml(&content, options)
}

#[derive(Clone)]
struct OperationSpec {
    command_name: String,
    operation_id: Option<String>,
    static_segments: Vec<String>,
    method: HttpMethod,
    endpoint: String,
    description: Option<String>,
    params: HashMap<String, Parameter>,
    headers: HashMap<String, String>,
    body: Option<BodyConfig>,
    auth: Option<String>,
    tags: Vec<String>,
    responses: Option<ResponseConfig>,
}

fn collect_path_operations(
    path_item: &Value,
    root: &Value,
    path: &str,
    api_errors: Option<&ApiErrorConfig>,
    out: &mut Vec<OperationSpec>,
) {
    let path_level_params =
        merge_parameter_list(path_item.get("parameters").unwrap_or(&Value::Null), root);

    for method_name in HTTP_METHODS {
        let Some(op) = path_item.get(method_name) else {
            continue;
        };
        let op = resolve_value(op, root);
        if let Some(spec) =
            operation_to_spec(&op, root, path, method_name, &path_level_params, api_errors)
        {
            out.push(spec);
        }
    }
}

fn operation_to_spec(
    op: &Value,
    root: &Value,
    path: &str,
    method_name: &str,
    path_level_params: &[Value],
    api_errors: Option<&ApiErrorConfig>,
) -> Option<OperationSpec> {
    let method = parse_http_method(method_name)?;

    let description = op
        .get("summary")
        .or_else(|| op.get("description"))
        .and_then(Value::as_str)
        .map(|s| s.lines().next().unwrap_or(s).trim().to_string());

    let tags: Vec<String> = op
        .get("tags")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let mut params = HashMap::new();
    let mut headers = HashMap::new();

    let op_params = merge_parameter_list(op.get("parameters").unwrap_or(&Value::Null), root);
    for param in path_level_params.iter().chain(op_params.iter()) {
        apply_openapi_parameter(param, root, &mut params, &mut headers);
    }

    let body = extract_request_body(op.get("requestBody"), root, &mut params);

    let static_segments = path_segments(path)
        .into_iter()
        .filter(|seg| !is_path_param_segment(seg))
        .map(|seg| sanitize_segment(&seg))
        .collect::<Vec<_>>();

    let auth = op
        .get("security")
        .or_else(|| root.get("security"))
        .and_then(first_security_scheme_name);

    let operation_id = op
        .get("operationId")
        .and_then(Value::as_str)
        .map(str::to_string);
    let command_name = command_name_from_operation(op, &method);

    Some(OperationSpec {
        command_name,
        operation_id,
        static_segments,
        method,
        endpoint: path.to_string(),
        description,
        params,
        headers,
        body,
        auth,
        tags,
        responses: extract_responses(op, root, api_errors),
    })
}

fn build_command_tree(operations: Vec<OperationSpec>) -> HashMap<String, Command> {
    let mut commands = HashMap::new();
    for op in operations {
        insert_operation(&mut commands, &op);
    }
    commands
}

fn apply_nesting(op: &mut OperationSpec, nest_by: OpenApiNestBy, short_names: bool) {
    if nest_by == OpenApiNestBy::Tag {
        op.static_segments = nest_segments_from_tags(op);
    }

    if short_names {
        op.command_name = shorten_leaf_name(
            &op.command_name,
            &op.static_segments,
            op.operation_id.as_deref(),
        );
    }
}

fn nest_segments_from_tags(op: &OperationSpec) -> Vec<String> {
    if let Some(tag) = op.tags.first() {
        return vec![sanitize_segment(tag)];
    }
    if let Some(prefix) = op
        .operation_id
        .as_deref()
        .and_then(|oid| oid.split('/').next())
    {
        return vec![sanitize_segment(prefix)];
    }
    vec!["untagged".to_string()]
}

fn shorten_leaf_name(name: &str, segments: &[String], operation_id: Option<&str>) -> String {
    let mut short = name.to_string();

    if let Some(oid) = operation_id {
        if let Some((prefix, tail)) = oid.split_once('/') {
            let prefix_slug = slugify_name(prefix);
            if segments.first().is_some_and(|seg| seg == &prefix_slug) {
                short = slugify_name(tail);
            }
        }
    }

    for seg in segments {
        let seg_prefix = format!("{seg}-");
        if short.starts_with(&seg_prefix) {
            short = short[seg_prefix.len()..].to_string();
        }

        let embedded = format!("-{seg}-");
        if short.contains(&embedded) {
            short = short.replace(&embedded, "-");
        }

        if short.ends_with(&format!("-{seg}")) {
            short.truncate(short.len().saturating_sub(seg.len() + 1));
        }

        if seg.ends_with('s') {
            let singular = &seg[..seg.len() - 1];
            if short.ends_with(&format!("-{singular}")) {
                short.truncate(short.len().saturating_sub(singular.len() + 1));
            }
        }
    }

    while short.contains("--") {
        short = short.replace("--", "-");
    }

    let trimmed = short.trim_matches('-');
    if trimmed.is_empty() {
        name.to_string()
    } else {
        trimmed.to_string()
    }
}

fn insert_operation(commands: &mut HashMap<String, Command>, op: &OperationSpec) {
    let leaf_key = op.command_name.clone();
    if op.static_segments.is_empty() {
        merge_leaf(commands, leaf_key, op);
        return;
    }
    insert_at_segments(commands, &op.static_segments, 0, leaf_key, op);
}

fn insert_at_segments(
    commands: &mut HashMap<String, Command>,
    segments: &[String],
    idx: usize,
    leaf_key: String,
    op: &OperationSpec,
) {
    let segment = &segments[idx];
    let is_last = idx == segments.len() - 1;

    if !commands.contains_key(segment) {
        commands.insert(
            segment.clone(),
            Command {
                description: None,
                endpoint: None,
                method: None,
                auth: None,
                headers: HashMap::new(),
                params: HashMap::new(),
                body: None,
                responses: None,
                commands: Some(HashMap::new()),
            },
        );
    }

    let entry = commands.get_mut(segment).unwrap();
    if entry.commands.is_none() {
        entry.commands = Some(HashMap::new());
    }

    if is_last {
        merge_leaf(entry.commands.as_mut().unwrap(), leaf_key, op);
    } else {
        let children = entry.commands.as_mut().unwrap();
        insert_at_segments(children, segments, idx + 1, leaf_key, op);
    }
}

fn merge_leaf(commands: &mut HashMap<String, Command>, leaf_key: String, op: &OperationSpec) {
    if let Some(existing) = commands.get(&leaf_key) {
        if existing.endpoint.as_deref() == Some(op.endpoint.as_str())
            && existing.method == Some(op.method.clone())
        {
            return;
        }
        let alt_key = unique_command_key(commands, &leaf_key);
        commands.insert(alt_key, op.to_command());
    } else {
        commands.insert(leaf_key, op.to_command());
    }
}

impl OperationSpec {
    fn to_command(&self) -> Command {
        let mut headers = self.headers.clone();
        if self.body.is_some() && !headers.contains_key("Content-Type") {
            headers.insert("Content-Type".to_string(), "application/json".to_string());
        }

        Command {
            description: self.description.clone(),
            endpoint: Some(self.endpoint.clone()),
            method: Some(self.method.clone()),
            auth: self.auth.clone(),
            headers,
            params: self.params.clone(),
            body: self.body.clone(),
            responses: self.responses.clone(),
            commands: None,
        }
    }
}

fn apply_openapi_parameter(
    param: &Value,
    root: &Value,
    params: &mut HashMap<String, Parameter>,
    headers: &mut HashMap<String, String>,
) {
    let Some(name) = param.get("name").and_then(Value::as_str) else {
        return;
    };
    let location = param.get("in").and_then(Value::as_str).unwrap_or("query");
    let required = param
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let description = param
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or(name)
        .to_string();

    let schema = param.get("schema").unwrap_or(&Value::Null);
    let resolved_schema = resolve_value(schema, root);
    let param_type = schema_to_param_type(resolved_schema.as_ref());

    match location {
        "path" | "query" => {
            params.insert(
                name.to_string(),
                make_parameter(
                    description,
                    resolved_schema.as_ref(),
                    root,
                    param_type,
                    required || location == "path",
                ),
            );
        }
        "header" => {
            if let Some(default) = schema.get("default").and_then(Value::as_str) {
                headers.insert(name.to_string(), default.to_string());
            }
        }
        _ => {}
    }
}

fn extract_request_body(
    request_body: Option<&Value>,
    root: &Value,
    params: &mut HashMap<String, Parameter>,
) -> Option<BodyConfig> {
    let request_body = resolve_value(request_body?, root);
    let content = request_body.get("content")?.as_object()?;

    let (media_type, media) = content
        .iter()
        .find(|(mt, _)| mt.contains("json") || mt.contains("form"))
        .or_else(|| content.iter().next())?;

    let schema = resolve_value(media.get("schema").unwrap_or(&Value::Null), root);

    if media_type.contains("json") {
        let (json, body_params) = schema_to_json_template(&schema, root);
        for (name, param) in body_params {
            params.entry(name).or_insert(param);
        }
        return Some(BodyConfig {
            json: Some(json),
            form: None,
            multipart: None,
            raw: None,
        });
    }

    if media_type.contains("form") {
        let mut fields = HashMap::new();
        if let Some(props) = schema.get("properties").and_then(Value::as_object) {
            for (name, prop) in props {
                let prop = resolve_value(prop, root);
                let param_name = name.clone();
                fields.insert(name.clone(), format!("{{{param_name}}}"));
                let required = schema_field_required(&schema, name);
                params.entry(param_name.clone()).or_insert(make_parameter(
                    prop_description(&prop, &param_name),
                    prop.as_ref(),
                    root,
                    schema_to_param_type(&prop),
                    required,
                ));
            }
        }
        if fields.is_empty() {
            return None;
        }
        return Some(BodyConfig {
            json: None,
            form: Some(fields),
            multipart: None,
            raw: None,
        });
    }

    None
}

fn schema_to_json_template(schema: &Value, root: &Value) -> (Value, HashMap<String, Parameter>) {
    let mut params = HashMap::new();
    let resolved = resolve_value(schema, root);
    let json = schema_to_json_value(resolved.as_ref(), root, "", &resolved, &mut params, 0);
    (json, params)
}

fn schema_fallback_value(prefix: &str) -> Value {
    if prefix.is_empty() {
        Value::Object(serde_json::Map::new())
    } else {
        Value::String(format!("{{{prefix}}}"))
    }
}

fn schema_to_json_value(
    schema: &Value,
    root: &Value,
    prefix: &str,
    parent_schema: &Value,
    params: &mut HashMap<String, Parameter>,
    depth: usize,
) -> Value {
    if depth > 32 {
        return schema_fallback_value(prefix);
    }

    let schema = if schema.get("$ref").is_some() {
        resolve_value(schema, root).into_owned()
    } else {
        schema.clone()
    };

    let field_name = prefix.rsplit('.').next().unwrap_or(prefix);
    let required = if prefix.is_empty() {
        false
    } else {
        schema_field_required(parent_schema, field_name)
    };

    match schema.get("type").and_then(Value::as_str) {
        Some("object") => {
            let mut map = serde_json::Map::new();
            if let Some(props) = schema.get("properties").and_then(Value::as_object) {
                for (key, prop) in props {
                    let child_prefix = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{prefix}.{key}")
                    };
                    map.insert(
                        key.clone(),
                        schema_to_json_value(prop, root, &child_prefix, &schema, params, depth + 1),
                    );
                }
            }
            Value::Object(map)
        }
        Some("array") => {
            if prefix.is_empty() {
                return Value::Array(vec![]);
            }
            insert_schema_param(params, prefix, &schema, root, ParamType::Array, required);
            Value::Array(vec![Value::String(format!("{{{prefix}}}"))])
        }
        Some("integer") | Some("number") => {
            if !prefix.is_empty() {
                insert_schema_param(params, prefix, &schema, root, ParamType::Number, required);
            }
            schema_fallback_value(prefix)
        }
        Some("boolean") => {
            if !prefix.is_empty() {
                insert_schema_param(params, prefix, &schema, root, ParamType::Boolean, required);
            }
            schema_fallback_value(prefix)
        }
        _ => {
            if !prefix.is_empty() {
                insert_schema_param(params, prefix, &schema, root, ParamType::String, required);
            }
            schema_fallback_value(prefix)
        }
    }
}

fn insert_schema_param(
    params: &mut HashMap<String, Parameter>,
    name: &str,
    schema: &Value,
    root: &Value,
    param_type: ParamType,
    required: bool,
) {
    let mut description = prop_description(schema, name);
    if param_type == ParamType::Array {
        description = format!("{description} (comma-separated or JSON array)");
    }
    params.entry(name.to_string()).or_insert(make_parameter(
        description,
        schema,
        root,
        param_type,
        required,
    ));
}

fn make_parameter(
    description: String,
    schema: &Value,
    root: &Value,
    param_type: ParamType,
    required: bool,
) -> Parameter {
    Parameter {
        description,
        param_type,
        required,
        enum_values: extract_enum_values_from_schema(schema, root),
    }
}

fn extract_enum_values_from_schema(schema: &Value, root: &Value) -> Option<Vec<String>> {
    let schema = resolve_value(schema, root);
    if let Some(enum_values) = schema
        .get("enum")
        .and_then(enum_array_to_strings)
        .filter(|values| !values.is_empty())
    {
        return Some(enum_values);
    }

    for composite_key in ["allOf", "anyOf", "oneOf"] {
        if let Some(items) = schema.get(composite_key).and_then(Value::as_array) {
            for item in items {
                if let Some(enum_values) = extract_enum_values_from_schema(item, root) {
                    return Some(enum_values);
                }
            }
        }
    }

    None
}

fn enum_array_to_strings(value: &Value) -> Option<Vec<String>> {
    let arr = value.as_array()?;
    let mut out = Vec::new();
    for item in arr {
        match item {
            Value::String(s) => out.push(s.clone()),
            Value::Number(n) => out.push(n.to_string()),
            Value::Bool(b) => out.push(b.to_string()),
            _ => {}
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn schema_field_required(schema: &Value, field: &str) -> bool {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|required| required.iter().any(|name| name.as_str() == Some(field)))
        .unwrap_or(false)
}

fn command_name_from_operation(op: &Value, method: &HttpMethod) -> String {
    if let Some(operation_id) = op.get("operationId").and_then(Value::as_str) {
        let tail = operation_id.rsplit('/').next().unwrap_or(operation_id);
        let slug = slugify_name(tail);
        if !slug.is_empty() {
            return slug;
        }
    }
    http_method_key(method)
}

fn extract_responses(
    op: &Value,
    root: &Value,
    api_errors: Option<&ApiErrorConfig>,
) -> Option<ResponseConfig> {
    let responses = op.get("responses")?.as_object()?;

    let mut success = None;
    let mut failure = None;
    let mut warn = None;
    let mut codes = HashMap::new();
    let mut default_error_template = None;

    for (code, response) in responses {
        let resolved = resolve_value(response, root);
        let status = parse_response_status_code(code);

        if code == "default" {
            if let Some(template) = error_message_template_from_response(&resolved, root) {
                default_error_template = Some(template);
            }
            continue;
        }

        let Some(status) = status else {
            continue;
        };

        if (200..300).contains(&status) {
            let template = success_message_template_from_response(&resolved, root)
                .unwrap_or_else(|| "{status}".to_string());
            if success.is_none() {
                success = Some(ResponseEntry { message: template });
            }
            continue;
        }

        if (300..400).contains(&status) {
            let template = success_message_template_from_response(&resolved, root)
                .unwrap_or_else(|| "Redirect ({status})".to_string());
            if warn.is_none() {
                warn = Some(ResponseEntry { message: template });
            }
            continue;
        }

        if !(400..600).contains(&status) {
            continue;
        }

        let template = error_message_template_from_response(&resolved, root)
            .unwrap_or_else(|| "Error {status}".to_string());

        if api_errors
            .and_then(|errors| errors.get_entry_for_status(status))
            .is_some_and(|entry| entry.message == template)
        {
            continue;
        }

        if api_errors
            .and_then(|e| e.default.as_ref())
            .is_some_and(|entry| entry.message == template)
            && !api_errors.is_some_and(|e| e.codes.contains_key(code))
        {
            continue;
        }

        codes.insert(code.clone(), ResponseEntry { message: template });
    }

    if failure.is_none() {
        if let Some(template) = default_error_template {
            let use_failure = !api_errors
                .and_then(|e| e.default.as_ref())
                .is_some_and(|entry| entry.message == template);
            if use_failure {
                failure = Some(ResponseEntry { message: template });
            }
        } else if codes.is_empty() {
            if let Some(first) = ["400", "401", "403", "404", "422", "500"]
                .iter()
                .find_map(|code| responses.get(*code))
            {
                let resolved = resolve_value(first, root);
                if let Some(template) = error_message_template_from_response(&resolved, root) {
                    let use_failure = !api_errors
                        .and_then(|e| e.default.as_ref())
                        .is_some_and(|entry| entry.message == template);
                    if use_failure {
                        failure = Some(ResponseEntry { message: template });
                    }
                } else if api_errors.is_none() {
                    failure = Some(ResponseEntry {
                        message: "Error {status}".to_string(),
                    });
                }
            } else if api_errors.is_none() {
                failure = Some(ResponseEntry {
                    message: "Error {status}".to_string(),
                });
            }
        }
    }

    if success.is_none() && failure.is_none() && warn.is_none() && codes.is_empty() {
        return None;
    }

    Some(ResponseConfig {
        success,
        failure,
        warn,
        codes,
    })
}

fn parse_response_status_code(code: &str) -> Option<u16> {
    if code.len() != 3 || !code.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    code.parse().ok()
}

fn response_json_schema<'a>(response: &'a Value, root: &'a Value) -> Option<Cow<'a, Value>> {
    let content = response.get("content")?.as_object()?;
    let (_, media) = content
        .iter()
        .find(|(media_type, _)| media_type.contains("json") || media_type.contains("problem"))
        .or_else(|| content.iter().next())?;
    let schema = media.get("schema")?;
    Some(resolve_value(schema, root))
}

fn error_message_template_from_response(response: &Value, root: &Value) -> Option<String> {
    if let Some(schema) = response_json_schema(response, root) {
        if let Some(template) = error_message_template_from_schema(schema.as_ref(), root) {
            return Some(template);
        }
    }

    response
        .get("description")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|desc| !desc.is_empty())
        .map(|desc| {
            if desc.contains("{output.") || desc.contains("{status}") {
                desc.to_string()
            } else {
                format!("{desc} ({{status}})")
            }
        })
}

fn success_message_template_from_response(response: &Value, root: &Value) -> Option<String> {
    let schema = response_json_schema(response, root)?;
    success_message_template_from_schema(schema.as_ref(), root)
}

fn error_message_template_from_schema(schema: &Value, root: &Value) -> Option<String> {
    let schema = resolve_value(schema, root);
    if let Some(template) = validation_error_template(&schema) {
        return Some(template);
    }

    for key in ["message", "detail", "error", "title"] {
        if schema_has_string_property(schema.as_ref(), key) {
            return Some(format!("{{output.{key}}}"));
        }
    }

    None
}

fn validation_error_template(schema: &Value) -> Option<String> {
    let errors = schema.get("properties")?.get("errors")?;
    let items = errors.get("items")?;
    let item_props = items.get("properties")?.as_object()?;

    if !item_props.contains_key("message") {
        return None;
    }

    if schema_has_string_property(schema, "message") {
        if item_props.contains_key("field") {
            return Some(
                "{output.message} ({output.errors.0.field}: {output.errors.0.message})".into(),
            );
        }
        return Some("{output.message} ({output.errors.0.message})".into());
    }

    if item_props.contains_key("field") {
        return Some("{output.errors.0.field}: {output.errors.0.message}".into());
    }

    Some("{output.errors.0.message}".into())
}

fn success_message_template_from_schema(schema: &Value, root: &Value) -> Option<String> {
    let schema = resolve_value(schema, root);
    if schema.get("type").and_then(Value::as_str) == Some("array") {
        if let Some(items) = schema.get("items") {
            return success_message_template_from_object_schema(items, root, "0");
        }
        return None;
    }
    success_message_template_from_object_schema(schema.as_ref(), root, "")
}

fn success_message_template_from_object_schema(
    schema: &Value,
    root: &Value,
    prefix: &str,
) -> Option<String> {
    let schema = resolve_value(schema, root);
    let props = schema.get("properties")?.as_object()?;

    let field_path = |name: &str| {
        if prefix.is_empty() {
            format!("{{output.{name}}}")
        } else {
            format!("{{output.{prefix}.{name}}}")
        }
    };

    if props.contains_key("number") && props.contains_key("title") {
        return Some(format!(
            "#{}: {}",
            field_path("number"),
            field_path("title")
        ));
    }

    if props.contains_key("id") && props.contains_key("name") {
        return Some(format!("{} ({})", field_path("name"), field_path("id")));
    }

    for key in [
        "title", "name", "login", "number", "id", "key", "sha", "url",
    ] {
        if props.contains_key(key) {
            return Some(field_path(key));
        }
    }

    None
}

fn schema_has_string_property(schema: &Value, key: &str) -> bool {
    schema
        .get("properties")
        .and_then(|props| props.get(key))
        .and_then(|prop| prop.get("type"))
        .and_then(Value::as_str)
        .is_some_and(|ty| ty == "string")
}

fn is_error_schema_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("error") || lower.contains("problem") || lower == "rpc_status"
}

fn extract_api_errors(doc: &Value) -> Option<ApiErrorConfig> {
    let schemas = doc
        .get("components")
        .and_then(|c| c.get("schemas"))
        .and_then(Value::as_object)?;

    let mut config = ApiErrorConfig::default();
    let mut validation_template = None;

    for (name, schema) in schemas {
        if !is_error_schema_name(name) && !looks_like_error_schema(schema) {
            continue;
        }
        let Some(template) = error_message_template_from_schema(schema, doc) else {
            continue;
        };

        let lower = name.to_ascii_lowercase();
        if lower.contains("validation") {
            validation_template = Some(template);
        } else if config.default.is_none() {
            config.default = Some(ResponseEntry { message: template });
        }
    }

    if let Some(template) = validation_template {
        config
            .codes
            .entry("422".to_string())
            .or_insert(ResponseEntry { message: template });
    }

    if config.default.is_none() && config.codes.is_empty() {
        None
    } else {
        Some(config)
    }
}

fn looks_like_error_schema(schema: &Value) -> bool {
    ["message", "detail", "error"]
        .iter()
        .any(|key| schema_has_string_property(schema, key))
}

fn extract_auth(doc: &Value) -> (HashMap<String, AuthConfig>, Vec<EnvVar>) {
    let mut auth = HashMap::new();
    let mut env = Vec::new();
    let mut seen_env = HashSet::new();

    let Some(schemes) = doc
        .get("components")
        .and_then(|c| c.get("securitySchemes"))
        .and_then(Value::as_object)
    else {
        return (auth, env);
    };

    for (name, scheme) in schemes {
        let scheme = resolve_value(scheme, doc);
        let scheme_type = scheme.get("type").and_then(Value::as_str).unwrap_or("");

        match scheme_type {
            "http" => {
                let http_scheme = scheme
                    .get("scheme")
                    .and_then(Value::as_str)
                    .unwrap_or("bearer")
                    .to_ascii_lowercase();
                if http_scheme == "bearer" {
                    let env_name = format!("{}_TOKEN", name.to_ascii_uppercase().replace('-', "_"));
                    push_env(&mut env, &mut seen_env, &env_name, true);
                    auth.insert(
                        name.clone(),
                        AuthConfig::Bearer {
                            token: format!("${{{env_name}}}"),
                        },
                    );
                } else if http_scheme == "basic" {
                    let user_env =
                        format!("{}_USERNAME", name.to_ascii_uppercase().replace('-', "_"));
                    let pass_env =
                        format!("{}_PASSWORD", name.to_ascii_uppercase().replace('-', "_"));
                    push_env(&mut env, &mut seen_env, &user_env, true);
                    push_env(&mut env, &mut seen_env, &pass_env, true);
                    auth.insert(
                        name.clone(),
                        AuthConfig::Http {
                            scheme: "basic".to_string(),
                            token: None,
                            username: Some(format!("${{{user_env}}}")),
                            password: Some(format!("${{{pass_env}}}")),
                            prefix: None,
                        },
                    );
                }
            }
            "apiKey" => {
                let key_name = scheme
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("X-API-Key");
                let location = match scheme.get("in").and_then(Value::as_str) {
                    Some("query") => ApiKeyLocation::Query,
                    Some("cookie") => ApiKeyLocation::Cookie,
                    _ => ApiKeyLocation::Header,
                };
                let env_name = format!("{}_KEY", name.to_ascii_uppercase().replace('-', "_"));
                push_env(&mut env, &mut seen_env, &env_name, true);
                auth.insert(
                    name.clone(),
                    AuthConfig::ApiKey {
                        key: format!("${{{env_name}}}"),
                        name: key_name.to_string(),
                        in_: location,
                    },
                );
            }
            "oauth2" | "openIdConnect" => {
                let env_name = format!("{}_TOKEN", name.to_ascii_uppercase().replace('-', "_"));
                push_env(&mut env, &mut seen_env, &env_name, true);
                auth.insert(
                    name.clone(),
                    AuthConfig::Bearer {
                        token: format!("${{{env_name}}}"),
                    },
                );
            }
            _ => {}
        }
    }

    (auth, env)
}

fn push_env(env: &mut Vec<EnvVar>, seen: &mut HashSet<String>, name: &str, required: bool) {
    if seen.insert(name.to_string()) {
        env.push(EnvVar {
            name: name.to_string(),
            required,
        });
    }
}

const GITHUB_ACCEPT_HEADER: &str = "application/vnd.github.v3+json";

fn apply_vendor_presets(
    doc: &Value,
    title: &str,
    base_url: &str,
    preset: OpenApiPreset,
    api: &mut ApiDefinition,
) {
    let use_github = match preset {
        OpenApiPreset::GitHub => true,
        OpenApiPreset::None => false,
        OpenApiPreset::Auto => detect_github_api(doc, title, base_url),
    };

    if use_github {
        apply_github_api_defaults(api);
    }
}

fn detect_github_api(doc: &Value, title: &str, base_url: &str) -> bool {
    let title_lower = title.to_ascii_lowercase();
    if title_lower.contains("github") {
        return true;
    }

    let base_lower = base_url.to_ascii_lowercase();
    if base_lower.contains("api.github.com") {
        return true;
    }

    if base_lower.contains("/api/v3") && doc_has_github_paths(doc) {
        return true;
    }

    doc_server_urls(doc).any(|url| {
        let lower = url.to_ascii_lowercase();
        lower.contains("api.github.com") || lower.contains("/api/v3")
    })
}

fn doc_server_urls(doc: &Value) -> impl Iterator<Item = String> + '_ {
    doc.get("servers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|server| server.get("url").and_then(Value::as_str))
        .map(str::to_string)
}

fn doc_has_github_paths(doc: &Value) -> bool {
    doc.get("paths")
        .and_then(Value::as_object)
        .is_some_and(|paths| paths.keys().any(|path| path.starts_with("/repos/")))
}

fn apply_github_api_defaults(api: &mut ApiDefinition) {
    apply_github_auth(&mut api.auth, &mut api.env);
    apply_default_headers_to_leaves(&mut api.commands, github_default_headers());
    if api.auth.contains_key("github") {
        link_commands_to_auth(&mut api.commands, "github");
    } else if api.auth.len() == 1 {
        let scheme = api.auth.keys().next().expect("single auth scheme").clone();
        link_commands_to_auth(&mut api.commands, &scheme);
    }
}

fn github_default_headers() -> HashMap<String, String> {
    HashMap::from([("Accept".to_string(), GITHUB_ACCEPT_HEADER.to_string())])
}

fn apply_github_auth(auth: &mut HashMap<String, AuthConfig>, env: &mut Vec<EnvVar>) {
    if auth.contains_key("github") {
        return;
    }

    let has_bearer = auth
        .values()
        .any(|cfg| matches!(cfg, AuthConfig::Bearer { .. }));
    if has_bearer {
        return;
    }

    auth.insert(
        "github".to_string(),
        AuthConfig::Bearer {
            token: "${GITHUB_TOKEN}".to_string(),
        },
    );

    if !env.iter().any(|var| var.name == "GITHUB_TOKEN") {
        env.push(EnvVar {
            name: "GITHUB_TOKEN".to_string(),
            required: true,
        });
    }
}

fn apply_default_headers_to_leaves(
    commands: &mut HashMap<String, Command>,
    headers: HashMap<String, String>,
) {
    for cmd in commands.values_mut() {
        if let Some(children) = cmd.commands.as_mut() {
            apply_default_headers_to_leaves(children, headers.clone());
            continue;
        }

        if !cmd.is_leaf() {
            continue;
        }

        for (name, value) in &headers {
            cmd.headers
                .entry(name.clone())
                .or_insert_with(|| value.clone());
        }
    }
}

fn link_commands_to_auth(commands: &mut HashMap<String, Command>, scheme: &str) {
    for cmd in commands.values_mut() {
        if let Some(children) = cmd.commands.as_mut() {
            link_commands_to_auth(children, scheme);
            continue;
        }

        if cmd.is_leaf() && cmd.auth.is_none() {
            cmd.auth = Some(scheme.to_string());
        }
    }
}

fn extract_base_url(doc: &Value) -> String {
    let Some(server) = doc
        .get("servers")
        .and_then(Value::as_array)
        .and_then(|servers| servers.first())
    else {
        return default_base_url_placeholder();
    };

    let Some(url_template) = server.get("url").and_then(Value::as_str) else {
        return default_base_url_placeholder();
    };

    let resolved = resolve_server_url(url_template, server.get("variables"));
    if resolved.is_empty() {
        default_base_url_placeholder()
    } else {
        resolved
    }
}

fn resolve_server_url(url_template: &str, variables: Option<&Value>) -> String {
    let mut url = url_template.to_string();
    if let Some(vars) = variables.and_then(Value::as_object) {
        for (name, spec) in vars {
            let default = spec
                .get("default")
                .and_then(Value::as_str)
                .unwrap_or("example.com");
            url = url.replace(&format!("{{{name}}}"), default);
        }
    }

    // Replace unresolved `{var}` placeholders.
    while let Some(start) = url.find('{') {
        let Some(rel_end) = url[start + 1..].find('}') else {
            break;
        };
        let end = start + 1 + rel_end;
        url.replace_range(start..=end, "example.com");
    }

    url.trim_end_matches('/').to_string()
}

fn default_base_url_placeholder() -> String {
    "https://api.example.com".to_string()
}

fn first_security_scheme_name(security: &Value) -> Option<String> {
    security
        .as_array()?
        .first()?
        .as_object()?
        .keys()
        .next()
        .cloned()
}

fn merge_parameter_list(value: &Value, root: &Value) -> Vec<Value> {
    match value {
        Value::Array(arr) => arr
            .iter()
            .map(|p| resolve_value(p, root))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|cow| match cow {
                Cow::Borrowed(v) => v.clone(),
                Cow::Owned(v) => v,
            })
            .collect(),
        Value::Object(obj) if obj.contains_key("$ref") => {
            let resolved = resolve_value(value, root);
            merge_parameter_list(resolved.as_ref(), root)
        }
        _ => Vec::new(),
    }
}

fn resolve_value<'a>(value: &'a Value, root: &'a Value) -> Cow<'a, Value> {
    resolve_value_depth(value, root, 0).unwrap_or(Cow::Borrowed(value))
}

fn resolve_value_depth<'a>(
    value: &'a Value,
    root: &'a Value,
    depth: usize,
) -> Option<Cow<'a, Value>> {
    if depth > 32 {
        return None;
    }
    let ref_path = value.get("$ref").and_then(Value::as_str)?;
    if !ref_path.starts_with("#/") {
        return None;
    }
    let mut current = root;
    for segment in ref_path[2..].split('/') {
        let decoded = segment.replace("~1", "/").replace("~0", "~");
        current = current.get(decoded.as_str())?;
    }
    resolve_value_depth(current, root, depth + 1).or(Some(Cow::Borrowed(current)))
}

fn path_segments(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn is_path_param_segment(segment: &str) -> bool {
    segment.starts_with('{') && segment.ends_with('}')
}

fn sanitize_segment(segment: &str) -> String {
    let mut out = segment
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}

fn slugify_name(title: &str) -> String {
    let slug: String = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let mut slug = slug.trim_matches('-').to_string();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    if slug.is_empty() {
        "api".to_string()
    } else {
        slug
    }
}

fn parse_http_method(method: &str) -> Option<HttpMethod> {
    match method.to_ascii_uppercase().as_str() {
        "GET" => Some(HttpMethod::GET),
        "POST" => Some(HttpMethod::POST),
        "PUT" => Some(HttpMethod::PUT),
        "DELETE" => Some(HttpMethod::DELETE),
        "PATCH" => Some(HttpMethod::PATCH),
        _ => None,
    }
}

fn http_method_key(method: &HttpMethod) -> String {
    match method {
        HttpMethod::GET => "get".to_string(),
        HttpMethod::POST => "post".to_string(),
        HttpMethod::PUT => "put".to_string(),
        HttpMethod::DELETE => "delete".to_string(),
        HttpMethod::PATCH => "patch".to_string(),
    }
}

fn unique_command_key(commands: &HashMap<String, Command>, base: &str) -> String {
    let mut n = 2;
    loop {
        let candidate = format!("{base}-{n}");
        if !commands.contains_key(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

fn schema_to_param_type(schema: &Value) -> ParamType {
    match schema.get("type").and_then(Value::as_str) {
        Some("integer") | Some("number") => ParamType::Number,
        Some("boolean") => ParamType::Boolean,
        Some("array") => ParamType::Array,
        _ => ParamType::String,
    }
}

fn prop_description(schema: &Value, fallback: &str) -> String {
    schema
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .lines()
        .next()
        .unwrap_or(fallback)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_err_contains<T>(result: Result<T>, needle: &str) {
        match result {
            Err(err) => assert!(
                err.to_string().contains(needle),
                "expected error containing '{needle}', got: {err}"
            ),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }

    const MINIMAL_OPENAPI: &str = r#"
openapi: 3.0.3
info:
  title: GitHub REST API
  version: "1.0.0"
  description: GitHub API example
servers:
  - url: https://api.github.com
paths:
  /repos/{owner}/{repo}/issues:
    get:
      summary: List issues
      operationId: repos/list-issues
      tags: [issues]
      parameters:
        - name: owner
          in: path
          required: true
          schema: { type: string }
        - name: repo
          in: path
          required: true
          schema: { type: string }
        - name: state
          in: query
          schema: { type: string }
    post:
      summary: Create issue
      operationId: repos/create-issue
      tags: [issues]
      parameters:
        - name: owner
          in: path
          required: true
          schema: { type: string }
        - name: repo
          in: path
          required: true
          schema: { type: string }
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [title]
              properties:
                title: { type: string }
                body: { type: string }
components:
  securitySchemes:
    github:
      type: http
      scheme: bearer
security:
  - github: []
"#;

    #[test]
    fn test_import_nested_path_commands() {
        let doc = parse_openapi_content(MINIMAL_OPENAPI).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                name: Some("github".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(api.name, "github");
        assert_eq!(api.base_url, "https://api.github.com");
        assert!(api.commands.contains_key("repos"));

        let repos = api.commands.get("repos").unwrap();
        let issues = repos.commands.as_ref().unwrap().get("issues").unwrap();
        let get = issues
            .commands
            .as_ref()
            .unwrap()
            .get("list-issues")
            .unwrap();
        assert_eq!(
            get.endpoint.as_deref(),
            Some("/repos/{owner}/{repo}/issues")
        );
        assert_eq!(get.method.as_ref(), Some(&HttpMethod::GET));
        assert!(get.params.contains_key("owner"));
        assert!(get.params.contains_key("state"));

        let post = issues
            .commands
            .as_ref()
            .unwrap()
            .get("create-issue")
            .unwrap();
        assert!(post.body.is_some());
        assert!(post.params.get("title").is_some_and(|p| p.required));
        assert!(!post.params.get("body").is_some_and(|p| p.required));
    }

    #[test]
    fn test_import_nested_tag_commands() {
        let doc = parse_openapi_content(MINIMAL_OPENAPI).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                name: Some("github".to_string()),
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(api.commands.contains_key("issues"));
        assert!(!api.commands.contains_key("repos"));

        let issues = api.commands.get("issues").unwrap();
        let list = issues.commands.as_ref().unwrap().get("list").unwrap();
        assert_eq!(
            list.endpoint.as_deref(),
            Some("/repos/{owner}/{repo}/issues")
        );
        assert_eq!(list.method.as_ref(), Some(&HttpMethod::GET));

        let create = issues.commands.as_ref().unwrap().get("create").unwrap();
        assert_eq!(
            create.endpoint.as_deref(),
            Some("/repos/{owner}/{repo}/issues")
        );
        assert_eq!(create.method.as_ref(), Some(&HttpMethod::POST));
    }

    #[test]
    fn test_import_operation_id_and_nested_body() {
        const ADMIN_HOOKS: &str = include_str!("../examples/github_admin_hooks.openapi.yaml");
        let doc = parse_openapi_content(ADMIN_HOOKS).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                name: Some("github-enterprise".to_string()),
                tag: Some("enterprise-admin".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        let hooks = api
            .commands
            .get("admin")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("hooks")
            .unwrap()
            .commands
            .as_ref()
            .unwrap();

        assert!(hooks.contains_key("list-global-webhooks"));
        assert!(hooks.contains_key("create-global-webhook"));
        assert!(hooks.contains_key("get-global-webhook"));
        assert!(!hooks.contains_key("get-2"));

        let create = hooks.get("create-global-webhook").unwrap();
        assert!(create.params.get("name").is_some_and(|p| p.required));
        assert!(create.params.get("config.url").is_some_and(|p| p.required));
        assert!(create.params.contains_key("events"));
        assert_eq!(
            create.params.get("events").unwrap().param_type,
            ParamType::Array
        );
    }

    #[test]
    fn test_import_tag_filter() {
        let doc = parse_openapi_content(MINIMAL_OPENAPI).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                name: Some("github".to_string()),
                tag: Some("issues".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(api.commands.contains_key("repos"));

        let result = import_openapi(
            &doc,
            &OpenApiImportOptions {
                name: Some("github".to_string()),
                tag: Some("nonexistent".to_string()),
                ..Default::default()
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_import_auth_and_env() {
        let doc = parse_openapi_content(MINIMAL_OPENAPI).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                name: Some("github".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(api.auth.contains_key("github"));
        assert!(api.env.iter().any(|e| e.name == "GITHUB_TOKEN"));
    }

    #[test]
    fn test_import_ghes_server_template_base_url() {
        const GHES_SERVERS: &str = r#"
openapi: 3.0.3
info:
  title: GitHub v3 REST API
  version: "1.0.0"
servers:
  - url: "{protocol}://{hostname}/api/v3"
    variables:
      hostname:
        default: HOSTNAME
      protocol:
        default: http
paths:
  /user:
    get:
      operationId: users/get-authenticated
      responses:
        '200':
          description: ok
"#;
        let doc = parse_openapi_content(GHES_SERVERS).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                name: Some("gh".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(api.base_url, "http://HOSTNAME/api/v3");
    }

    #[test]
    fn test_import_openapi_to_yaml_roundtrip() {
        let yaml = import_openapi_to_yaml(
            MINIMAL_OPENAPI,
            &OpenApiImportOptions {
                name: Some("github".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(yaml.contains("name: github"));
        assert!(yaml.contains("repos:"));
        let reparsed = crate::yaml_parser::parse_yaml_for_client(&yaml).unwrap();
        assert_eq!(reparsed.name, "github");
    }

    #[test]
    fn test_nest_by_parse() {
        assert_eq!(OpenApiNestBy::parse("path").unwrap(), OpenApiNestBy::Path);
        assert_eq!(OpenApiNestBy::parse("PATH").unwrap(), OpenApiNestBy::Path);
        assert_eq!(OpenApiNestBy::parse("url").unwrap(), OpenApiNestBy::Path);
        assert_eq!(OpenApiNestBy::parse("tag").unwrap(), OpenApiNestBy::Tag);
        assert_eq!(OpenApiNestBy::parse(" tags ").unwrap(), OpenApiNestBy::Tag);
        assert!(OpenApiNestBy::parse("hybrid").is_err());
    }

    #[test]
    fn test_tag_nesting_uses_first_tag_when_multiple() {
        const MULTI_TAG: &str = r#"
openapi: 3.0.3
info:
  title: Multi Tag API
  version: "1.0.0"
servers:
  - url: https://api.example.com
paths:
  /items:
    get:
      operationId: items/list
      tags: [catalog, search]
      responses:
        '200':
          description: ok
"#;
        let doc = parse_openapi_content(MULTI_TAG).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(api.commands.contains_key("catalog"));
        assert!(!api.commands.contains_key("search"));
        let catalog = api.commands.get("catalog").unwrap();
        assert!(catalog.commands.as_ref().unwrap().contains_key("list"));
    }

    #[test]
    fn test_tag_nesting_sanitizes_tag_names() {
        const SPACED_TAG: &str = r#"
openapi: 3.0.3
info:
  title: Spaced Tags
  version: "1.0.0"
servers:
  - url: https://api.example.com
paths:
  /ping:
    get:
      operationId: ping/get
      tags: ["Enterprise Admin"]
      responses:
        '200':
          description: ok
"#;
        let doc = parse_openapi_content(SPACED_TAG).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(api.commands.contains_key("enterprise-admin"));
        assert!(!api.commands.contains_key("Enterprise Admin"));
    }

    #[test]
    fn test_tag_nesting_untagged_uses_operation_id_prefix() {
        const UNTAGGED: &str = r#"
openapi: 3.0.3
info:
  title: Untagged API
  version: "1.0.0"
servers:
  - url: https://api.example.com
paths:
  /repos/{owner}/{repo}/issues:
    get:
      operationId: repos/list-issues
      responses:
        '200':
          description: ok
"#;
        let doc = parse_openapi_content(UNTAGGED).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(api.commands.contains_key("repos"));
        assert!(!api.commands.contains_key("untagged"));
        let repos = api.commands.get("repos").unwrap();
        assert!(repos.commands.as_ref().unwrap().contains_key("list-issues"));
    }

    #[test]
    fn test_tag_nesting_untagged_without_operation_id() {
        const UNTAGGED: &str = r#"
openapi: 3.0.3
info:
  title: Untagged API
  version: "1.0.0"
servers:
  - url: https://api.example.com
paths:
  /health:
    get:
      summary: Health check
      responses:
        '200':
          description: ok
"#;
        let doc = parse_openapi_content(UNTAGGED).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(api.commands.contains_key("untagged"));
        let untagged = api.commands.get("untagged").unwrap();
        assert!(untagged.commands.as_ref().unwrap().contains_key("get"));
    }

    #[test]
    fn test_tag_filter_with_tag_nesting() {
        let doc = parse_openapi_content(MINIMAL_OPENAPI).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                name: Some("github".to_string()),
                tag: Some("issues".to_string()),
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(api.commands.len(), 1);
        assert!(api.commands.contains_key("issues"));
        assert!(!api.commands.contains_key("repos"));
        let issues = api.commands.get("issues").unwrap();
        let cmds = issues.commands.as_ref().unwrap();
        assert_eq!(cmds.len(), 2);
        assert!(cmds.contains_key("list"));
        assert!(cmds.contains_key("create"));
    }

    #[test]
    fn test_tag_nesting_admin_hooks_example() {
        const ADMIN_HOOKS: &str = include_str!("../examples/github_admin_hooks.openapi.yaml");
        let doc = parse_openapi_content(ADMIN_HOOKS).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                name: Some("github-enterprise".to_string()),
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(api.commands.contains_key("enterprise-admin"));
        assert!(!api.commands.contains_key("admin"));

        let hooks = api
            .commands
            .get("enterprise-admin")
            .unwrap()
            .commands
            .as_ref()
            .unwrap();
        assert!(hooks.contains_key("list-global-webhooks"));
        assert!(hooks.contains_key("create-global-webhook"));
        assert!(hooks.contains_key("get-global-webhook"));
    }

    #[test]
    fn test_tag_nesting_resolves_leaf_name_collision() {
        const COLLISION: &str = r#"
openapi: 3.0.3
info:
  title: Collision API
  version: "1.0.0"
servers:
  - url: https://api.example.com
paths:
  /a/items:
    get:
      operationId: items/list
      tags: [items]
      responses:
        '200':
          description: ok
  /b/items:
    get:
      operationId: items/list
      tags: [items]
      responses:
        '200':
          description: ok
"#;
        let doc = parse_openapi_content(COLLISION).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();

        let items = api.commands.get("items").unwrap();
        let cmds = items.commands.as_ref().unwrap();
        assert!(cmds.contains_key("list"));
        assert!(cmds.contains_key("list-2"));
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn test_short_names_with_path_nesting() {
        let doc = parse_openapi_content(MINIMAL_OPENAPI).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                name: Some("github".to_string()),
                nest_by: OpenApiNestBy::Path,
                short_names: true,
                ..Default::default()
            },
        )
        .unwrap();

        let issues = api
            .commands
            .get("repos")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("issues")
            .unwrap()
            .commands
            .as_ref()
            .unwrap();
        assert!(issues.contains_key("list"));
        assert!(issues.contains_key("create"));
        assert!(!issues.contains_key("list-issues"));
    }

    #[test]
    fn test_import_openapi_to_yaml_roundtrip_tag_nesting() {
        let yaml = import_openapi_to_yaml(
            MINIMAL_OPENAPI,
            &OpenApiImportOptions {
                name: Some("github".to_string()),
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(yaml.contains("issues:"));
        assert!(!yaml.contains("\nrepos:"));
        let reparsed = crate::yaml_parser::parse_yaml_for_client(&yaml).unwrap();
        assert!(reparsed.commands.contains_key("issues"));
        assert!(!reparsed.commands.contains_key("repos"));
    }

    #[test]
    fn test_parse_rejects_invalid_yaml() {
        assert_err_contains(
            parse_openapi_content("openapi: [\n  broken"),
            "Invalid OpenAPI YAML",
        );
    }

    #[test]
    fn test_parse_rejects_invalid_json() {
        assert_err_contains(parse_openapi_content("{not json"), "Invalid OpenAPI JSON");
    }

    #[test]
    fn test_import_rejects_unsupported_openapi_version() {
        const SWAGGER_2: &str = r#"
swagger: "2.0"
info:
  title: Legacy API
  version: "1.0.0"
paths:
  /ping:
    get:
      responses:
        '200':
          description: ok
"#;
        let doc = parse_openapi_content(SWAGGER_2).unwrap();
        assert_err_contains(
            import_openapi(&doc, &OpenApiImportOptions::default()),
            "Unsupported OpenAPI version",
        );
    }

    #[test]
    fn test_import_rejects_missing_openapi_version() {
        const NO_VERSION: &str = r#"
info:
  title: No Version API
  version: "1.0.0"
paths:
  /ping:
    get:
      responses:
        '200':
          description: ok
"#;
        let doc = parse_openapi_content(NO_VERSION).unwrap();
        assert_err_contains(
            import_openapi(&doc, &OpenApiImportOptions::default()),
            "Unsupported OpenAPI version",
        );
    }

    #[test]
    fn test_import_rejects_empty_paths() {
        const EMPTY_PATHS: &str = r#"
openapi: 3.0.3
info:
  title: Empty API
  version: "1.0.0"
servers:
  - url: https://api.example.com
paths: {}
"#;
        let doc = parse_openapi_content(EMPTY_PATHS).unwrap();
        assert_err_contains(
            import_openapi(&doc, &OpenApiImportOptions::default()),
            "No operations found",
        );
    }

    #[test]
    fn test_import_rejects_paths_without_http_methods() {
        const NO_METHODS: &str = r#"
openapi: 3.0.3
info:
  title: Params Only
  version: "1.0.0"
servers:
  - url: https://api.example.com
paths:
  /items:
    parameters:
      - name: q
        in: query
        schema:
          type: string
"#;
        let doc = parse_openapi_content(NO_METHODS).unwrap();
        assert_err_contains(
            import_openapi(&doc, &OpenApiImportOptions::default()),
            "No operations found",
        );
    }

    #[test]
    fn test_import_rejects_unsupported_http_methods_only() {
        const HEAD_ONLY: &str = r#"
openapi: 3.0.3
info:
  title: Head Only
  version: "1.0.0"
servers:
  - url: https://api.example.com
paths:
  /items:
    head:
      responses:
        '200':
          description: ok
"#;
        let doc = parse_openapi_content(HEAD_ONLY).unwrap();
        assert_err_contains(
            import_openapi(&doc, &OpenApiImportOptions::default()),
            "No operations found",
        );
    }

    #[test]
    fn test_import_rejects_unknown_tag_filter() {
        let doc = parse_openapi_content(MINIMAL_OPENAPI).unwrap();
        assert_err_contains(
            import_openapi(
                &doc,
                &OpenApiImportOptions {
                    tag: Some("nonexistent".to_string()),
                    ..Default::default()
                },
            ),
            "No operations found",
        );
    }

    #[test]
    fn test_import_rejects_empty_base_url_override() {
        let doc = parse_openapi_content(MINIMAL_OPENAPI).unwrap();
        assert_err_contains(
            import_openapi(
                &doc,
                &OpenApiImportOptions {
                    base_url: Some(String::new()),
                    ..Default::default()
                },
            ),
            "Base URL cannot be empty",
        );
    }

    #[test]
    fn test_import_rejects_invalid_base_url_override() {
        let doc = parse_openapi_content(MINIMAL_OPENAPI).unwrap();
        assert_err_contains(
            import_openapi(
                &doc,
                &OpenApiImportOptions {
                    base_url: Some("not-a-valid-url".to_string()),
                    ..Default::default()
                },
            ),
            "Base URL must use http:// or https://",
        );
    }

    #[test]
    fn test_import_rejects_invalid_profile_name() {
        let doc = parse_openapi_content(MINIMAL_OPENAPI).unwrap();
        assert_err_contains(
            import_openapi(
                &doc,
                &OpenApiImportOptions {
                    name: Some("bad name".to_string()),
                    ..Default::default()
                },
            ),
            "API name must be alphanumeric or dash",
        );
    }

    #[test]
    fn test_nest_by_parse_rejects_unknown_value() {
        assert_err_contains(
            OpenApiNestBy::parse("hybrid").map(|_| ()),
            "Invalid nest-by value",
        );
    }

    #[test]
    fn test_import_openapi_to_yaml_propagates_parse_errors() {
        assert_err_contains(
            import_openapi_to_yaml("openapi: [broken", &OpenApiImportOptions::default()),
            "Invalid OpenAPI YAML",
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_import_openapi_file_missing_source() {
        assert_err_contains(
            import_openapi_file(
                std::path::Path::new("/nonexistent/ycallr-openapi-test.yaml"),
                &OpenApiImportOptions::default(),
            ),
            "Failed to read",
        );
    }

    #[test]
    fn test_import_tolerates_broken_parameter_ref() {
        const SPEC: &str = r##"
openapi: 3.0.3
info:
  title: Broken Refs API
  version: "1.0.0"
servers:
  - url: https://api.example.com
paths:
  /items:
    get:
      operationId: items/list
      tags: [items]
      parameters:
        - $ref: "#/components/parameters/missing"
        - name: q
          in: query
          schema: { type: string }
      responses:
        '200':
          description: ok
components:
  parameters: {}
"##;
        let doc = parse_openapi_content(SPEC).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();

        let list = api
            .commands
            .get("items")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("list")
            .unwrap();
        assert!(list.params.contains_key("q"));
        assert!(!list.params.contains_key("missing"));
    }

    #[test]
    fn test_import_tolerates_external_parameter_ref() {
        const SPEC: &str = r##"
openapi: 3.0.3
info:
  title: Broken Refs API
  version: "1.0.0"
servers:
  - url: https://api.example.com
paths:
  /items:
    get:
      operationId: items/list
      tags: [items]
      parameters:
        - $ref: "other.yaml#/components/parameters/page"
        - name: q
          in: query
          schema: { type: string }
      responses:
        '200':
          description: ok
"##;
        let doc = parse_openapi_content(SPEC).unwrap();
        let api = import_openapi(&doc, &OpenApiImportOptions::default()).unwrap();
        let get = api
            .commands
            .get("items")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("list")
            .unwrap();
        assert!(get.params.contains_key("q"));
        assert_eq!(get.params.len(), 1);
    }

    #[test]
    fn test_import_tolerates_broken_request_body_ref() {
        const SPEC: &str = r##"
openapi: 3.0.3
info:
  title: Broken Refs API
  version: "1.0.0"
servers:
  - url: https://api.example.com
paths:
  /items:
    post:
      operationId: items/create
      tags: [items]
      requestBody:
        $ref: "#/components/requestBodies/missing"
      responses:
        '201':
          description: created
components:
  requestBodies: {}
"##;
        let doc = parse_openapi_content(SPEC).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();

        let create = api
            .commands
            .get("items")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("create")
            .unwrap();
        assert!(create.body.is_none());
    }

    #[test]
    fn test_import_tolerates_broken_schema_ref_in_body() {
        const SPEC: &str = r##"
openapi: 3.0.3
info:
  title: Broken Refs API
  version: "1.0.0"
servers:
  - url: https://api.example.com
paths:
  /items:
    post:
      operationId: items/create
      tags: [items]
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/Missing"
      responses:
        '201':
          description: created
components:
  schemas: {}
"##;
        let doc = parse_openapi_content(SPEC).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();

        let create = api
            .commands
            .get("items")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("create")
            .unwrap();
        let body = create.body.as_ref().expect("body scaffold expected");
        let json = body.json.as_ref().expect("json body expected");
        assert!(json.as_object().is_some_and(|obj| obj.is_empty()));
        assert!(create.params.is_empty());
    }

    #[test]
    fn test_import_tolerates_broken_path_level_parameter_ref() {
        const SPEC: &str = r##"
openapi: 3.0.3
info:
  title: Broken Refs API
  version: "1.0.0"
servers:
  - url: https://api.example.com
paths:
  /items/{id}:
    parameters:
      - $ref: "#/components/parameters/missing-id"
    get:
      operationId: items/get
      tags: [items]
      responses:
        '200':
          description: ok
components:
  parameters: {}
"##;
        let doc = parse_openapi_content(SPEC).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();

        let get = api
            .commands
            .get("items")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("get")
            .unwrap();
        assert_eq!(get.endpoint.as_deref(), Some("/items/{id}"));
        assert!(get.params.is_empty());
    }

    #[test]
    fn test_import_tolerates_circular_schema_ref() {
        const SPEC: &str = r##"
openapi: 3.0.3
info:
  title: Broken Refs API
  version: "1.0.0"
servers:
  - url: https://api.example.com
paths:
  /items:
    post:
      operationId: items/create
      tags: [items]
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: "#/components/schemas/Node"
      responses:
        '201':
          description: created
components:
  schemas:
    Node:
      type: object
      properties:
        child:
          $ref: "#/components/schemas/Node"
"##;
        let doc = parse_openapi_content(SPEC).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .expect("circular schema refs should not abort import");

        let create = api
            .commands
            .get("items")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("create")
            .unwrap();
        assert!(create.body.is_some());
    }

    const GITHUB_ERROR_SCHEMAS_OPENAPI: &str = r##"
openapi: 3.0.3
info:
  title: GitHub REST API
  version: "1.0.0"
servers:
  - url: https://api.github.com
paths:
  /repos/{owner}/{repo}/issues:
    post:
      operationId: repos/create-issue
      tags: [issues]
      parameters:
        - name: owner
          in: path
          required: true
          schema: { type: string }
        - name: repo
          in: path
          required: true
          schema: { type: string }
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [title]
              properties:
                title: { type: string }
      responses:
        '201':
          description: Created
          content:
            application/json:
              schema:
                type: object
                properties:
                  number: { type: integer }
                  title: { type: string }
        '404':
          $ref: '#/components/responses/not_found'
        '422':
          description: Validation failed
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/validation-error'
    get:
      operationId: repos/list-issues
      tags: [issues]
      responses:
        '200':
          description: OK
          content:
            application/json:
              schema:
                type: array
                items:
                  type: object
                  properties:
                    title: { type: string }
                    number: { type: integer }
        '404':
          $ref: '#/components/responses/not_found'
components:
  responses:
    not_found:
      description: Resource not found
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/basic-error'
  schemas:
    basic-error:
      type: object
      properties:
        message: { type: string }
        documentation_url: { type: string }
    validation-error:
      type: object
      properties:
        message: { type: string }
        errors:
          type: array
          items:
            type: object
            properties:
              field: { type: string }
              message: { type: string }
              code: { type: string }
"##;

    #[test]
    fn test_import_api_errors_from_github_schemas() {
        let doc = parse_openapi_content(GITHUB_ERROR_SCHEMAS_OPENAPI).unwrap();
        let api = import_openapi(&doc, &OpenApiImportOptions::default()).unwrap();

        let errors = api.errors.as_ref().expect("api errors expected");
        assert_eq!(errors.default.as_ref().unwrap().message, "{output.message}");
        assert_eq!(
            errors.codes.get("422").unwrap().message,
            "{output.message} ({output.errors.0.field}: {output.errors.0.message})"
        );
    }

    #[test]
    fn test_import_success_messages_from_response_schemas() {
        let doc = parse_openapi_content(GITHUB_ERROR_SCHEMAS_OPENAPI).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();

        let create = api
            .commands
            .get("issues")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("create")
            .unwrap();
        let responses = create.responses.as_ref().unwrap();
        assert_eq!(
            responses.success.as_ref().unwrap().message,
            "#{output.number}: {output.title}"
        );

        let list = api
            .commands
            .get("issues")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("list")
            .unwrap();
        let list_responses = list.responses.as_ref().unwrap();
        assert_eq!(
            list_responses.success.as_ref().unwrap().message,
            "#{output.0.number}: {output.0.title}"
        );
    }

    #[test]
    fn test_import_operation_error_codes_deduplicate_api_defaults() {
        let doc = parse_openapi_content(GITHUB_ERROR_SCHEMAS_OPENAPI).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();

        let create = api
            .commands
            .get("issues")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("create")
            .unwrap();
        let responses = create.responses.as_ref().unwrap();
        assert!(responses.failure.is_none());
        assert!(!responses.codes.contains_key("404"));
        assert!(!responses.codes.contains_key("422"));
    }

    #[test]
    fn test_import_response_messages_yaml_roundtrip() {
        let yaml = import_openapi_to_yaml(
            GITHUB_ERROR_SCHEMAS_OPENAPI,
            &OpenApiImportOptions {
                name: Some("github".to_string()),
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(yaml.contains("errors:"));
        assert!(yaml.contains("{output.message}"));
        assert!(yaml.contains("{output.errors.0.field}"));

        let reparsed = crate::yaml_parser::parse_yaml_for_client(&yaml).unwrap();
        assert!(reparsed.errors.is_some());
        let create = reparsed
            .commands
            .get("issues")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("create")
            .unwrap();
        assert_eq!(
            create
                .responses
                .as_ref()
                .unwrap()
                .success
                .as_ref()
                .unwrap()
                .message,
            "#{output.number}: {output.title}"
        );
    }

    #[test]
    fn test_github_auto_preset_adds_headers_and_auth_for_ghes() {
        const GHES: &str = r##"
openapi: 3.0.3
info:
  title: GitHub v3 REST API
  version: "1.0.0"
servers:
  - url: "{protocol}://{hostname}/api/v3"
    variables:
      hostname:
        default: HOSTNAME
      protocol:
        default: http
paths:
  /repos/{owner}/{repo}/issues:
    get:
      operationId: repos/list-issues
      tags: [issues]
      responses:
        '200':
          description: ok
components:
  schemas: {}
"##;
        let doc = parse_openapi_content(GHES).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(api.auth.contains_key("github"));
        assert!(api.env.iter().any(|e| e.name == "GITHUB_TOKEN"));

        let list = api
            .commands
            .get("issues")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("list")
            .unwrap();
        assert_eq!(
            list.headers.get("Accept").map(String::as_str),
            Some("application/vnd.github.v3+json")
        );
        assert_eq!(list.auth.as_deref(), Some("github"));
    }

    #[test]
    fn test_github_preset_none_skips_headers() {
        const GHES: &str = r##"
openapi: 3.0.3
info:
  title: GitHub v3 REST API
  version: "1.0.0"
servers:
  - url: http://HOSTNAME/api/v3
paths:
  /repos/{owner}/{repo}/issues:
    get:
      operationId: repos/list-issues
      responses:
        '200':
          description: ok
"##;
        let doc = parse_openapi_content(GHES).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                preset: OpenApiPreset::None,
                ..Default::default()
            },
        )
        .unwrap();

        assert!(api.auth.is_empty());
        assert!(api.env.is_empty());
        let list = api
            .commands
            .get("repos")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("issues")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("list-issues")
            .unwrap();
        assert!(!list.headers.contains_key("Accept"));
    }

    #[test]
    fn test_import_auth_and_env_includes_github_headers() {
        let doc = parse_openapi_content(MINIMAL_OPENAPI).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                name: Some("github".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(api.auth.contains_key("github"));
        assert!(api.env.iter().any(|e| e.name == "GITHUB_TOKEN"));

        let post = api
            .commands
            .get("repos")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("issues")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("create-issue")
            .unwrap();
        assert_eq!(
            post.headers.get("Accept").map(String::as_str),
            Some("application/vnd.github.v3+json")
        );
    }

    #[test]
    fn test_import_parameter_enum_values() {
        const ENUM_PARAM_OPENAPI: &str = r##"
openapi: 3.0.3
info:
  title: GitHub REST API
  version: "1.0.0"
servers:
  - url: https://api.github.com
paths:
  /repos/{owner}/{repo}/issues:
    get:
      operationId: repos/list-issues
      tags: [issues]
      parameters:
        - name: owner
          in: path
          required: true
          schema: { type: string }
        - name: repo
          in: path
          required: true
          schema: { type: string }
        - name: state
          in: query
          schema:
            type: string
            enum: [open, closed, all]
      responses:
        '200':
          description: ok
"##;
        let doc = parse_openapi_content(ENUM_PARAM_OPENAPI).unwrap();
        let api = import_openapi(
            &doc,
            &OpenApiImportOptions {
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();

        let list = api
            .commands
            .get("issues")
            .unwrap()
            .commands
            .as_ref()
            .unwrap()
            .get("list")
            .unwrap();
        let state = list.params.get("state").expect("state param");
        assert_eq!(
            state.enum_values.as_deref(),
            Some(&["open".to_string(), "closed".to_string(), "all".to_string()][..])
        );

        let yaml = import_openapi_to_yaml(
            ENUM_PARAM_OPENAPI,
            &OpenApiImportOptions {
                nest_by: OpenApiNestBy::Tag,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(yaml.contains("enum:"));
        assert!(yaml.contains("- open"));
    }
}
