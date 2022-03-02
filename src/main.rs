extern crate lazy_static;

mod app;
mod transform;
mod quantizer;
mod samples;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::start_app::<crate::app::DropPhoto>();
}