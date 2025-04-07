use alloc::{string::{String, ToString}, vec::Vec, vec};
use conquer_once::spin::OnceCell;
use log::warn;

use crate::{syscall, GETERRNO, OPEN};

use super::file::FileFlags;

/// list of users, will be initialized by internal::user::init()
pub static USERS: OnceCell<Vec<User>> = OnceCell::uninit();

/// initialize the list of users
pub fn init() {
    let mut buf = vec![0; 2048];

    let path = "/etc/users";
    let path_ptr = path.as_ptr() as usize;
    let path_len = path.len();

    let etc_users = syscall!(OPEN, path_ptr, path_len, FileFlags::Read as u8);

    let errno = syscall!(GETERRNO);
    if errno != 0 {
        warn!("failed to open /etc/users {}", errno);
        return;
    }

    syscall!(
        syscall::READ,
        etc_users,
        buf.as_mut_ptr() as usize,
        buf.len()
    );

    let errno = syscall!(GETERRNO);
    if errno != 0 {
        warn!("failed to read /etc/users {}", errno);
        return;
    }

    let data = String::from_utf8(buf)
        .unwrap()
        .chars()
        .filter(|c| c.is_ascii_graphic() || c.is_ascii_whitespace())
        .collect::<String>();

    let users: Vec<User> = data
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| User::try_from(line).unwrap())
        .collect();

    USERS.init_once(|| users);
}

/// represents a user
#[derive(Debug, Clone)]
pub struct User {
    /// user id
    pub uid: u64,
    /// user name
    pub name: String,
    /// password hash (sha256)
    pub passhash: String,
    /// home directory
    pub home_dir: String,
    /// default shell
    pub shell: String,
    /// user groups
    pub groups: Vec<String>,
}

impl User {
    /// check if the password is correct
    pub fn check_password(&self, pass: &str) -> bool {
        let passhash = hex::encode(pass);
        passhash == self.passhash
    }

    /// create a new user
    pub fn new(uid: u64, name: &str, pass: &str, home_dir: &str, shell: &str, groups: Vec<String>) -> Self {
        let passhash = hex::encode(pass);
        User {
            uid,
            name: name.to_string(),
            passhash,
            home_dir: home_dir.to_string(),
            shell: shell.to_string(),
            groups,
        }
    }
}

impl TryFrom<&str> for User {
    type Error = &'static str;

    /// format: name:passhash:uid:groups:home_dir:shell
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 6 {
            return Err("invalid user entry");
        }

        let uid = parts[2].parse().map_err(|_| "invalid uid")?;
        let name = parts[0].to_string();
        let passhash = parts[1].to_string();
        let home_dir = parts[4].to_string();
        let shell = parts[5].to_string();
        let groups = parts[3].split(',').map(|s| s.to_string()).collect();

        Ok(User {
            uid,
            name,
            passhash,
            home_dir,
            shell,
            groups,
        })
    }
}

/// get a uid by name
pub fn get_uid(name: &str) -> Option<u64> {
    USERS.get().unwrap().iter().find(|u| u.name == name).map(|u| u.uid)
}
