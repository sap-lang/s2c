use super::{ToC, c_stmt::Context};

/// now we are not considering embedded systems
pub enum Arch {
    WindowsX86,
    WindowsX86_64,
    WindowsAArch64,

    PosixX86_64,
    PosixAArch64,
    PosixRiscv64GC,
    Posixloongarch64,

    EmscriptenWasm32,
    // future plan: esp32? stm32? riscv32imac? ios? android?
}

impl ToC for Arch {
    fn to_c(&self, dialect: super::CDialect, _context: &Context) -> Option<String> {
        assert!(matches!(dialect, super::CDialect::Standard));
        match self {
            Arch::WindowsX86 => {
                Some("defined(_WIN32) && defined(_M_IX86) || defined(__i386__)".to_string())
            }
            Arch::WindowsX86_64 => {
                Some("defined(_WIN32) && defined(_M_X64) || defined(__x86_64__)".to_string())
            }
            Arch::WindowsAArch64 => {
                Some("defined(_WIN32) && defined(_M_ARM64) || defined(__aarch64__)".to_string())
            }
            Arch::PosixX86_64 => Some("defined(__unix__) && defined(__x86_64__)".to_string()),
            Arch::PosixAArch64 => Some("defined(__unix__) && defined(__aarch64__)".to_string()),
            Arch::PosixRiscv64GC => Some("defined(__unix__) && defined(__riscv)".to_string()),
            Arch::Posixloongarch64 => {
                Some("defined(__unix__) && defined(__loongarch64)".to_string())
            }
            Arch::EmscriptenWasm32 => {
                Some("defined(__EMSCRIPTEN__) && defined(__wasm32__)".to_string())
            }
        }
    }
}
