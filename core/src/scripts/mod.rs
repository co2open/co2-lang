// pub extern crate rhai;
// use std::convert::TryFrom;
// #[macro_use]
// pub mod macros;
// use regex::Regex;
// use rhai::{serde::to_dynamic, Engine, EvalAltResult, Scope, AST};
// use serde_json::{Error as SerdeJsonError, Map, Value};

// use crate::debug;

// #[derive(Debug, Default)]
// pub struct Error {
//     serde_json: Option<SerdeJsonError>,
//     rhai: Option<Box<EvalAltResult>>,
// }

// impl From<SerdeJsonError> for Error {
//     fn from(error: SerdeJsonError) -> Self {
//         Self {
//             serde_json: Some(error),
//             ..Default::default()
//         }
//     }
// }

// impl From<Box<EvalAltResult>> for Error {
//     fn from(error: Box<EvalAltResult>) -> Self {
//         Self {
//             rhai: Some(error),
//             ..Default::default()
//         }
//     }
// }

// struct ScriptInner {
//     value: Value,
//     scripts: Vec<String>,
// }

// impl ScriptInner {
//     fn new(engine: &Engine, value: &Value) -> Self {
//         let mut scripts = Vec::new();

//         if let Some(obj) = value.as_object() {
//             if let Some(obj_type_value) = obj.get("__type") {
//                 if obj_type_value.as_str().unwrap().eq("script") {
//                     let script = vec![obj.get("script").unwrap().as_str().unwrap().to_string()];
//                     return Self {
//                         value: value.clone(),
//                         scripts,
//                     };
//                 }
//             }

//             let map = obj
//                 .into_iter()
//                 .map(|(key, value)| {
//                     let inner = Self::new(engine, value);
//                     scripts.push(inner);
//                     (key.clone(), value.clone())
//                 })
//                 .collect::<Map<String, Value>>();

//             let value = Value::from(map);
//             Self { scripts }
//         } else if let Some(array) = value.as_array() {
//             let list = array
//                 .iter()
//                 .map(|item| {
//                     let replaced = Self::new(engine, item);
//                     scripts.extend(replaced.scripts);
//                     replaced.value
//                 })
//                 .collect::<Vec<_>>();

//             let value = Value::from(list);
//             Self { value, scripts }
//         } else {
//             Self {
//                 value: value.clone(),
//                 scripts,
//             }
//         }
//     }
// }

// #[derive(Clone, Debug)]
// pub struct Interpolation {
//     ast: AST,
//     target: String,
// }

// #[derive(Debug)]
// pub struct Script {
//     script: String,
//     engine: Engine,
// }

// impl Script {
//     pub fn resolve(&self, payload: Value) -> Result<String, Error> {
//         match to_dynamic(payload) {
//             Ok(dynamic) => {
//                 let mut replaced = self.replaced.clone();

//                 for inter in self.scripts.iter() {
//                     let mut scope = Scope::new();
//                     scope.push("payload", dynamic.clone());

//                     match self
//                         .engine
//                         .eval_ast_with_scope::<String>(&mut scope, &inter.ast)
//                     {
//                         Ok(output) => {
//                             if self.re_no_script.is_match(&output) {
//                                 replaced = replaced.replace(&inter.target, &output);
//                             } else {
//                                 replaced =
//                                     replaced.replace(&inter.target, &format!(r#""{}""#, output));
//                             }
//                         }
//                         Err(err) => return Err(Error::from(err)),
//                     };
//                 }

//                 Ok(replaced)
//             }
//             Err(err) => Err(Error::from(err)),
//         }
//     }

//     pub fn resolve_value(&self, payload: Value) -> Result<Value, Error> {
//         match self.resolve(payload) {
//             Ok(value) => match serde_json::from_str(&value) {
//                 Ok(value) => Ok(value),
//                 Err(err) => Err(Error::from(err)),
//             },
//             Err(err) => Err(err),
//         }
//     }
// }

// impl TryFrom<&Value> for Script {
//     type Error = Error;

//     fn try_from(value: &Value) -> Result<Self, Self::Error> {
//         let engine = Engine::new();
//         let script = ScriptInner::new(&engine, value);

//         match serde_json::to_string(&inner.value) {
//             Ok(replaced) => Ok(Self { script, engine }),
//             Err(err) => Err(Error::from(err)),
//         }
//     }
// }

// #[cfg(test)]
// mod test {
//     use super::Script;
//     use serde_json::json;
//     use std::convert::TryFrom;

//     #[test]
//     fn test_interpolation() {
//         let data = json!({
//             "number": 1,
//             "inter": {
//                 "__type": "interpolation",
//                 "__raw": "${ payload.item }",
//                 "__replaced": "#__{123}",
//                 "__scripts": [{
//                     "__target": "#__{123}",
//                     "__script": "payload.item"
//                 }]
//             }
//         });
//         let compare = json!({
//             "number": 1,
//             "inter": 2
//         });

//         let payload = json!({
//             "item": 2,
//         });

//         let script = Script::try_from(&data).unwrap();
//         let resolve = script.resolve_value(payload).unwrap();

//         assert_eq!(compare, resolve);
//     }

//     #[test]
//     fn test_string_interpolation() {
//         let data = json!({
//             "number": 1,
//             "inter": {
//                 "__type": "interpolation",
//                 "__raw": "string interpolation: ${ payload.item }",
//                 "__replaced": "\"string interpolation: \" + #__{123} + \"\"",
//                 "__scripts": [{
//                     "__target": "#__{123}",
//                     "__script": "payload.item"
//                 }]
//             }
//         });
//         let compare = json!({
//             "number": 1,
//             "inter": "string interpolation: 2"
//         });

//         let payload = json!({
//             "item": 2,
//         });

//         let script = Script::try_from(&data).unwrap();
//         let resolve = script.resolve_value(payload).unwrap();

//         assert_eq!(compare, resolve);
//     }

//     #[test]
//     fn test_string_interpolation_2() {
//         let data = json!({
//             "number": 1,
//             "inter": {
//                 "inner": {
//                     "__type": "interpolation",
//                     "__raw": "${ payload.item }",
//                     "__replaced": "#__{123}",
//                     "__scripts": [{
//                         "__target": "#__{123}",
//                         "__script": "payload.item"
//                     }]
//                 },
//                 "other": true
//             }
//         });
//         let compare = json!({
//             "number": 1,
//             "inter": {
//                 "inner": false,
//                 "other": true
//             }
//         });

//         let payload = json!({
//             "item": false,
//         });

//         let script = Script::try_from(&data).unwrap();
//         let resolve = script.resolve_value(payload).unwrap();

//         assert_eq!(compare, resolve);
//     }

//     #[test]
//     fn test_complex() {
//         let data = json!({
//             "inter": {
//                 "inner": {
//                     "__type": "interpolation",
//                     "__raw": "${ payload.item }",
//                     "__replaced": "#__{123}",
//                     "__scripts": [{
//                         "__target": "#__{123}",
//                         "__script": "payload.item"
//                     }]
//                 }
//             }
//         });
//         let compare = json!({
//             "inter": {
//                 "inner": "asd"
//             }
//         });

//         let payload = json!({
//             "item": "asd",
//         });

//         let script = Script::try_from(&data).unwrap();
//         let resolve = script.resolve_value(payload).unwrap();

//         assert_eq!(compare, resolve);
//     }
// }