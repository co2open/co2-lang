extern crate pest;
#[macro_use]
extern crate pest_derive;
pub mod value;

use escapade::Append;
use escapade::Escapable;
use pest::error::Error as PestError;
use pest::iterators::Pair;
use pest::Parser;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use value::Value;

use crate::value::Script;
#[derive(Parser)]
#[grammar = "pipe.pest"]
struct PipeParser;

#[derive(Debug)]
pub struct Pipe {}

#[macro_export]
macro_rules! map {
    () => {
        HashMap::new()
    };
    ($key:expr, $value:expr) => {{
        let mut map = map!();
        map.insert($key, $value);
        map
    }};
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        println!("{:#?}", $($arg)*)
    };
}

#[derive(Debug)]
pub struct Error {
    parse: Option<PestError<Rule>>,
    file: Option<String>,
}

impl Error {
    pub fn parse(error: PestError<Rule>) -> Self {
        Self {
            parse: Some(error),
            file: None,
        }
    }

    pub fn file(local: &str) -> Self {
        Self {
            parse: None,
            file: Some(local.to_string()),
        }
    }
}

// TODO: criar exportação de .pipe com pre_parse realizado.
impl Pipe {
    pub fn from_path(path: &str) -> Result<Value, Error> {
        let unparsed_file = match fs::read_to_string(path) {
            Ok(file) => file,
            Err(err) => return Err(Error::file(&err.to_string())),
        };

        Self::from_str(&unparsed_file)
    }

    pub fn from_str(unparsed_file: &str) -> Result<Value, Error> {
        let pre_parse = Self::pre_parse(unparsed_file.to_string());
        Self::from_pre_parsed_str(&pre_parse)
    }

    pub fn from_pre_parsed_str(pre_parse_file: &str) -> Result<Value, Error> {
        match PipeParser::parse(Rule::pipe, pre_parse_file) {
            Ok(mut pairs) => match pairs.next() {
                Some(pair) => Ok(Self::parse(pair)),
                None => Ok(Value::Undefined),
            },
            Err(error) => Err(Error::parse(error)),
        }
    }

    pub fn pre_parse(content: String) -> String {
        Self::parse_embedded(content)
        //TODO:parse_module_run
    }

    // pub fn parse_tags(content: String) -> String {
    //     let re = Regex::new(r#":[a-z_]*\([a-z_0-9.!?]*\)"#).unwrap();

    //     let mut result = String::default();
    //     let mut content_last_end = 0;
    //     let mut tags = HashMap::new();

    //     for cap in re.captures_iter(&content) {
    //         let range = cap.get(0).unwrap().range();
    //         let target = content[range.start..range.end].to_string();
    //     }

    //     result.push_str(&content[content_last_end..]);

    //     result
    // }

