use crate::export::js;

// TODO(eddyb) figure out a way to avoid copying the data URL around.
pub fn export_mp3(mp3: &[u8]) -> js::Code {
    let mut code = js::code! { "new Audio('data:audio/mpeg;base64," };
    base64::encode_config_buf(mp3, base64::STANDARD, &mut code.0);
    code += js::code! { "')" };
    code
}
