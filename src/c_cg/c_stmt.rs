use core::panic;
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
};

use crate::escape::{
    format_to_escape_replace, get_temp_variable_name, string_to_escape_to_c_ansi_id,
};

use super::{CDialect, ToC, c_arch::Arch, c_file::CFile, c_type::CType, c_value::CValue};
pub type Variable = String;

pub struct Context {
    pub c_file: Arc<Mutex<CFile>>,
    pub module: String,
    pub dialect: CDialect,
    pub variables: Mutex<BTreeSet<Variable>>,
    pub current_source: Mutex<String>,
}

impl Context {
    pub fn standard(module_name: String) -> Self {
        Self {
            c_file: Default::default(),
            module: module_name,
            dialect: CDialect::Standard,
            variables: Default::default(),
            current_source: Default::default(),
        }
    }
}

impl Context {
    pub fn global_inline_c(&self, code: String) -> &Self {
        self.c_file.lock().unwrap().global_inline_c.push(code);
        self
    }

    pub fn local_inline_c(self, code: String) -> Self {
        let code = format!("do {{{}}} while(0);\n", code);
        self.current_source.lock().unwrap().push_str(&code);
        self
    }

    pub fn inline_asm(
        &self,
        arch: Arch,
        volatile: bool,
        code: Vec<String>,
        inputs: Vec<Variable>,
        outputs: Vec<Variable>,
        input_output: Vec<Variable>,
        changed_registers: Vec<String>,
    ) -> &Self {
        if self.dialect != CDialect::Standard {
            panic!("inline asm is not supported in dialect {:?}", self.dialect);
        }

        let arch = format!("#ifdef {}", arch.to_c(self.dialect, self).unwrap());
        let code = code
            .into_iter()
            .map(|x| format!("\"{}\"", x))
            .collect::<Vec<_>>()
            .join("\n");
        let mut mapping = String::new();

        if outputs.is_empty() {
            mapping.push_str("\n: ");

            let outputs = outputs
                .into_iter()
                .map(|x| format!(" \"=r\"({})", x))
                .collect::<Vec<_>>()
                .join(", ");
            mapping.push_str(&outputs);
        }

        if inputs.is_empty() {
            mapping.push_str("\n: ");

            let inputs = inputs
                .into_iter()
                .map(|x| format!(" \"r\"({})", x))
                .collect::<Vec<_>>()
                .join(", ");
            mapping.push_str(&inputs);
        }

        if input_output.is_empty() {
            mapping.push_str("\n: ");

            let input_output = input_output
                .into_iter()
                .map(|x| format!(" \"r+\"({})", x))
                .collect::<Vec<_>>()
                .join(", ");
            mapping.push_str(&input_output);
        }

        if changed_registers.is_empty() {
            mapping.push_str("\n: ");

            let changed_registers = changed_registers
                .into_iter()
                .map(|x| format!(" \"{}\"", x))
                .collect::<Vec<_>>()
                .join(", ");
            mapping.push_str(&changed_registers);
        }

        let asm = format!(
            "{}\n __asm__ {} (\n{}\n{})\n#endif\n",
            arch,
            if volatile { "volatile" } else { "" },
            code,
            mapping
        );
        self.current_source.lock().unwrap().push_str(&asm);
        self
    }

    // pub fn decl(&self, name: String, ty: CType, value: CValue) -> &Self {
    //     let name = string_to_escape_to_c_ansi_id(&self.module,&name);
    //     let ty = ty.to_c(self.dialect, self).unwrap();
    //     let value = value.to_c(self.dialect, self).unwrap();
    //     self.current_source
    //         .lock()
    //         .unwrap()
    //         .push_str(&format!("{} {} = {};\n", ty, name, value));
    //     self
    // }

    pub fn set(&self, ty: CType, name: Variable, value: CValue) -> &Self {
        if self.variables.lock().unwrap().insert(name.clone()) {
            let name = string_to_escape_to_c_ansi_id(&self.module, &name);
            let ty = ty.to_c(self.dialect, self).unwrap();
            let value = value.to_c(self.dialect, self).unwrap();
            self.current_source
                .lock()
                .unwrap()
                .push_str(&format!("{} {} = {};\n", ty, name, value));
        } else {
            let name = string_to_escape_to_c_ansi_id(&self.module, &name);
            self.current_source.lock().unwrap().push_str(&format!(
                "{} = {};\n",
                name,
                value.to_c(self.dialect, self).unwrap()
            ));
        }

        self
    }

    fn decl_tmp(&self, ty: &CType) -> (&Self, String) {
        let name = get_temp_variable_name(&self.module);
        self.current_source.lock().unwrap().push_str(&format!(
            "{} {};\n",
            ty.to_c(self.dialect, self).unwrap(),
            name.clone()
        ));
        (self, name)
    }

