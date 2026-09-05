//! Tool schemas, derived from the protobuf descriptor (ADR 0006 §6).
//!
//! MCP describes a tool with a JSON Schema. Writing those by hand would be a second
//! description of the model beside `song.proto`, and it fails the way second descriptions
//! always do: a field is added, the schema is not, and the only symptom is that a model never
//! learns the field exists. Nothing breaks, nothing is logged, the tool is just quietly less
//! capable than it looks.
//!
//! So the schemas come out of `escribass_proto::DESCRIPTOR`, which `proto/codegen.sh` builds
//! from the same `.proto` as the Rust types. A field cannot exist in one and not the other.
//!
//! The mapping is proto3 JSON's own (protobuf's canonical JSON), with property names in
//! **proto** form rather than `jsonName`, because that is the form ADR 0002 §4 fixes for every
//! document in this system.

use escribass_proto::prost_types::field_descriptor_proto::Type;
use escribass_proto::prost_types::{
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorProto,
    FileDescriptorSet, SourceCodeInfo,
};
use prost::Message;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

/// One tool, as MCP needs to advertise it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSchema {
    /// The RPC name in the snake_case form §5 uses: `AddTrack` becomes `add_track`.
    pub name: String,
    /// The RPC's leading comment in `song_tools.proto`. The proto is the documentation.
    pub description: String,
    /// The request message as a JSON Schema object.
    pub input_schema: Map<String, Value>,
}

/// How deep a nested message is expanded before it is described as a bare object.
///
/// `ponytail:` the model has no recursive messages, so this never fires today. It is here so
/// that adding one is a shallower schema rather than a stack overflow at start-up.
const MAX_DEPTH: usize = 8;

/// The service whose RPCs are tools: §5's `SongTools`, and only it.
///
/// The descriptor also carries `Render` (`proto/render.proto`), the engine's boundary. It is
/// not a tool — a model never calls the engine; `render_export` on `SongTools` is how a
/// render is asked for — and advertising it would hand a model a `RenderPlan` to fill in,
/// which is exactly the thing ADR 0007 §1 keeps out of every process but core.
const TOOL_SERVICE: &str = "SongTools";

/// Every tool in the service, in declaration order.
pub fn tool_schemas(descriptor: &[u8]) -> Result<Vec<ToolSchema>, String> {
    let set = FileDescriptorSet::decode(descriptor)
        .map_err(|e| format!("the descriptor set does not decode: {e}"))?;
    let index = Index::build(&set);

    let mut tools = Vec::new();
    for file in &set.file {
        for (position, service) in file.service.iter().enumerate() {
            if service.name() != TOOL_SERVICE {
                continue;
            }
            for (method_position, method) in service.method.iter().enumerate() {
                let request = index
                    .message(method.input_type())
                    .ok_or_else(|| format!("unknown request type {}", method.input_type()))?;
                // The rpc's own comment if it has one, otherwise the request message's.
                // §5's service block lists twenty RPCs on twenty lines; what each tool means
                // is documented where its arguments are.
                let at_rpc = comment(file, &[6, position as i32, 2, method_position as i32]);
                let description = if at_rpc.is_empty() {
                    comment(request.0, &request.1)
                } else {
                    at_rpc
                };
                tools.push(ToolSchema {
                    name: snake_case(method.name()),
                    description,
                    input_schema: object_schema(&index, request, 0),
                });
            }
        }
    }
    Ok(tools)
}

/// Every message and enum in the set, by fully qualified name (`.escribass.song.v1.Note`).
struct Index<'a> {
    messages: BTreeMap<String, (&'a FileDescriptorProto, Vec<i32>, &'a DescriptorProto)>,
    enums: BTreeMap<String, &'a EnumDescriptorProto>,
}

impl<'a> Index<'a> {
    fn build(set: &'a FileDescriptorSet) -> Self {
        let mut index = Index { messages: BTreeMap::new(), enums: BTreeMap::new() };
        for file in &set.file {
            let prefix = format!(".{}", file.package());
            for (position, message) in file.message_type.iter().enumerate() {
                index.add_message(file, &prefix, vec![4, position as i32], message);
            }
            for enumeration in &file.enum_type {
                index.enums.insert(format!("{prefix}.{}", enumeration.name()), enumeration);
            }
        }
        index
    }

    fn add_message(
        &mut self,
        file: &'a FileDescriptorProto,
        prefix: &str,
        path: Vec<i32>,
        message: &'a DescriptorProto,
    ) {
        let name = format!("{prefix}.{}", message.name());
        for (position, nested) in message.nested_type.iter().enumerate() {
            let mut deeper = path.clone();
            deeper.extend([3, position as i32]);
            self.add_message(file, &name, deeper, nested);
        }
        for enumeration in &message.enum_type {
            self.enums.insert(format!("{name}.{}", enumeration.name()), enumeration);
        }
        self.messages.insert(name, (file, path, message));
    }

