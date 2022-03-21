#[macro_export(local_inner_macros)]
macro_rules! render {
    ($engine:ident, $type:ty, $payload:expr, $target:expr) => {{
        let mut scope = $crate::scripts::rhai::Scope::new();
        match $crate::scripts::rhai::serde::to_dynamic($payload) {
            Ok(value) => {
                scope.push_dynamic("payload", value);
                $engine.eval_with_scope::<$type>(&mut scope, $target)
            }
            Err(err) => Err(err),
        }
    }};
}

#[macro_export(local_inner_macros)]
macro_rules! param_test {
    ($value: expr) => {
        $crate::serde_json::json!({
            "___PIPE___type": "script",
            "___PIPE___list": $value
        })
    };
}
