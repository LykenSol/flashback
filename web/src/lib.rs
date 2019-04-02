use futures::Future;
use js_sys::{Promise, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{future_to_promise, JsFuture};
use web_sys::{console, Element, Request, RequestInit, RequestMode, Response, SvgScriptElement};

fn convert_swf(swf: &[u8]) -> String {
    match swf_parser::parsers::movie::parse_movie(swf) {
        Ok((remaining, movie)) => {
            if !remaining.is_empty() {
                console::log_1(
                    &format!(
                        "swf-parser parsing incomplete: {} bytes left",
                        remaining.len()
                    )
                    .into(),
                );
            }

            flashback::export::svg::export(&movie, flashback::export::svg::Config { use_js: true })
                .to_string()
        }
        Err(e) => format!("swf-parser errored: {:?}", e),
    }
}

fn load_swf_from_url(container: Element, url: &str) {
    container.set_inner_html(&format!("Loading `{}`...", url));

    let mut opts = RequestInit::new();
    opts.mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(
        &format!("https://cors-anywhere.herokuapp.com/{}", url),
        &opts,
    )
    .unwrap();

    let window = web_sys::window().unwrap();
    future_to_promise(
        JsFuture::from(window.fetch_with_request(&request))
            .and_then(|resp_value| {
                assert!(resp_value.is_instance_of::<Response>());
                let resp: Response = resp_value.dyn_into().unwrap();
                resp.array_buffer()
            })
            .and_then(|buffer_value: Promise| JsFuture::from(buffer_value))
            .map(move |buffer_value| {
                let buffer = Uint8Array::new(&buffer_value);
                let mut data = vec![0; buffer.length() as usize];
                buffer.copy_to(&mut data);

                container.set_inner_html(&convert_swf(&data));

                // HACK(eddyb) manually evaluate script sources;
                if let Some(script) = container.query_selector("script").unwrap() {
                    if let Ok(script) = script.dyn_into::<SvgScriptElement>() {
                        js_sys::eval(&script.text_content().unwrap()).unwrap();
                    }
                }

                JsValue::undefined()
            }),
    );
}

fn load_swf_from_hash(container: Element) {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let hash = location.hash().unwrap();
    if !hash.starts_with("#") {
        container.set_inner_html(&format!(
            "Please navigate to {}#foo.com/path/to/flash/file.swf",
            location.href().unwrap()
        ));
    } else {
        load_swf_from_url(container, &hash[1..]);
    }
}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let body = document.body().unwrap();
    let container = document.create_element("pre")?;
    body.append_child(&container)?;

    load_swf_from_hash(container.clone());

    {
        let closure = Closure::wrap(Box::new(move || {
            load_swf_from_hash(container.clone());
        }) as Box<dyn FnMut()>);
        let window_et: web_sys::EventTarget = window.into();
        window_et
            .add_event_listener_with_callback("hashchange", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    Ok(())
}
