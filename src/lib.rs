mod accessor;
pub mod args;
pub mod config;
mod context;
mod data_model;
pub mod discovery;
mod doc;
mod doc_builder;
mod enum_def;
pub mod formatter;
mod formatting_session;
pub mod message_helper;
mod source_formatter;
mod utility;
use formatter::Formatter;

pub fn format(f: Formatter) -> Vec<Result<String, String>> {
    f.format()
}

pub fn format_source(source_code: &str, config: formatter::Config) -> Result<String, String> {
    Formatter::try_format_source(source_code, config)
}

//#[wasm_bindgen]
//pub fn greet(source_code: &str) -> String {
//    let config = Config::default();
//    Formatter::format_one(source_code, config)
//}

//#[wasm_bindgen]
//pub fn greet(source_code: &str) -> String {
//    "hello".to_string()
//}

//#[wasm_bindgen]
//pub fn greet() -> Result<String, JsValue> {
//    Ok("hello world!".to_string())
//}
