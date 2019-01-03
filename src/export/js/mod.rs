use std::fmt;
use std::ops::AddAssign;

pub mod timeline;

pub use crate::__mod_hack__js_code as code;
#[macro_export]
// HACK(eddyb) quick quasi-quoting hack.
macro_rules! __mod_hack__js_code {
    ($($x:expr),*) => {{
        #[allow(unused_imports)]
        use std::fmt::Write as _;

        let mut _code = String::new();
        $(write!(_code, "{}", $x).unwrap();)*
        crate::export::js::Code(_code)
    }}
}

#[derive(Clone, Debug)]
pub struct Code(pub String);

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AddAssign for Code {
    fn add_assign(&mut self, other: Self) {
        self.0.push_str(&other.0);
    }
}

pub fn array(elems: impl IntoIterator<Item = Code>) -> Code {
    let mut code = code! { "[\n" };
    for elem in elems {
        code += code! { "    ", elem.0.replace("\n", "\n    "), ",\n" };
    }
    code += code! { "]" };
    code
}

pub fn object<S: fmt::Display>(props: impl IntoIterator<Item = (S, Code)>) -> Code {
    let mut code = code! { "{\n" };
    for (name, value) in props {
        code += code! { "    ", name, ": ", value.0.replace("\n", "\n    "), ",\n" };
    }
    code += code! { "}" };
    code
}
