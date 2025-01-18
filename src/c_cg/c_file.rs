pub struct CFile {
    pub global_inline_c: Vec<String>,
}

impl CFile {}

impl Default for CFile {
    fn default() -> Self {
        Self {
            global_inline_c: Default::default(),
        }
    }
}
