use crate::timeline::Frame;

#[derive(Clone, Debug)]
pub enum Value {
    Undefined,
    Null,
    Bool(bool),
    I32(i32),
    F32(f32),
    F64(f64),
    Str(String),

    OpRes(usize),
}

impl Value {
    pub fn as_i32(&self) -> Option<i32> {
        match *self {
            Value::I32(x) => Some(x),
            Value::F32(x) if x == (x as i32 as f32) => Some(x as i32),
            Value::F64(x) if x == (x as i32 as f64) => Some(x as i32),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum Op {
    Play,
    Stop,
    GotoFrame(Frame),
    // FIXME(eddyb) can we statically resolve this?
    GotoLabel(String),
    GetUrl(String, String),

    GetVar(String),
    SetVar(String, Value),

    Call(Value, Vec<Value>),
    // FIXME(eddyb) integrate with GetMember.
    CallMethod(Value, String, Vec<Value>),
}

#[derive(Debug)]
pub struct Code {
    pub ops: Vec<Op>,
}

impl Code {
    pub fn parse_and_compile(mut data: &[u8]) -> Self {
        let mut actions = vec![];
        while data[0] != 0 {
            let (rest, action) = avm1_parser::parse_action(data).unwrap();
            data = rest;
            actions.push(action);
        }
        assert_eq!(data, [0]);

        Code::compile(actions)
    }

    pub fn compile(actions: Vec<avm1_tree::Action>) -> Self {
        let mut consts = vec![];
        let mut regs = vec![];
        let mut stack = vec![];
        let mut ops = vec![];

        // HACK(eddyb) this hides the warnings / inference errors about `regs`.
        // FIXME(eddyb) remove after register writes are implemented.
        regs.push(Value::Undefined);
        regs.pop();

        for action in actions {
            match action {
                avm1_tree::Action::Play => ops.push(Op::Play),
                avm1_tree::Action::Stop => ops.push(Op::Stop),
                avm1_tree::Action::GotoFrame(goto) => {
                    ops.push(Op::GotoFrame(Frame(goto.frame as u16)));
                }
                avm1_tree::Action::GotoLabel(goto) => {
                    ops.push(Op::GotoLabel(goto.label));
                }
                avm1_tree::Action::GetUrl(get_url) => {
                    ops.push(Op::GetUrl(get_url.url, get_url.target));
                }

                // All of frames are loaded ahead of time, no waiting needed.
                avm1_tree::Action::WaitForFrame(_) => {}
                avm1_tree::Action::WaitForFrame2(_) => {
                    stack.pop();
                }

                avm1_tree::Action::ConstantPool(pool) => {
                    consts = pool.constant_pool;
                }
                avm1_tree::Action::Push(push) => {
                    stack.extend(push.values.into_iter().map(|value| match value {
                        avm1_tree::Value::Undefined => Value::Undefined,
                        avm1_tree::Value::Null => Value::Null,
                        avm1_tree::Value::Boolean(x) => Value::Bool(x),
                        avm1_tree::Value::Sint32(x) => Value::I32(x),
                        avm1_tree::Value::Float32(x) => Value::F32(x),
                        avm1_tree::Value::Float64(x) => Value::F64(x),
                        avm1_tree::Value::String(s) => Value::Str(s),

                        // FIXME(eddyb) avoid per-use cloning.
                        avm1_tree::Value::Constant(i) => Value::Str(consts[i as usize].to_string()),
                        avm1_tree::Value::Register(i) => regs[i as usize].clone(),
                    }));
                }
                avm1_tree::Action::Pop => {
                    stack.pop();
                }
                avm1_tree::Action::GetVariable => match stack.pop().unwrap() {
                    Value::Str(name) => {
                        ops.push(Op::GetVar(name));
                        stack.push(Value::OpRes(ops.len() - 1));
                    }
                    name => {
                        eprintln!("avm1: too dynamic GetVar({:?})", name);
                        break;
                    }
                },
                avm1_tree::Action::SetVariable => {
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
                avm1_tree::Action::CallFunction => {
                    let name = stack.pop().unwrap();
                    let arg_count = stack.pop().unwrap();
                    match (name, arg_count.as_i32()) {
                        (Value::Str(name), Some(arg_count)) => {
                            let args = (0..arg_count).map(|_| stack.pop().unwrap()).collect();
                            ops.push(Op::GetVar(name));
                            ops.push(Op::Call(Value::OpRes(ops.len() - 1), args));
                            stack.push(Value::OpRes(ops.len() - 1));
                        }
                        (name, _) => {
                            eprintln!(
                                "avm1: too dynamic CallFunction({:?}, {:?})",
                                name, arg_count
                            );
                            break;
                        }
                    }
                }
                avm1_tree::Action::CallMethod => {
                    let mut name = stack.pop().unwrap();
                    let this = stack.pop().unwrap();
                    let arg_count = stack.pop().unwrap();

                    if let Value::Str(s) = &name {
                        if s.is_empty() {
                            name = Value::Undefined;
                        }
                    }

                    match (name, arg_count.as_i32()) {
                        (Value::Undefined, Some(arg_count)) => {
                            let args = (0..arg_count).map(|_| stack.pop().unwrap()).collect();
                            ops.push(Op::Call(this, args));
                            stack.push(Value::OpRes(ops.len() - 1));
                        }
                        (Value::Str(name), Some(arg_count)) => {
                            let args = (0..arg_count).map(|_| stack.pop().unwrap()).collect();
                            ops.push(Op::CallMethod(this, name, args));
                            stack.push(Value::OpRes(ops.len() - 1));
                        }
                        (name, _) => {
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

        Code { ops }
    }
}
