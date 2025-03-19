use alloc::{collections::BTreeMap, string::String};

pub struct Process {
    dir: String,
    env_vars: BTreeMap<String, String>,
    user: String,

}