    pub fn parse_embedded(content: String) -> String {
        let re = Regex::new(r#"(?s)```.*?```"#).unwrap();

        let mut result = String::default();
        let mut content_last_end = 0;

        for cap in re.captures_iter(&content) {
            let range = cap.get(0).unwrap().range();
            let content_start = range.start + 3;
            let content_end = range.end - 3;
            let target = content[content_start..content_end].to_string();

            let (runtime, target_start) = {
                let mut first_line = Vec::new();
                let mut end = 0;

                for char in target.chars() {
                    if char == '\n' {
                        break;
                    }

                    end += 1;
                    first_line.push(char as u8);
                }

                if first_line.len() == 0 {
                    ("default".to_string(), end)
                } else {
                    (String::from_utf8(first_line).unwrap(), end)
                }
            };

            let target_fix = target[target_start..].to_string();

            let body = format!(
                r#"{{ "___PIPE___type": "embedded", "___PIPE___content": "{}", "___PIPE___lang": "{}"}}"#,
                target_fix.escape().into_inner(),
                runtime
            );

            result.push_str(&content[content_last_end..range.start]);
            result.push_str(&body);
            content_last_end = range.end;
        }

        result.push_str(&content[content_last_end..]);

        result
    }

    fn parse(pair: Pair<Rule>) -> Value {
        match pair.as_rule() {
            Rule::sessions => {
                let mut item: HashMap<String, Value> = map!();

                for pair in pair.into_inner() {
                    match pair.as_rule() {
                        Rule::session_generic_config => {
                            let mut inner = pair.into_inner();
                            let name = inner.next().unwrap().as_str().to_string();
                            let value = Self::parse(inner.next().unwrap());

                            match item.get(&name) {
                                Some(cur_value) => {
                                    let value =
                                        cur_value.merge_object(value.to_object().unwrap()).unwrap();
                                    item.insert(name, value);
                                }
                                None => {
                                    item.insert(name, value);
                                }
                            };
                        }
                        Rule::session_pipeline => {
                            let mut inner = pair.into_inner();
                            let name = inner.next().unwrap().as_str().to_string();
                            let value = Self::parse(inner.next().unwrap());

                            match item.get(&name) {
                                Some(cur_value) => {
                                    let mut new_value = cur_value.to_array().unwrap();
                                    new_value.extend(value.to_array().unwrap());
                                    item.insert(name, Value::Array(new_value));
                                }
                                None => {
                                    item.insert(name, value);
                                }
                            };
                        }
                        _ => {}
                    }
                }

                Value::Object(item)
            }
            Rule::session_generic_config_content => match pair.into_inner().next() {
                Some(pair) => Self::parse(pair),
                None => Value::Undefined,
            },
            Rule::session_pipeline_content => {
                let mut list = Vec::new();

                for pair in pair.into_inner() {
                    list.push(Self::parse(pair));
                }

                Value::Array(list)
            }
            Rule::module => {
                let mut inner = pair.into_inner();

                let module_name = Value::String(inner.next().unwrap().as_str().to_string());
                let attr2 = Self::parse(inner.next().unwrap());

                let (reference, params) = if attr2.is_object() {
                    (Value::Null, attr2.to_object().unwrap())
                } else {
                    (
                        attr2,
                        Self::parse(inner.next().unwrap()).to_object().unwrap(),
                    )
                };

                let mut map = map!("module".to_string(), module_name);
                map.insert("ref".to_string(), reference);
                map.extend(params);

                let mut tags = map!();
                while let Some(pair) = inner.next() {
                    let tag = Self::parse(pair).to_array().unwrap();
                    let key = tag.get(0).unwrap().to_string().unwrap();
                    let value = tag.get(1).unwrap();

                    tags.insert(key, value.clone());
                }

                if tags.len() > 0 {
                    map.insert("___tags".to_string(), Value::Object(tags));
                }

                Value::Object(map)
            }
            Rule::module_content => {
                let mut map = map!();

                for pair in pair.into_inner() {
                    let rule = pair.as_rule();
                    let value = Self::parse(pair);

                    match rule {
                        Rule::attach => {
                            let value = map!("attach".to_string(), value);

                            map.extend(value);
                        }
                        _ => {
                            let value = match map.get("params") {
                                Some(cur_value) => {
                                    cur_value.merge_object(value.to_object().unwrap()).unwrap();

                                    map!("params".to_string(), cur_value.clone())
                                }
                                None => {
                                    map!("params".to_string(), value)
                                }
                            };

                            map.extend(value);
                        }
                    };
                }

                Value::Object(map)
            }
            Rule::attach => {
                let mut inner = pair.into_inner();
                let value = Self::parse(inner.next().unwrap());
                value
            }
            Rule::param_macro_content => {
                let mut inner = pair.into_inner();

                match inner.next() {
                    Some(pair) => Self::parse(pair),
                    None => Value::Object(map!()),
                }
            }
            Rule::common_content => {
                let mut map = map!();

                for pair in pair.into_inner() {
                    match pair.as_rule() {
                        Rule::attach => {
                            let value = Self::parse(pair);
                            map.insert("attach".to_string(), value);
                        }
                        Rule::param => {
                            let value = Self::make_param(pair);
                            map.insert(value.0, value.1);
                        }
                        Rule::param_macro => {
                            let (name, value) = Self::make_param_macro(pair);

                            match map.get(&name) {
                                Some(cur_value) => {
                                    let value = cur_value.array_push(value).unwrap();
                                    map.insert(name, value);
                                }
                                None => {
                                    map.insert(name, Value::Array(vec![value]));
                                }
                            }
                        }
                        _ => {
                            map.insert("".to_string(), Value::Undefined);
                        }
                    };
                }

                Value::Object(map)
            }
            Rule::object => {
                let mut map = map!();

                for pair in pair.into_inner() {
                    let mut inner = pair.into_inner();
                    let key = inner
                        .next()
                        .unwrap()
                        .into_inner()
                        .next()
                        .unwrap()
                        .as_str()
                        .to_string();
                    let value = Self::parse(inner.next().unwrap());

                    map.insert(key, value);
                }

                Value::Object(map)
            }
            Rule::array => {
                let mut list = Vec::new();

                for pair in pair.into_inner() {
                    list.push(Self::parse(pair));
                }

                Value::Array(list)
            }
            Rule::number => Value::Number(pair.as_str().to_string()),
            Rule::boolean => {
                let value = if pair.as_str().eq("true") {
                    true
                } else {
                    false
                };

                Value::Boolean(value)
            }
            Rule::string => {
                let mut inner = pair.clone().into_inner();
                Self::parse(inner.next().unwrap())
            }
            Rule::string_alt => {
                let mut inner = pair.clone().into_inner();
                Self::parse(inner.next().unwrap())
            }
            Rule::string_content => Value::String(pair.as_str().to_string()),
            Rule::string_content_alt => Value::String(pair.as_str().to_string()),
            Rule::ident => Value::String(pair.as_str().to_string()),
            Rule::string_interpolation => {
                let mut inner = pair.clone().into_inner();
                Self::parse(inner.next().unwrap())
            }
            Rule::string_interpolation_content => {
                let raw = pair.as_str().to_string();
                Value::Interpolation(Script::from_string(raw))
            }
            Rule::interpolation => {
                let inner = pair.into_inner();
                let value = inner.as_str().trim().to_string();

                Value::Interpolation(Script::from_interpolation(value))
            }
            Rule::object_interpolation => {
                let raw = pair.as_str().to_string();
                Value::Interpolation(Script::from_string(raw))
            }
            Rule::reference => Value::String(pair.as_str().to_string()),
            Rule::tag => {
                let mut inner = pair.into_inner();

                let key = Self::parse(inner.next().unwrap()).to_string().unwrap();
                let mut value = Vec::new();

                if let Some(args) = inner.next() {
                    value.extend(Self::parse(args).to_array().unwrap());
                }

                let list = vec![Value::String(key), Value::Array(value)];

                Value::Array(list)
            }
            Rule::tag_args => {
                let mut list = Vec::new();
                let mut inner = pair.into_inner();

                while let Some(pair) = inner.next() {
                    list.push(Self::parse(pair))
                }

                Value::Array(list)
            }
            Rule::null => Value::Null,
            _ => Value::Undefined,
        }
    }

    fn make_param(pair: Pair<Rule>) -> (String, Value) {
        let mut inner = pair.into_inner();
        let key = inner.next().unwrap().as_str().to_string();
        let value = Self::parse(inner.next().unwrap());

        (key, value)
    }

    fn make_param_macro(pair: Pair<Rule>) -> (String, Value) {
        let mut inner = pair.into_inner();
        let mut map = map!();
        let total = inner.clone().count();
        let key = inner.next().unwrap().as_str().to_string();

        match total {
            2 => {
                let params = Self::parse(inner.next().unwrap());
                (key, params)
            }
            3 => {
                let value = Self::parse(inner.next().unwrap());
                map.insert(key.clone(), value);
                let params = Self::parse(inner.next().unwrap());

                (key, params.merge_object(map).unwrap())
            }
            total => {
                map.insert(key.clone(), {
                    let mut list = Vec::new();
                    let limit = total - 2;

                    loop {
                        if list.len() < limit {
                            list.push(Self::parse(inner.next().unwrap()));
                        } else {
                            break Value::Array(list);
                        }
                    }
                });
                let params = Self::parse(inner.next().unwrap());
                (key, params.merge_object(map).unwrap())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let pipe = r#"
            pipeline {
                macro "http" (
                    test=true
                )
            }
        "#;
        let value = Pipe::from_str(pipe);
        assert!(value.is_ok());
    }

    #[test]
    fn parse_json() {
        let pipe = r#"
            pipeline {
                macro "http" (
                    test=true
                )
            }
        "#;
        let value = Pipe::from_str(pipe).unwrap();
        assert!(value.as_json().contains("pipeline"));
    }

    #[test]
    fn complex_macro() {
        let pipe = r#"
        import {
            module 1 [1.5] {"item": true} false "name" (
                item=false
            )
        }
        "#;
        let value = Pipe::from_str(pipe).unwrap();
        let module_base = value
            .to_object()
            .unwrap()
            .get("import")
            .unwrap()
            .to_object()
            .unwrap()
            .get("module")
            .unwrap()
            .to_array()
            .unwrap()
            .get(0)
            .unwrap()
            .to_object()
            .unwrap();

        let module = module_base.get("module").unwrap().to_array().unwrap();
        let item = module_base.get("item").unwrap().to_boolean().unwrap();

        assert_eq!(module.get(0).unwrap(), &Value::Number("1".to_string()));
        assert_eq!(
            module.get(1).unwrap(),
            &Value::Array(vec![Value::Number("1.5".to_string())])
        );
        assert_eq!(
            module.get(2).unwrap(),
            &Value::Object({
                let mut map = HashMap::new();
                map.insert("item".to_string(), Value::Boolean(true));
                map
            })
        );
        assert_eq!(module.get(3).unwrap(), &Value::Boolean(false));
        assert_eq!(module.get(4).unwrap(), &Value::String("name".to_string()));
        assert_eq!(item, false);
    }

    const PIPE_CONTENT: &str = r#"
            interpolation {
                value=${item}
                string=`item is ${item}!`
                object={ "value": ${item} }
            }
        "#;

    #[test]
    fn interpolation_string() {
        let value = match Pipe::from_str(PIPE_CONTENT) {
            Ok(value) => value,
            Err(err) => panic!("{:?}", err),
        };

        let obj = value
            .to_object()
            .unwrap()
            .get("interpolation")
            .unwrap()
            .to_object()
            .unwrap();

        assert_eq!(
            &obj.get("string").unwrap().to_string().unwrap(),
            r#""item is \"+(item)+\"!""#
        );
    }

    #[test]
    fn interpolation() {
        let value = match Pipe::from_str(PIPE_CONTENT) {
            Ok(value) => value,
            Err(err) => panic!("{:?}", err),
        };

        let obj = value
            .to_object()
            .unwrap()
            .get("interpolation")
            .unwrap()
            .to_object()
            .unwrap();

        let object = obj.get("object").unwrap().to_string().unwrap();

        assert_eq!(object, r#"{ \"value\": \"+(item)+\" }"#);
    }
}
