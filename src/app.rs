use gloo_utils::document;
use wasm_bindgen::JsCast;
use web_sys::{DragEvent, HtmlImageElement};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};
use web_sys::Url;
use yew::{Component, Context, html, Html};

pub enum Msg {
    Dropped(DragEvent),
    Dragged(DragEvent),
    ImageLoaded,
}

pub struct DropPhoto {}

impl Component for DropPhoto {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {}
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
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
            </>
        }
    }
}

//make generic for element type
// fn with_element<F>(id: &str, action: F)
//     where
//         F: Fn(Element),
// {
//     if let Some(element) = document()
//         .get_element_by_id(id) {
//         action(element);
//     }
// }

fn create_element<'a, T>(element_type: &str) -> T
    where
        T: JsCast,
{
    let element = document().create_element(element_type).unwrap();
    element
        .dyn_into::<T>()
        .expect(&format!("Cannot create element {}", element_type))
}

pub fn _get_image_data(canvas: &HtmlCanvasElement, ctx: &CanvasRenderingContext2d) -> ImageData {
    ctx.get_image_data(0.0, 0.0, canvas.width() as f64, canvas.height() as f64).unwrap()
}