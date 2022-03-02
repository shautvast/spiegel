use gloo_utils::document;
use image::RgbImage;
use wasm_bindgen::{Clamped, JsCast};
use wasm_bindgen::prelude::*;
use web_sys::{DragEvent, HtmlImageElement};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};
use web_sys::Url;
use yew::{Component, Context, html, Html};

use crate::{samples, transform};
use crate::samples::{ColorSample, Samples};

pub enum Msg {
    Dropped(DragEvent),
    Dragged(DragEvent),
    ImageLoaded,
}

pub struct DropPhoto {
    samples: Samples
}

impl Component for DropPhoto {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            samples: Samples::new()
        }
    }

    fn update(& mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Dragged(event) => {
                event.prevent_default();
                false
            }
            Msg::Dropped(event) => {
                event.prevent_default();
                let data_transfer = event
                    .data_transfer()
                    .expect("Event should have DataTransfer");
                let item_list = data_transfer.items();
                for i in 0..item_list.length() {
                    let item = item_list.get(i).expect("Should find an item");
                    if item.kind() == "file" {
                        let file = item
                            .get_as_file()
                            .expect("Should find a file here")
                            .unwrap();
                        let url = Url::create_object_url_with_blob(&file).expect("Cannot create url");
                        let img = document().get_element_by_id("source-image").expect("cannot get #source-image").dyn_into::<HtmlImageElement>().unwrap();
                        img.set_src(&url);
                    }
                }
                true
            }
            Msg::ImageLoaded => {
                if let Some(canvas) = document().get_element_by_id("source").and_then(|e| e.dyn_into::<HtmlCanvasElement>().ok()) {
                    let ctx: CanvasRenderingContext2d = canvas
                        .get_context("2d")
                        .unwrap()
                        .unwrap()
                        .dyn_into::<CanvasRenderingContext2d>()
                        .unwrap();
                    if let Some(img) = document().get_element_by_id("source-image").and_then(|e| e.dyn_into::<HtmlImageElement>().ok()) {
                        canvas.set_width(img.width());
                        canvas.set_height(img.height());
                        ctx.draw_image_with_html_image_element(&img, 0.0, 0.0).expect("Cannot draw image on canvas");
                    }
                    if let Some(drop_zone) = document().get_element_by_id("drop-zone") {
                        drop_zone.set_attribute("style", "display:none").expect("Cannot update attribute");
                    }

                    let imgdata = ctx
                        .get_image_data(0.0, 0.0, canvas.width() as f64, canvas.height() as f64)
                        .unwrap();
                    let raw_pixels: Vec<u8> = imgdata.data().to_vec();
                    let rgb_src = RgbImage::from_raw(canvas.width(), canvas.height(), raw_pixels).unwrap();
                    let transformed = transform::apply(&rgb_src).expect("Cannot transform image");
                    let image_data = ImageData::new_with_u8_clamped_array_and_sh(Clamped(&rgb_src.to_vec()),
                                                                          canvas.width(), canvas.height());

                    ctx.put_image_data(&image_data.expect(""), 0.0, 0.0);
                }
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        html! {
            <>
            <div id="drop-zone" class="drop-zone"
                ondragover={link.callback(|e| Msg::Dragged(e))}
                ondrop={link.callback(|e| Msg::Dropped(e))}>
                <p>{ "drag your photos here" }</p>
            </div>
            <img id="source-image" style="display:none" onload={link.callback(|_| Msg::ImageLoaded)}/>
            <canvas id="source"></canvas>
            <canvas id="dest"></canvas>
            <div id="samples" class="hidden"></div>
            <canvas id="buffer" class="hidden"></canvas>
            </>
        }
    }
}

// if the transformer needs a new sample, it uses HtmlImageElement to download it.
// all is async
pub fn add_sample(name: &'static str) {
    let sample = document().create_element("img").unwrap().dyn_into::<HtmlImageElement>().expect("Cannot create img element");
    sample.set_src(&format!("/static/samples/{}.jpg", name));
    let samples = document().get_element_by_id("samples").unwrap();

    let image_loaded = Closure::wrap(Box::new( |_: web_sys::Event| {
        if let Some(canvas) = document().get_element_by_id("buffer").and_then(|e| e.dyn_into::<HtmlCanvasElement>().ok()) {
            let ctx: CanvasRenderingContext2d = canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .dyn_into::<CanvasRenderingContext2d>()
                .unwrap();
            canvas.set_width(sample.width());
            canvas.set_height(sample.height());
            ctx.draw_image_with_html_image_element(&sample, 0.0, 0.0).expect("Cannot draw image on canvas");
            let imgdata = ctx
                .get_image_data(0.0, 0.0, canvas.width() as f64, canvas.height() as f64)
                .unwrap();
            let raw_pixels: Vec<u8> = imgdata.data().to_vec();
            let sample = RgbImage::from_raw(canvas.width(), canvas.height(), raw_pixels).unwrap();
            samples::insert(name.to_owned(), ColorSample::new(&name, sample));
        }
    }) as Box<dyn FnMut(_)>);
    sample.add_event_listener_with_callback("load", image_loaded.as_ref().unchecked_ref()).expect("cannot add onload listener");
    samples.append_child(&sample);
}

fn create_element<'a, T>(element_type: &str) -> T
    where
        T: JsCast,
{
    let element = document().create_element(element_type).unwrap();
    element
        .dyn_into::<T>()
        .expect(&format!("Cannot create element {}", element_type))
}