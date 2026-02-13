use jsonschema::Validator;
use serde_json::Value;

/// Validate a JSON value against a JSON Schema.
/// Returns Ok(()) if valid, Err with a human-readable error message if invalid.
pub fn validate_input(schema: &Value, input: &Value) -> Result<(), String> {
    let validator = Validator::new(schema)
        .map_err(|e| format!("Invalid schema: {}", e))?;

    validator
        .validate(input)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_valid_input() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        });
        let input = json!({ "name": "test" });
        assert!(validate_input(&schema, &input).is_ok());
    }

    #[test]
    fn test_missing_required_field() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"]
        });
        let input = json!({});
        assert!(validate_input(&schema, &input).is_err());
    }

    #[test]
    fn test_wrong_type() {
        let schema = json!({
            "type": "object",
            "properties": {
                "count": { "type": "integer" }
            }
        });
        let input = json!({ "count": "not a number" });
        assert!(validate_input(&schema, &input).is_err());
    }

    #[test]
    fn test_empty_object_valid() {
        let schema = json!({
            "type": "object",
            "properties": {}
        });
        let input = json!({});
        assert!(validate_input(&schema, &input).is_ok());
    }
}
