#[macro_use]
extern crate pipe_core;

use std::convert::TryFrom;

use pipe_core::{
    modules::{Config, Listener, Return},
    scripts::Params,
    serde_json::{Map, Value},
};

macro_rules! message {
    ($level:expr, $message:expr) => {
        format!(r#"{{"level":"{}", "message": {}}}"#, $level, $message)
    };
}

enum OutputType {
    Stdout,
}

struct Output {
    output_type: OutputType,
}

impl Output {
    pub fn new(options: Map<String, Value>) -> Self {
        let output_type = {
            let ottype = match options.get("type") {
                Some(value) => value.as_str().unwrap().to_string(),
                None => "stdout".to_string(),
            };

            if ottype.eq("stdout") {
                OutputType::Stdout
            } else {
                OutputType::Stdout
            }
        };
        Self { output_type }
    }

    pub fn send(&self, message: String) {
        match self.output_type {
            OutputType::Stdout => println!("{}", message),
        }
    }
}

pub fn pipe_log<F: Fn(Return)>(listener: Listener, send: F, config: Config) {
    match config.params {
        Some(params_raw) => {
            let mut params = Params::try_from(&params_raw).unwrap();
            let level = match params_raw.as_object().unwrap().get("level") {
                Some(value) => value.as_str().unwrap().to_string(),
                None => "info".to_string(),
            };

            let options = config
                .module_params
                .get("output")
                .unwrap()
                .as_object()
                .unwrap()
                .clone();

            let output = Output::new(options);

            for request in listener {
                match params.set_request(&request) {
                    Ok(_) => match params.get_param("message") {
                        Ok(message) => {
                            output.send(message!(level, message));

                            send(Return {
                                payload: request.payload,
                                attach: config.default_attach.clone(),
                                trace_id: request.trace_id,
                            })
                        }
                        Err(err) => {
                            output.send(message!("error", err));

                            send(Return {
                                payload: request.payload,
                                attach: config.default_attach.clone(),
                                trace_id: request.trace_id,
                            })
                        }
                    },
                    Err(err) => {
                        output.send(message!("error", err));

                        send(Return {
                            payload: request.payload,
                            attach: config.default_attach.clone(),
                            trace_id: request.trace_id,
                        })
                    }
                }
            }
        }
        _ => panic!("No params"),
    };
}

create_module!(pipe_log);
