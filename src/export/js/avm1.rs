use crate::avm1;
use crate::export::js;

impl<'a> avm1::Value<'a> {
    fn to_js(self) -> js::Code {
        match self {
            avm1::Value::Undefined => js::code! { "undefined" },
            avm1::Value::Null => js::code! { "null" },
            avm1::Value::Bool(false) => js::code! { "false" },
            avm1::Value::Bool(true) => js::code! { "true" },
            avm1::Value::I32(x) => js::code! { x },
            avm1::Value::F32(x) => js::code! { x },
            avm1::Value::F64(x) => js::code! { x },
            avm1::Value::Str(s) => js::string(s),

            avm1::Value::OpRes(i) => js::code! { "_", i },
        }
    }
}

pub fn export(codes: &[avm1::Code]) -> js::Code {
    let mut js_body = js::code! {};

    fn this_call(name: &str, args: impl IntoIterator<Item = js::Code>) -> js::Code {
        js::call(js::code! { "local.this.", name }, args)
    }

    for code in codes {
        for (i, op) in code.ops.iter().enumerate() {
            let assign = |value| js::code! { "var _", i, " = ", value };
            js_body += js::code! { "\n" };
            js_body += match op {
                avm1::Op::Play => this_call("play", vec![]),
                avm1::Op::Stop => this_call("stop", vec![]),
                avm1::Op::GotoFrame(frame) => this_call("gotoAndPlay", vec![js::code! { frame.0 }]),

                avm1::Op::GetVar(name) => assign(js::code! {
                    "(", js::string(name), " in local) ? ",
                    "local[", js::string(name), "] : ",
                    "global[", js::string(name), "]"
                }),
                avm1::Op::SetVar(name, value) => js::code! {
                    "local[", js::string(name), "] = ", value.to_js()
                },

                avm1::Op::Call(callee, args) => assign(
                    js::call(callee.to_js(), args.iter().map(|arg| arg.to_js()))
                ),
                avm1::Op::CallMethod(receiver, name, args) => assign(js::call(
                    js::code! { receiver.to_js(), ".", name },
                    args.iter().map(|arg| arg.to_js()),
                )),
            };
            js_body += js::code! { ";" };
        }
    }

    js::code! { "function(global, local) {", js_body.indent(), "\n}" }
}