    fn message(&self, name: &str) -> Option<&(&'a FileDescriptorProto, Vec<i32>, &'a DescriptorProto)> {
        self.messages.get(name)
    }
}

type Entry<'a> = (&'a FileDescriptorProto, Vec<i32>, &'a DescriptorProto);

/// The fields core sets on every entity, whatever a caller sends (§4.3, ADR 0006 §4).
///
/// Left in the schema, a model fills them in on every call — inventing ids, writing its own
/// provenance, choosing versions — and every one of those is discarded. Advertising an
/// argument that is ignored is worse than omitting it: it spends the model's attention and
/// teaches it a contract that is not real.
const OVERWRITTEN: [&str; 3] = ["id", "provenance", "version"];

/// Whether this message is an entity, so its §4.3 fields are core's rather than a caller's.
fn is_entity(message: &DescriptorProto) -> bool {
    OVERWRITTEN.iter().all(|owned| message.field.iter().any(|f| f.name() == *owned))
}

fn object_schema(index: &Index, entry: &Entry, depth: usize) -> Map<String, Value> {
    let (file, path, message) = entry;
    let mut properties = Map::new();
    let entity = is_entity(message);

    for (position, field) in message.field.iter().enumerate() {
        if entity && OVERWRITTEN.contains(&field.name()) {
            continue;
        }
        let mut at = path.clone();
        at.extend([2, position as i32]);
        let mut schema = field_schema(index, field, depth);
        let mut description = comment(file, &at);
        // A real oneof means the caller sets exactly one of its members. Proto3 JSON has no
        // marker for that, so it is said in the description, where a model will read it.
        if let Some(group) = real_oneof(message, field) {
            let siblings: Vec<&str> = message
                .field
                .iter()
                .filter(|f| f.oneof_index == field.oneof_index)
                .map(FieldDescriptorProto::name)
                .filter(|name| *name != field.name())
                .collect();
            let note = format!("Set exactly one of `{}`, `{}`.", field.name(), siblings.join("`, `"));
            description = if description.is_empty() {
                format!("({group}) {note}")
            } else {
                format!("{description} ({group}) {note}")
            };
        }
        if !description.is_empty() {
            schema.insert("description".to_string(), Value::String(description));
        }
        properties.insert(field.name().to_string(), Value::Object(schema));
    }

    // No `required`. Proto3 has no required fields, and deciding per tool which ones "really"
    // matter would be hand-maintained knowledge beside the proto — the drift this module
    // exists to avoid. A missing argument comes back as a structured error §6's retry loop can
    // act on. `ponytail:` if models turn out to need the hint, add a proto option and derive
    // `required` from that instead of from a list here.
    Map::from_iter([
        ("type".to_string(), json!("object")),
        ("properties".to_string(), Value::Object(properties)),
        ("additionalProperties".to_string(), json!(false)),
    ])
}

/// The name of the `oneof` this field belongs to, if it is a real one.
///
/// A `proto3 optional` field is compiled into a *synthetic* one-member oneof named after the
/// field. Treating that as a real oneof would tell a model that `index` is a choice between
/// alternatives when it is simply an optional field.
fn real_oneof<'a>(message: &'a DescriptorProto, field: &FieldDescriptorProto) -> Option<&'a str> {
    if field.proto3_optional() {
        return None;
    }
    let position = field.oneof_index? as usize;
    message.oneof_decl.get(position).map(|d| d.name())
}

fn field_schema(index: &Index, field: &FieldDescriptorProto, depth: usize) -> Map<String, Value> {
    // A protobuf map is a repeated field of a synthetic two-field entry message. In JSON it is
    // an object, so it must be recognised before `repeated` is.
    if let Some(entry) = map_entry(index, field) {
        let value = entry.2.field.iter().find(|f| f.name() == "value").expect("a map entry has a value");
        return Map::from_iter([
            ("type".to_string(), json!("object")),
            ("additionalProperties".to_string(), Value::Object(base_schema(index, value, depth + 1))),
        ]);
    }

    let base = base_schema(index, field, depth);
    if field.label() == escribass_proto::prost_types::field_descriptor_proto::Label::Repeated {
        return Map::from_iter([
            ("type".to_string(), json!("array")),
            ("items".to_string(), Value::Object(base)),
        ]);
    }
    base
}

fn map_entry<'a>(index: &'a Index, field: &FieldDescriptorProto) -> Option<&'a Entry<'a>> {
    if field.r#type() != Type::Message {
        return None;
    }
    let entry = index.message(field.type_name())?;
    entry.2.options.as_ref()?.map_entry.unwrap_or(false).then_some(entry)
}