    pub fn block(&self, ty: &CType, block: impl Fn(Self, Variable) -> Self) -> &Self {
        let (_, ret) = self.decl_tmp(ty);
        let s = block(
            Context {
                c_file: self.c_file.clone(),
                dialect: self.dialect,
                module: self.module.clone(),
                variables: Mutex::new(self.variables.lock().unwrap().clone()),
                current_source: Mutex::new(String::new()),
            },
            ret.clone(),
        );
        let block = s.current_source.lock().unwrap().clone();
        self.current_source
            .lock()
            .unwrap()
            .push_str(format!("{{\n{}\n}}", block).as_str());
        self
    }

    /// variable in pragma should escape with {variable}
    pub fn raw_pragma(&self, mut code: String) -> &Self {
        if self.dialect != CDialect::Standard {
            panic!("raw pragma is not supported in dialect {:?}", self.dialect);
        }
        code = format_to_escape_replace(&self.module, code);
        self.current_source
            .lock()
            .unwrap()
            .push_str(&format!("#pragma {}\n", code));
        self
    }

    // if else if else
    /// in `builder` the function param is a new context and the `phi` variable
    pub fn cond(
        &self,
        ty: &CType,
        conds: Vec<CValue>,
        builder: Vec<Box<dyn Fn(Self, Variable) -> Self>>,
        otherwise: impl Fn(Self, Variable) -> Self,
    ) -> &Self {
        if conds.is_empty() || builder.len() != conds.len() || builder.is_empty() {
            panic!(
                "malformed cond expecting at least one condition and one block, got {} conditions and {} blocks",
                conds.len(),
                builder.len()
            );
        } else {
            let mut code = String::new();
            let (_, phi) = self.decl_tmp(ty);
            for (i, (cond, block)) in conds.into_iter().zip(builder.into_iter()).enumerate() {
                let s = block(
                    Context {
                        c_file: self.c_file.clone(),
                        dialect: self.dialect,
                        module: self.module.clone(),
                        variables: Mutex::new(self.variables.lock().unwrap().clone()),
                        current_source: Mutex::new(String::new()),
                    },
                    phi.clone(),
                );
                let block = s.current_source.lock().unwrap().clone();

                let cond = cond.to_c(self.dialect, &s).unwrap();
                if i == 0 {
                    code.push_str(&format!("if({}) {{\n{}\n", cond, block));
                } else {
                    code.push_str(&format!("}} else if({}) {{\n{}\n", cond, block));
                }
            }

            let s = otherwise(
                Context {
                    c_file: self.c_file.clone(),
                    dialect: self.dialect,
                    module: self.module.clone(),
                    variables: Mutex::new(self.variables.lock().unwrap().clone()),
                    current_source: Mutex::new(String::new()),
                },
                phi.clone(),
            );
            let block = s.current_source.lock().unwrap().clone();
            code.push_str(&format!("}} else {{\n{}\n}}\n", block));
            s.current_source.lock().unwrap().push_str(&code);
            self
        }
    }

    pub fn for_loop(
        &self,
        init: Option<(CType, String, CValue)>,
        condition: Option<CValue>,
        step: Option<CValue>,

        block: impl Fn(Self) -> (Self, Variable),
    ) -> &Self {
        let init = init
            .map(|(ty, name, value)| {
                let name = string_to_escape_to_c_ansi_id(&self.module, &name);
                let ty = ty.to_c(self.dialect, self).unwrap();
                let value = value.to_c(self.dialect, self).unwrap();
                format!("{} {} = {};", ty, name, value)
            })
            .unwrap_or(";".to_string());
        let condition = condition
            .map(|c| c.to_c(self.dialect, self).unwrap())
            .unwrap_or(";".to_string());
        let step = step
            .map(|c| c.to_c(self.dialect, self).unwrap())
            .unwrap_or("".to_string());
        let (block, _) = block(Context {
            c_file: self.c_file.clone(),
            dialect: self.dialect,
            module: self.module.clone(),
            variables: Mutex::new(self.variables.lock().unwrap().clone()),
            current_source: Mutex::new(String::new()),
        });
        let block = block.current_source.lock().unwrap().clone();
        let code = format!("for({}{}{}) {{{}}}", init, condition, step, block);
        self.current_source.lock().unwrap().push_str(&code);
        self
    }

    pub fn def(
        &self,
        name: Variable,
        ret: CType,
        args: Vec<(CType, Variable)>,
        body: impl Fn(Self) -> Self,
    ) -> &Self {
        let name = string_to_escape_to_c_ansi_id(&self.module, &name);
        let ret = ret.to_c(self.dialect, self).unwrap();
        let args = args
            .iter()
            .map(|(ty, name)| {
                format!(
                    "{} {}",
                    ty.to_c(self.dialect, self).unwrap(),
                    string_to_escape_to_c_ansi_id(&self.module, name)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let body = body(Context {
            c_file: self.c_file.clone(),
            dialect: self.dialect,
            module: self.module.clone(),
            variables: Mutex::new(self.variables.lock().unwrap().clone()),
            current_source: Mutex::new(String::new()),
        });
        let body = body.current_source.lock().unwrap().clone();
        let code = format!("{} {}({}) {{{}}}", ret, name, args, body);
        self.global_inline_c(code);
        self
    }
}
