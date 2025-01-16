pub mod c_arch;
pub mod c_file;
pub mod c_stmt;
pub mod c_type;
pub mod c_value;

use c_stmt::Context;

pub trait ToC {
    fn to_c(&self, dialect: CDialect, context: &Context) -> Option<String>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CDialect {
    // glsl, opencl/cuda host, etc.
    Kernel,

    // openmp, openacc, mpi
    Parallel,

    // standard c11 memory model C language
    Standard,
}
