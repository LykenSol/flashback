use crate::timeline::Frame;
use swf_tree as swf;

#[derive(Copy, Clone, Debug)]
pub enum Value<'a> {
    Undefined,
    Null,
    Bool(bool),
    I32(i32),
    F32(f32),
    F64(f64),
    Str(&'a str),

    OpRes(usize),
}

impl<'a> Value<'a> {
    pub fn as_i32(&self) -> Option<i32> {
        match *self {
            Value::I32(x) => Some(x),
            Value::F32(x) if x == (x as i32 as f32) => Some(x as i32),
            Value::F64(x) if x == (x as i32 as f64) => Some(x as i32),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&'a str> {
        match *self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum Op<'a> {
    Play,
    Stop,
    GotoFrame(Frame),

    GetVar(&'a str),
    SetVar(&'a str, Value<'a>),

    GetFn(&'a str),
    Call(Value<'a>, Vec<Value<'a>>),
    // FIXME(eddyb) integrate with GetMember.
    CallMethod(Value<'a>, &'a str, Vec<Value<'a>>),
}

#[derive(Debug)]
pub struct Code<'a> {
    pub ops: Vec<Op<'a>>,
}

impl<'a> Code<'a> {
    pub fn compile(actions: &'a [swf::avm1::Action]) -> Self {
        let mut consts = &[][..];
        let mut regs = vec![];
        let mut stack = vec![];
        let mut ops = vec![];

        for action in actions {
            match action {
                swf::avm1::Action::Play => ops.push(Op::Play),
                swf::avm1::Action::Stop => ops.push(Op::Stop),
                swf::avm1::Action::GotoFrame(goto) => {
                    ops.push(Op::GotoFrame(Frame(goto.frame as u16)));
                }
                // All of frames are loaded ahead of time, no waiting needed.
                swf::avm1::Action::WaitForFrame(_) => {}
                swf::avm1::Action::WaitForFrame2(_) => {
                    stack.pop();
                }

                swf::avm1::Action::ConstantPool(pool) => {
                    consts = &pool.constant_pool;
                }
                swf::avm1::Action::Push(push) => {
                    stack.extend(push.values.iter().map(|value| match *value {
                        swf::avm1::Value::Undefined => Value::Undefined,
                        swf::avm1::Value::Null => Value::Null,
                        swf::avm1::Value::Boolean(x) => Value::Bool(x),
                        swf::avm1::Value::Sint32(x) => Value::I32(x),
                        swf::avm1::Value::Float32(x) => Value::F32(x),
                        swf::avm1::Value::Float64(x) => Value::F64(x),
                        swf::avm1::Value::String(ref s) => Value::Str(s),

                        swf::avm1::Value::Constant(i) => Value::Str(&consts[i as usize]),
                        swf::avm1::Value::Register(i) => regs[i as usize],
                    }));
                }
                swf::avm1::Action::Pop => {
                    stack.pop();
                }
                swf::avm1::Action::GetVariable => match stack.pop().unwrap() {
                    Value::Str(name) => {
                        ops.push(Op::GetVar(name));
                        stack.push(Value::OpRes(ops.len() - 1));
                    }
                    name => {
                        eprintln!("avm1: too dynamic GetVar({:?})", name);
                        break;
                    }
                },
                swf::avm1::Action::SetVariable => {
                    let value = stack.pop().unwrap();
                    match stack.pop().unwrap() {
                        Value::Str(name) => {
                            ops.push(Op::SetVar(name, value));
                            stack.push(Value::OpRes(ops.len() - 1));
                        }
                        name => {
                            eprintln!("avm1: too dynamic SetVar({:?}, {:?})", name, value);
                            break;
                        }
                    }
                }
                swf::avm1::Action::CallFunction => {
                    let name = stack.pop().unwrap();
                    let arg_count = stack.pop().unwrap();
                    match (name, arg_count.as_i32()) {
                        (Value::Str(name), Some(arg_count)) => {
                            let args = (0..arg_count).map(|_| stack.pop().unwrap()).collect();
                            ops.push(Op::GetFn(name));
                            ops.push(Op::Call(Value::OpRes(ops.len() - 1), args));
                            stack.push(Value::OpRes(ops.len() - 1));
                        }
                        _ => {
                            eprintln!(
                                "avm1: too dynamic CallFunction({:?}, {:?})",
                                name, arg_count
                            );
                            break;
                        }
                    }
                }
                swf::avm1::Action::CallMethod => {
                    let name = stack.pop().unwrap();
                    let this = stack.pop().unwrap();
                    let arg_count = stack.pop().unwrap();
                    match (name, arg_count.as_i32()) {
                        (Value::Str(""), Some(arg_count)) | (Value::Undefined, Some(arg_count)) => {
                            let args = (0..arg_count).map(|_| stack.pop().unwrap()).collect();
                            ops.push(Op::Call(this, args));
                            stack.push(Value::OpRes(ops.len() - 1));
                        }
                        (Value::Str(name), Some(arg_count)) => {
                            let args = (0..arg_count).map(|_| stack.pop().unwrap()).collect();
                            ops.push(Op::CallMethod(this, name, args));
                            stack.push(Value::OpRes(ops.len() - 1));
                        }
                        _ => {
                            eprintln!("avm1: too dynamic CallMethod({:?}, {:?})", name, arg_count);
                            break;
                        }
                    }
                }
                _ => {
                    eprintln!("unknown action: {:?}", action);
                    break;
                }
            }
        }

        eprintln!("AVM1 ops: {:?}", ops);

        Code { ops }
    }
}
