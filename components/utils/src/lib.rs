pub mod entity;
pub mod fs;
pub mod json_path;
//pub mod progress;
pub mod symlink;
pub mod url;

pub fn generate_id(len: i32) -> String {
    let mut s = "".to_string();
    for _ in 0..len {
        let x = rand::random::<u8>();
        s.push_str(&format!("{:x}", x));
    }
    s
}
