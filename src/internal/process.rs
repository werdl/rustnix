use alloc::{collections::BTreeMap, string::String};

use crate::internal::io::Resource;

pub struct Process {
    dir: String,
    env_vars: BTreeMap<String, String>,
    user: String,

    /// should hold stdin, stdout, stderr
    streams: BTreeMap<String, Resource>,
}
