use serde_json::Value;

pub fn plugin_schema() -> Value {
    serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Nexus Plugin",
        "type": "object",
        "required": ["id", "name", "version", "description", "author", "image", "manifest_url", "status"],
        "properties": {
            "id": {
                "type": "string",
                "pattern": "^[a-z][a-z0-9]*\\.[a-z][a-z0-9]*(\\.[a-z][a-z0-9-]*)+$",
                "description": "Reverse-domain plugin identifier"
            },
            "name": {
                "type": "string",
                "minLength": 1
            },
            "version": {
                "type": "string",
                "pattern": "^\\d+\\.\\d+\\.\\d+"
            },
            "description": {
                "type": "string",
                "minLength": 1
            },
            "author": {
                "type": "string",
                "minLength": 1
            },
            "license": {
                "type": "string"
            },
            "homepage": {
                "type": "string",
                "format": "uri"
            },
            "icon": {
                "type": "string"
            },
            "image": {
                "type": "string",
                "minLength": 1,
                "description": "Docker image reference"
            },
            "image_digest": {
                "type": "string",
                "pattern": "^sha256:[a-f0-9]{64}$"
            },
            "manifest_url": {
                "type": "string",
                "format": "uri"
            },
            "status": {
                "type": "string",
                "enum": ["active", "deprecated", "unlisted"]
            },
            "categories": {
                "type": "array",
                "items": { "type": "string" }
            }
        },
        "additionalProperties": false
    })
}

pub fn extension_schema() -> Value {
    serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Nexus Extension",
        "type": "object",
        "required": ["id", "name", "version", "description", "author", "manifest_url", "status"],
        "properties": {
            "id": {
                "type": "string",
                "pattern": "^[a-z][a-z0-9]*\\.[a-z][a-z0-9]*(\\.[a-z][a-z0-9-]*)+$",
                "description": "Reverse-domain extension identifier"
            },
            "name": {
                "type": "string",
                "minLength": 1
            },
            "version": {
                "type": "string",
                "pattern": "^\\d+\\.\\d+\\.\\d+"
            },
            "description": {
                "type": "string",
                "minLength": 1
            },
            "author": {
                "type": "string",
                "minLength": 1
            },
            "license": {
                "type": "string"
            },
            "homepage": {
                "type": "string",
                "format": "uri"
            },
            "author_public_key": {
                "type": "string"
            },
            "manifest_url": {
                "type": "string",
                "format": "uri"
            },
            "status": {
                "type": "string",
                "enum": ["active", "deprecated", "unlisted"]
            },
            "platforms": {
                "type": "array",
                "items": { "type": "string" }
            },
            "categories": {
                "type": "array",
                "items": { "type": "string" }
            }
        },
        "additionalProperties": false
    })
}

pub fn registry_schema() -> Value {
    serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Nexus Registry Index",
        "type": "object",
        "required": ["version", "registry", "updated_at", "plugins", "extensions"],
        "properties": {
            "version": {
                "type": "integer",
                "const": 2
            },
            "registry": {
                "type": "object",
                "required": ["name", "description"],
                "properties": {
                    "name": { "type": "string", "minLength": 1 },
                    "description": { "type": "string" },
                    "homepage": { "type": "string", "format": "uri" },
                    "maintainer": { "type": "string" }
                },
                "additionalProperties": false
            },
            "updated_at": {
                "type": "string",
                "format": "date-time"
            },
            "plugins": {
                "type": "array",
                "items": { "$ref": "#/definitions/plugin" }
            },
            "extensions": {
                "type": "array",
                "items": { "$ref": "#/definitions/extension" }
            }
        },
        "additionalProperties": false,
        "definitions": {
            "plugin": plugin_schema(),
            "extension": extension_schema()
        }
    })
}

pub fn plugin_schema_pretty() -> String {
    serde_json::to_string_pretty(&plugin_schema()).unwrap()
}

pub fn extension_schema_pretty() -> String {
    serde_json::to_string_pretty(&extension_schema()).unwrap()
}

pub fn registry_schema_pretty() -> String {
    serde_json::to_string_pretty(&registry_schema()).unwrap()
}
