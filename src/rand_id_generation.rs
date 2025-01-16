use crate::escape;

pub fn get_temp_variable_name() -> String {
    let mut res = String::new();
    res.push_str("_tmp_");
    res.push_str(&escape::string_to_escape_to_c_ansi_id(
        &uuid::Uuid::now_v7().to_string(),
    ));
    res
}
