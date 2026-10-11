//! Schemas and typed values for locally registered agent commands.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use ts_rs::TS;

pub type FieldErrors = BTreeMap<String, Vec<String>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged, deny_unknown_fields)]
pub enum CommandSchema {
    Options { options: Vec<CommandOption> },
    Subcommands { subcommands: Vec<Subcommand> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Subcommand {
    pub name: String,
    pub description: String,
    pub options: Vec<CommandOption>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct Choice<T> {
    pub name: String,
    pub value: T,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum CommandOption {
    String {
        name: String,
        description: String,
        required: bool,
        #[ts(optional)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        choices: Option<Vec<Choice<String>>>,
        #[ts(optional)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min_length: Option<u32>,
        #[ts(optional)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_length: Option<u32>,
    },
    Integer {
        name: String,
        description: String,
        required: bool,
        #[ts(optional)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        choices: Option<Vec<Choice<i64>>>,
        #[ts(optional)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<i64>,
        #[ts(optional)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<i64>,
    },
    Number {
        name: String,
        description: String,
        required: bool,
        #[ts(optional)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        choices: Option<Vec<Choice<f64>>>,
        #[ts(optional)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        #[ts(optional)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
    },
    Boolean {
        name: String,
        description: String,
        required: bool,
    },
    User {
        name: String,
        description: String,
        required: bool,
    },
    Room {
        name: String,
        description: String,
        required: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(untagged)]
pub enum OptionValue {
    String(String),
    Integer(i64),
    Number(f64),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct CommandValues {
    #[serde(default)]
    pub subcommand: Option<String>,
    #[serde(default)]
    pub values: BTreeMap<String, OptionValue>,
}

fn add(errors: &mut FieldErrors, field: impl Into<String>, message: &str) {
    errors.entry(field.into()).or_default().push(message.into());
}
fn named(errors: &mut FieldErrors, path: &str, name: &str, description: &str) {
    if !(1..=32).contains(&name.len())
        || !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-".contains(&b))
    {
        add(
            errors,
            format!("{path}.name"),
            "must be 1–32 lowercase letters, digits, underscores or hyphens",
        );
    }
    if !(1..=100).contains(&description.chars().count()) {
        add(
            errors,
            format!("{path}.description"),
            "must be 1–100 characters",
        );
    }
}
fn bounds<T: PartialOrd>(errors: &mut FieldErrors, path: &str, min: &Option<T>, max: &Option<T>) {
    if let (Some(min), Some(max)) = (min, max)
        && min > max
    {
        add(errors, path, "minimum must not exceed maximum");
    }
}
fn choices<T>(errors: &mut FieldErrors, path: &str, choices: &Option<Vec<Choice<T>>>) {
    if let Some(choices) = choices {
        if choices.len() > 25 {
            add(errors, path, "must have at most 25 choices");
        }
        let mut names = HashSet::new();
        for (i, choice) in choices.iter().enumerate() {
            if !(1..=100).contains(&choice.name.chars().count()) {
                add(
                    errors,
                    format!("{path}[{i}].name"),
                    "must be 1–100 characters",
                );
            }
            if !names.insert(&choice.name) {
                add(errors, format!("{path}[{i}].name"), "must be unique");
            }
        }
    }
}
impl CommandOption {
    pub fn attributes(&self) -> (&str, &str, bool) {
        match self {
            Self::String {
                name,
                description,
                required,
                ..
            } => (name, description, *required),
            Self::Integer {
                name,
                description,
                required,
                ..
            } => (name, description, *required),
            Self::Number {
                name,
                description,
                required,
                ..
            } => (name, description, *required),
            Self::Boolean {
                name,
                description,
                required,
                ..
            } => (name, description, *required),
            Self::User {
                name,
                description,
                required,
                ..
            } => (name, description, *required),
            Self::Room {
                name,
                description,
                required,
                ..
            } => (name, description, *required),
        }
    }
    fn validate(&self, path: &str, errors: &mut FieldErrors) {
        let (name, description, _) = self.attributes();
        named(errors, path, name, description);
        match self {
            Self::String {
                choices: c,
                min_length,
                max_length,
                ..
            } => {
                choices(errors, &format!("{path}.choices"), c);
                bounds(
                    errors,
                    &format!("{path}.min_length"),
                    min_length,
                    max_length,
                );
            }
            Self::Integer {
                choices: c,
                min,
                max,
                ..
            } => {
                choices(errors, &format!("{path}.choices"), c);
                bounds(errors, &format!("{path}.min"), min, max);
            }
            Self::Number {
                choices: c,
                min,
                max,
                ..
            } => {
                choices(errors, &format!("{path}.choices"), c);
                bounds(errors, &format!("{path}.min"), min, max);
                if min.iter().chain(max).any(|n| !n.is_finite())
                    || c.iter().flatten().any(|c| !c.value.is_finite())
                {
                    add(errors, path, "numbers must be finite");
                }
            }
            Self::Boolean { .. } | Self::User { .. } | Self::Room { .. } => {}
        }
    }
    fn validate_value(&self, value: &OptionValue, path: &str, errors: &mut FieldErrors) {
        match (self, value) {
            (
                Self::String {
                    choices,
                    min_length,
                    max_length,
                    ..
                },
                OptionValue::String(value),
            ) => {
                let len = value.chars().count() as u64;
                if min_length.is_some_and(|min| len < u64::from(min))
                    || max_length.is_some_and(|max| len > u64::from(max))
                {
                    add(errors, path, "length is out of range");
                }
                if choices
                    .as_ref()
                    .is_some_and(|c| !c.iter().any(|c| c.value == *value))
                {
                    add(errors, path, "must be one of the choices");
                }
            }
            (
                Self::Integer {
                    choices, min, max, ..
                },
                OptionValue::Integer(value),
            ) => {
                if min.is_some_and(|min| *value < min) || max.is_some_and(|max| *value > max) {
                    add(errors, path, "is out of range");
                }
                if choices
                    .as_ref()
                    .is_some_and(|c| !c.iter().any(|c| c.value == *value))
                {
                    add(errors, path, "must be one of the choices");
                }
            }
            (
                Self::Number {
                    choices, min, max, ..
                },
                OptionValue::Integer(_) | OptionValue::Number(_),
            ) => {
                let value = match value {
                    OptionValue::Integer(n) => *n as f64,
                    OptionValue::Number(n) => *n,
                    _ => unreachable!(),
                };
                if !value.is_finite()
                    || min.is_some_and(|min| value < min)
                    || max.is_some_and(|max| value > max)
                {
                    add(errors, path, "is out of range");
                }
                if choices
                    .as_ref()
                    .is_some_and(|c| !c.iter().any(|c| c.value == value))
                {
                    add(errors, path, "must be one of the choices");
                }
            }
            (Self::Boolean { .. }, OptionValue::Boolean(_)) => {}
            (Self::User { .. } | Self::Room { .. }, OptionValue::Integer(id)) if *id > 0 => {}
            _ => add(errors, path, "has the wrong type"),
        }
    }
}
fn validate_options(options: &[CommandOption], path: &str, errors: &mut FieldErrors) {
    if options.len() > 25 {
        add(errors, path, "must have at most 25 options");
    }
    let mut names = HashSet::new();
    let mut optional = false;
    for (i, option) in options.iter().enumerate() {
        let field = format!("{path}[{i}]");
        let (name, _, required) = option.attributes();
        if !names.insert(name) {
            add(errors, format!("{field}.name"), "must be unique");
        }
        if required && optional {
            add(
                errors,
                format!("{field}.required"),
                "required options must precede optional options",
            );
        }
        optional |= !required;
        option.validate(&field, errors);
    }
}
impl CommandSchema {
    pub fn validate(&self) -> FieldErrors {
        let mut errors = FieldErrors::new();
        match self {
            Self::Options { options } => validate_options(options, "options", &mut errors),
            Self::Subcommands { subcommands } => {
                if subcommands.is_empty() || subcommands.len() > 25 {
                    add(&mut errors, "subcommands", "must have 1–25 subcommands");
                }
                let mut names = HashSet::new();
                for (i, subcommand) in subcommands.iter().enumerate() {
                    let path = format!("subcommands[{i}]");
                    named(
                        &mut errors,
                        &path,
                        &subcommand.name,
                        &subcommand.description,
                    );
                    if !names.insert(&subcommand.name) {
                        add(&mut errors, format!("{path}.name"), "must be unique");
                    }
                    validate_options(&subcommand.options, &format!("{path}.options"), &mut errors);
                }
            }
        }
        errors
    }
    pub fn options_for(&self, subcommand: Option<&str>) -> Result<&[CommandOption], FieldErrors> {
        match (self, subcommand) {
            (Self::Options { options }, None) => Ok(options),
            (Self::Subcommands { subcommands }, Some(name)) => subcommands
                .iter()
                .find(|s| s.name == name)
                .map(|s| s.options.as_slice())
                .ok_or_else(|| {
                    BTreeMap::from([("options.subcommand".into(), vec!["is unknown".into()])])
                }),
            _ => Err(BTreeMap::from([(
                "options.subcommand".into(),
                vec!["is required only for commands with subcommands".into()],
            )])),
        }
    }
    pub fn validate_values(&self, values: &CommandValues) -> FieldErrors {
        let options = match self.options_for(values.subcommand.as_deref()) {
            Ok(options) => options,
            Err(errors) => return errors,
        };
        let mut errors = FieldErrors::new();
        for name in values.values.keys() {
            if !options.iter().any(|o| o.attributes().0 == name) {
                add(&mut errors, format!("options.values.{name}"), "is unknown");
            }
        }
        for option in options {
            let (name, _, required) = option.attributes();
            let path = format!("options.values.{name}");
            match values.values.get(name) {
                Some(value) => option.validate_value(value, &path, &mut errors),
                None if required => add(&mut errors, path, "is required"),
                None => {}
            }
        }
        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn option() -> Value {
        json!({"name":"target","description":"Target","type":"string","required":true})
    }
    fn schema(options: Value) -> CommandSchema {
        serde_json::from_value(json!({"options":options})).unwrap()
    }
    fn invalid_option(key: &str, value: Value, field: &str) {
        let mut o = option();
        o[key] = value;
        assert!(schema(json!([o])).validate().contains_key(field), "{field}");
    }
    #[test]
    fn names_and_descriptions_have_their_own_errors() {
        for name in ["", "UPPER", "has space", "é", &"a".repeat(33)] {
            invalid_option("name", json!(name), "options[0].name");
        }
        for description in ["", &"é".repeat(101)] {
            invalid_option("description", json!(description), "options[0].description");
        }
        let mut o = option();
        o["name"] = json!("0_-valid");
        o["description"] = json!("é".repeat(100));
        assert!(schema(json!([o])).validate().is_empty());
    }
    #[test]
    fn option_counts_names_and_required_order_are_checked() {
        assert!(
            schema(json!(vec![option(); 26]))
                .validate()
                .contains_key("options")
        );
        assert!(
            schema(json!([option(), option()]))
                .validate()
                .contains_key("options[1].name")
        );
        let mut optional = option();
        optional["required"] = json!(false);
        optional["name"] = json!("optional");
        assert!(
            schema(json!([optional, option()]))
                .validate()
                .contains_key("options[1].required")
        );
        assert!(schema(json!([])).validate().is_empty());
    }
    #[test]
    fn subcommands_are_named_unique_bounded_and_one_level_deep() {
        let sub = json!({"name":"run","description":"Run","options":[]});
        for list in [json!([]), json!(vec![sub.clone(); 26])] {
            let s: CommandSchema = serde_json::from_value(json!({"subcommands":list})).unwrap();
            assert!(s.validate().contains_key("subcommands"));
        }
        let s: CommandSchema =
            serde_json::from_value(json!({"subcommands":[sub.clone(),sub]})).unwrap();
        assert!(s.validate().contains_key("subcommands[1].name"));
        let s: CommandSchema = serde_json::from_value(
            json!({"subcommands":[{"name":"Bad","description":"","options":[option(),option()]}]}),
        )
        .unwrap();
        for field in [
            "subcommands[0].name",
            "subcommands[0].description",
            "subcommands[0].options[1].name",
        ] {
            assert!(s.validate().contains_key(field));
        }
        for value in [
            json!({"options":[],"subcommands":[]}),
            json!({"subcommands":[{"name":"run","description":"Run","subcommands":[]}]}),
            json!({}),
        ] {
            assert!(serde_json::from_value::<CommandSchema>(value).is_err());
        }
    }
    #[test]
    fn choice_counts_names_and_value_types_are_checked() {
        invalid_option(
            "choices",
            json!(vec![json!({"name":"One","value":"a"}); 26]),
            "options[0].choices",
        );
        invalid_option(
            "choices",
            json!([{"name":"","value":"a"}]),
            "options[0].choices[0].name",
        );
        invalid_option(
            "choices",
            json!([{"name":"x","value":"a"},{"name":"x","value":"b"}]),
            "options[0].choices[1].name",
        );
        for (kind, value) in [
            ("string", json!(1)),
            ("integer", json!(1.5)),
            ("number", json!("1")),
        ] {
            let mut o = option();
            o["type"] = json!(kind);
            o["choices"] = json!([{"name":"One","value":value}]);
            assert!(serde_json::from_value::<CommandSchema>(json!({"options":[o]})).is_err());
        }
    }
    #[test]
    fn bounds_and_type_specific_fields_are_checked() {
        for kind in ["integer", "number"] {
            let mut o = option();
            o["type"] = json!(kind);
            o["min"] = json!(2);
            o["max"] = json!(1);
            assert!(schema(json!([o])).validate().contains_key("options[0].min"));
        }
        let mut o = option();
        o["min_length"] = json!(2);
        o["max_length"] = json!(1);
        assert!(
            schema(json!([o]))
                .validate()
                .contains_key("options[0].min_length")
        );
        for (kind, field, value) in [
            ("string", "min", json!(1)),
            ("integer", "min_length", json!(1)),
            ("boolean", "choices", json!([])),
            ("room", "max", json!(1)),
            ("user", "choices", json!([])),
            ("unknown", "name", json!("x")),
            ("string", "min_length", json!(-1)),
        ] {
            let mut o = option();
            o["type"] = json!(kind);
            o[field] = value;
            assert!(serde_json::from_value::<CommandSchema>(json!({"options":[o]})).is_err());
        }
    }
    #[test]
    fn malformed_option_fields_and_subcommand_option_limits_are_rejected() {
        for field in ["name", "description", "required", "type"] {
            let mut o = option();
            o.as_object_mut().unwrap().remove(field);
            assert!(
                serde_json::from_value::<CommandSchema>(json!({"options":[o]})).is_err(),
                "{field}"
            );
        }
        let sub: CommandSchema = serde_json::from_value(
            json!({"subcommands":[{"name":"run","description":"Run","options":vec![option();26]}]}),
        )
        .unwrap();
        assert!(sub.validate().contains_key("subcommands[0].options"));
    }
    #[test]
    fn directly_constructed_number_bounds_and_choices_must_be_finite() {
        for (min, max, choices) in [
            (Some(f64::NAN), None, None),
            (None, Some(f64::INFINITY), None),
            (
                None,
                None,
                Some(vec![Choice {
                    name: "Infinite".into(),
                    value: f64::NEG_INFINITY,
                }]),
            ),
        ] {
            let s = CommandSchema::Options {
                options: vec![CommandOption::Number {
                    name: "number".into(),
                    description: "Number".into(),
                    required: false,
                    min,
                    max,
                    choices,
                }],
            };
            assert!(s.validate().contains_key("options[0]"));
        }
    }
    #[test]
    fn values_preserve_types_and_check_required_unknown_choices_and_ranges() {
        let s = schema(json!([
            {"name":"text","description":"Text","required":true,"type":"string","choices":[{"name":"Yes","value":"yes"}],"min_length":2,"max_length":3},
            {"name":"count","description":"Count","required":true,"type":"integer","min":1,"max":3,"choices":[{"name":"Two","value":2}]},
            {"name":"ratio","description":"Ratio","required":true,"type":"number","min":0.5,"max":2,"choices":[{"name":"One","value":1}]},
            {"name":"flag","description":"Flag","required":true,"type":"boolean"},
            {"name":"user","description":"User","required":true,"type":"user"},
            {"name":"room","description":"Room","required":false,"type":"room"}
        ]));
        assert!(s.validate().is_empty());
        let good = json!({"subcommand":null,"values":{"text":"yes","count":2,"ratio":1,"flag":false,"user":12,"room":13}});
        assert!(
            s.validate_values(&serde_json::from_value(good.clone()).unwrap())
                .is_empty()
        );
        for (name, value) in [
            ("text", json!(1)),
            ("text", json!("no")),
            ("text", json!("y")),
            ("text", json!("long")),
            ("count", json!(2.5)),
            ("count", json!(4)),
            ("count", json!(0)),
            ("count", json!(1)),
            ("ratio", json!(0.1)),
            ("ratio", json!(3)),
            ("ratio", json!(1.5)),
            ("flag", json!("false")),
            ("user", json!(0)),
            ("room", json!("13")),
            ("extra", json!(true)),
        ] {
            let mut bad = good.clone();
            bad["values"][name] = value;
            assert!(
                s.validate_values(&serde_json::from_value(bad).unwrap())
                    .contains_key(&format!("options.values.{name}"))
            );
        }
        assert!(
            s.validate_values(&CommandValues::default())
                .contains_key("options.values.text")
        );
        let sub: CommandSchema = serde_json::from_value(
            json!({"subcommands":[{"name":"run","description":"Run","options":[]}]}),
        )
        .unwrap();
        assert!(
            sub.validate_values(&CommandValues::default())
                .contains_key("options.subcommand")
        );
        let values = CommandValues {
            subcommand: Some("run".into()),
            ..Default::default()
        };
        assert!(sub.validate_values(&values).is_empty());
        assert!(
            s.validate_values(&values)
                .contains_key("options.subcommand")
        );
        assert!(
            sub.validate_values(&CommandValues {
                subcommand: Some("other".into()),
                ..Default::default()
            })
            .contains_key("options.subcommand")
        );
    }
}
