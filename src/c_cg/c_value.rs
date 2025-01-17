use std::collections::HashMap;

use crate::escape::{self, get_temp_variable_name};

use super::{ToC, c_stmt::Context, c_type::CType};

#[derive(Debug, Clone)]
pub enum IntegerSuffix {
    None,
    U32,
    I64,
    U64,
}

#[derive(Debug, Clone)]
pub enum FloatSuffix {
    None,
    F32,
    F64,
}

#[derive(Debug, Clone)]
pub enum CLiteral {
    Int(usize, IntegerSuffix),
    Float(f64, FloatSuffix),
    CChar(char),
    CString(String),
}

impl ToC for CLiteral {
    fn to_c(&self, dialect: super::CDialect, _c: &Context) -> Option<String> {
        match self {
            CLiteral::Int(v, suffix) => {
                let suffix = if dialect == super::CDialect::Standard {
                    match suffix {
                        IntegerSuffix::None => "".to_string(),
                        IntegerSuffix::U32 => "u".to_string(),
                        IntegerSuffix::I64 => "l".to_string(),
                        IntegerSuffix::U64 => "ul".to_string(),
                    }
                } else {
                    "".to_string()
                };
                Some(format!("{}{}", v, suffix))
            }
            CLiteral::Float(v, suffix) => {
                let suffix = if dialect == super::CDialect::Standard {
                    match suffix {
                        FloatSuffix::None => "".to_string(),
                        FloatSuffix::F32 => "f".to_string(),
                        FloatSuffix::F64 => "l".to_string(),
                    }
                } else {
                    "".to_string()
                };
                Some(format!("{}{}", v, suffix))
            }
            CLiteral::CChar(v) => Some(format!("'{}'", v)),
            CLiteral::CString(v) => Some(format!("\"{}\"", v)),
        }
    }
}

pub enum CValue {
    Literal(CLiteral),
    Variable(String),
    // CType should be array of something
    Array(CType, Vec<CValue>),
    Struct(HashMap<String, CValue>),
    Union(HashMap<String, CValue>),
    Reference(Box<CValue>),
    Dereference(Box<CValue>),
    MemberAccess(Box<CValue>, String),
    IndexAccess(Box<CValue>, Box<CValue>),
    FunctionCall(Box<CValue>, Vec<CValue>),
    BinOp(String, Box<CValue>, Box<CValue>),
    PrefixOp(String, Box<CValue>),
    PostfixOp(String, Box<CValue>),

    // compile with LLVM Enzyme Plugin
    AutoDiff(String, Vec<CValue>),
}

impl ToC for CValue {
    fn to_c(&self, dialect: super::CDialect, context: &Context) -> Option<String> {
        use CValue::*;
        match self {
            Literal(v) => v.to_c(dialect, context),
            Variable(v) => Some(escape::string_to_escape_to_c_ansi_id(&context.module, v)),
            Array(ty, values) => {
                let values = values
                    .iter()
                    .map(|value| value.to_c(dialect, context).unwrap())
                    .collect::<Vec<_>>()
                    .join(", ");
                let name = get_temp_variable_name(&context.module);
                if let CType::Array { ty, size } = ty {
                    let size = size.map(|s| s.to_string()).unwrap_or("".to_string());
                    let code = format!(
                        "{} {}[{}] = {{ {} }};",
                        ty.to_c(dialect, context).unwrap(),
                        name,
                        size,
                        values
                    );
                    context.current_source.lock().unwrap().push_str(&code);
                } else {
                    panic!("Array type should be array");
                }
                Some(name)
            }
            Struct(fields) => {
                let fields = fields
                    .iter()
                    .map(|(name, value)| {
                        format!(".{} = {}", name, value.to_c(dialect, context).unwrap())
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                Some(format!("{{ {} }}", fields))
            }
            Union(fields) => {
                if context.dialect != super::CDialect::Standard {
                    panic!("Union is not supported in {:?}", context.dialect);
                }
                let fields = fields
                    .iter()
                    .map(|(name, value)| {
                        format!(".{} = {}", name, value.to_c(dialect, context).unwrap())
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                Some(format!("{{ {} }}", fields))
            }
            Reference(value) => {
                let value = value.to_c(dialect, context).unwrap();
                Some(format!("(&{})", value))
            }
            Dereference(value) => {
                let value = value.to_c(dialect, context).unwrap();
                Some(format!("(*{})", value))
            }
            MemberAccess(value, member) => {
                let value = value.to_c(dialect, context).unwrap();
                Some(format!("({}.{})", value, member))
            }
            IndexAccess(value, index) => {
                let value = value.to_c(dialect, context).unwrap();
                let index = index.to_c(dialect, context).unwrap();
                Some(format!("({}[{}])", value, index))
            }
            FunctionCall(func, args) => {
                let func = func.to_c(dialect, context).unwrap();
                let args = args
                    .iter()
                    .map(|arg| arg.to_c(dialect, context).unwrap())
                    .collect::<Vec<_>>()
                    .join(", ");
                Some(format!("({}({}))", func, args))
            }
            BinOp(op, lhs, rhs) => {
                let lhs = lhs.to_c(dialect, context).unwrap();
                let rhs = rhs.to_c(dialect, context).unwrap();
                Some(format!("({} {} {})", lhs, op, rhs))
            }
            PrefixOp(op, value) => {
                let value = value.to_c(dialect, context).unwrap();
                Some(format!("({}{})", op, value))
            }
            PostfixOp(op, value) => {
                let value = value.to_c(dialect, context).unwrap();
                Some(format!("({}{})", value, op))
            }
            AutoDiff(_op, _args) => todo!(),
        }
    }
}