fn base_schema(index: &Index, field: &FieldDescriptorProto, depth: usize) -> Map<String, Value> {
    let object = |pairs: [(&str, Value); 1]| -> Map<String, Value> {
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    };

    match field.r#type() {
        Type::Double | Type::Float => object([("type", json!("number"))]),
        Type::Int32 | Type::Sint32 | Type::Sfixed32 | Type::Uint32 | Type::Fixed32 => {
            object([("type", json!("integer"))])
        }
        // Proto3 JSON encodes 64-bit integers as strings, because a double cannot hold one
        // exactly (ADR 0002 §1 is why the model has almost none). Both forms are accepted on
        // input, so both are advertised.
        Type::Int64 | Type::Sint64 | Type::Sfixed64 | Type::Uint64 | Type::Fixed64 => {
            Map::from_iter([
                ("type".to_string(), json!(["string", "integer"])),
                ("description".to_string(), json!("64-bit integer; proto3 JSON writes it as a string")),
            ])
        }
        Type::Bool => object([("type", json!("boolean"))]),
        Type::String => object([("type", json!("string"))]),
        Type::Bytes => Map::from_iter([
            ("type".to_string(), json!("string")),
            ("contentEncoding".to_string(), json!("base64")),
        ]),
        Type::Enum => match index.enums.get(field.type_name()) {
            Some(enumeration) => Map::from_iter([
                ("type".to_string(), json!("string")),
                (
                    "enum".to_string(),
                    Value::Array(
                        enumeration.value.iter().map(|v| json!(v.name())).collect(),
                    ),
                ),
            ]),
            None => object([("type", json!("string"))]),
        },
        Type::Message | Type::Group => {
            // Well-known types have their own JSON forms; a structural expansion of
            // Timestamp's seconds and nanos would be wrong as well as useless.
            if field.type_name() == ".google.protobuf.Timestamp" {
                return Map::from_iter([
                    ("type".to_string(), json!("string")),
                    ("format".to_string(), json!("date-time")),
                ]);
            }
            match index.message(field.type_name()) {
                Some(entry) if depth < MAX_DEPTH => object_schema(index, entry, depth + 1),
                _ => object([("type", json!("object"))]),
            }
        }
    }
}

/// The leading comment attached to a descriptor path, cleaned of its `//` and wrapping.
fn comment(file: &FileDescriptorProto, path: &[i32]) -> String {
    let Some(SourceCodeInfo { location, .. }) = file.source_code_info.as_ref() else {
        return String::new();
    };
    let Some(found) = location.iter().find(|l| l.path == path) else {
        return String::new();
    };
    let text = found.leading_comments();
    // Comments arrive as raw lines with a leading space each. Paragraphs are worth keeping;
    // the hard wrapping inside them is not, since the reader is a model or a tool picker.
    text.split("\n\n")
        .map(|paragraph| {
            paragraph
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|paragraph| !paragraph.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// `AddTrack` becomes `add_track` — §5 names every tool in snake_case.
fn snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (position, character) in name.char_indices() {
        if character.is_ascii_uppercase() && position > 0 {
            out.push('_');
        }
        out.push(character.to_ascii_lowercase());
    }
    out
}

/// The service's RPC names, snake_cased, for callers that only need the list.
pub fn tool_names(descriptor: &[u8]) -> Result<Vec<String>, String> {
    Ok(tool_schemas(descriptor)?.into_iter().map(|t| t.name).collect())
}

/// One field of a message, as the field-coverage guard sees it (ADR 0007 §5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    /// The fully qualified name of the message this field holds — for a map, of its value —
    /// when it holds one.
    pub message: Option<String>,
}

/// Every message in the set by fully qualified name (`.escribass.song.v1.Note`), with its
/// fields.
///
/// The synthetic entry message a `map<K, V>` compiles to is folded into the field that
/// declares the map: it exists on the wire and nowhere in the model, so a guard that walked
/// it would be covering `key` and `value` fields nobody wrote. This is the same index the
/// tool schemas are built from, so there is one reading of the descriptor, not two.
pub fn message_fields(descriptor: &[u8]) -> Result<BTreeMap<String, Vec<Field>>, String> {
    let set = FileDescriptorSet::decode(descriptor)
        .map_err(|e| format!("the descriptor set does not decode: {e}"))?;
    let index = Index::build(&set);

    let is_entry = |message: &DescriptorProto| {
        message.options.as_ref().and_then(|o| o.map_entry).unwrap_or(false)
    };
    let mut messages = BTreeMap::new();
    for (name, entry) in &index.messages {
        if is_entry(entry.2) {
            continue;
        }
        let fields = entry
            .2
            .field
            .iter()
            .map(|field| {
                let message = match map_entry(&index, field) {
                    Some(map) => map
                        .2
                        .field
                        .iter()
                        .find(|f| f.name() == "value")
                        .filter(|f| f.r#type() == Type::Message)
                        .map(|f| f.type_name().to_string()),
                    None if field.r#type() == Type::Message => Some(field.type_name().to_string()),
                    None => None,
                };
                Field { name: field.name().to_string(), message }
            })
            .collect();
        messages.insert(name.clone(), fields);
    }
    Ok(messages)
}
