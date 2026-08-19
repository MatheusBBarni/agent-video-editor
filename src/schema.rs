use serde_json::{Value, json};

const OPS: &[&str] = &[
    "trim",
    "cut-out",
    "keep",
    "concat",
    "resize",
    "frame",
    "captions",
    "text",
    "fade",
    "volume",
    "rotate",
    "crop",
    "speed",
    "extract-audio",
    "replace-audio",
    "overlay",
    "compress",
    "convert",
];

pub fn plan_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "ave run plan",
        "type": "object",
        "required": ["steps"],
        "properties": {
            "steps": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["op"],
                    "properties": {
                        "op": { "enum": OPS }
                    }
                }
            }
        }
    })
}

pub fn print() {
    println!("{}", serde_json::to_string(&plan_schema()).expect("json"));
}

pub fn allowed_ops() -> &'static [&'static str] {
    OPS
}
