use std::collections::BTreeMap;

use crate::escape::get_temp_variable_name;

use super::{CDialect, ToC, c_stmt::Context};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Repr {
    Packed,
    Aligned(usize),
    PackedAligned(usize),
}

impl ToC for Repr {
    fn to_c(&self, _dialect: CDialect, _c_file: &Context) -> Option<String> {
        match self {
            Repr::Packed => Some("__attribute__ ((packed))".to_string()),
            Repr::Aligned(align) => Some(format!("__attribute__ ((aligned({})))", align)),
            Repr::PackedAligned(align) => {
                Some(format!("__attribute__ ((packed, aligned({})))", align))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CType {
    Void,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,

    Struct {
        repr: Option<Repr>,
        fields: BTreeMap<String, CType>,
    },

    BitField {
        fields: BTreeMap<String, (CType, usize)>,
    },

    Array {
        ty: Box<CType>,
        size: Option<usize>,
    },

    Pointer {
        ty: Box<CType>,
    },

    Const {
        ty: Box<CType>,
    },

    FunctionPointer {
        return_ty: Box<CType>,
        arguments: Vec<CType>,
    },

    // ========== better codegen ==========
    UniformCallBack,
    // ========== modern c types ==========
    ModernCExtension(ModernCTypes),
    // ========== glsl types ==========
    GLSLExtension(GLSLType),
    // ========== opencl types ==========
    OpenCLExtension,
    // ========== cuda types ==========
    CudaExtension,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModernCTypes {
    // extended types
    F16,
    I128,
    U128,
    F128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GLSLType {
    Vec {
        // i b u d support
        ty: Box<CType>,
        // 2, 3, 4
        size: usize,
    },
    Mat {
        // i b u d support
        ty: Box<CType>,
        // 2, 3, 4
        rows: usize,
        // 2, 3, 4
        cols: usize,
    },
    // samplers
    // images
    // atomic counters
}

impl ToC for CType {
    fn to_c(&self, dialect: CDialect, c_file: &Context) -> Option<String> {
        match self {
            CType::I8 => Some("signed char".to_string()),
            CType::I16 => Some("signed short int".to_string()),
            CType::I32 => Some("signed int".to_string()),
            CType::I64 => Some("signed long int".to_string()),

            CType::U8 => Some("unsigned char".to_string()),
            CType::U16 => Some("unsigned short int".to_string()),
            CType::U32 => Some("unsigned int".to_string()),
            CType::U64 => Some("unsigned long int".to_string()),
            CType::F32 => Some("float".to_string()),
            CType::F64 => Some("double".to_string()),

            CType::Struct { repr, fields } => {
                let inner = fields
                    .iter()
                    .map(|(name, ty)| format!("{} {};", ty.to_c(dialect, c_file).unwrap(), name))
                    .collect::<Vec<_>>()
                    .join("\n");
                let name = get_temp_variable_name(&c_file.module);
                let repr = repr
                    .clone()
                    .map(|r| r.to_c(dialect, c_file).unwrap())
                    .unwrap_or("".to_string());
                let code = format!("struct {} {} {{\n{}\n}};", name, repr, inner);
                c_file.global_inline_c(code);

                Some(format!("struct {}", name))
            }

            CType::Array { ty, size } => {
                let size_str = size.map(|s| s.to_string()).unwrap_or("".to_string());
                Some(format!(
                    "{}[{}]",
                    ty.to_c(dialect, c_file).unwrap(),
                    size_str
                ))
            }

            CType::Pointer { ty } => Some(format!("{}*", ty.to_c(dialect, c_file).unwrap())),

            CType::Const { ty } => Some(format!("{} const", ty.to_c(dialect, c_file).unwrap())),

            CType::FunctionPointer {
                return_ty,
                arguments,
            } => {
                let name = get_temp_variable_name(&c_file.module);
                let return_ty = return_ty.to_c(dialect, c_file).unwrap();
                let arguments = arguments
                    .iter()
                    .map(|arg| arg.to_c(dialect, c_file).unwrap())
                    .collect::<Vec<_>>()
                    .join(", ");
                let code = format!("typedef {} (*{})({});", return_ty, name, arguments);
                c_file.global_inline_c(code);
                Some(name)
            }
            _ => None,
        }
    }
}
