use std::{collections::HashMap, path::Path};

mod fs;
mod img;

fn list_files(
    initial_path: &Path,
    dir: &Path,
    ignore_subdirs: &[String],
) -> Result<HashMap<String, Vec<u8>>, std::io::Error> {
    let mut files = HashMap::new();

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let mut relative_path = path
                .strip_prefix(initial_path)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();

            if relative_path == "disk.img" {
                continue;
            }

            if !relative_path.starts_with('/') {
                relative_path = format!("/{}", relative_path);
            }

            files.insert(relative_path, std::fs::read(&path)?);
        } else if path.is_dir() {
            let relative_subdir = path.strip_prefix(initial_path).unwrap().to_str().unwrap();
            if ignore_subdirs.contains(&relative_subdir.to_string()) {
                continue;
            }

            let subdir_files = list_files(initial_path, &path, ignore_subdirs)?;
            files.extend(subdir_files);
        }
    }

    Ok(files)
}

fn human_readable(size: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit = 0;

    while size >= 1024.0 {
        size /= 1024.0;
        unit += 1;
    }

    format!("{:.2} {}", size, units[unit])
}

fn main() {
    let files = list_files(
        std::path::Path::new("../disk"),
        std::path::Path::new("../disk"),
        &["target".to_string(), ".git".to_string()],
    )
    .unwrap();

    fs::init();
    let virtfs = fs::get_fs_mut(0,0).unwrap();
    virtfs.phys_fs.write_to_disk(0, 0).unwrap();

    for (file_name, contents) in files.clone() {
        println!("Handled file: {}, wrote {}", file_name, human_readable(contents.len() as u64));
        virtfs.phys_fs.create_file(file_name.as_str(), [7,7,7], 0).unwrap();
        virtfs.phys_fs.write_file(file_name.as_str(), &contents, Some([7,7,7]), Some(0)).unwrap();
    }

    virtfs.phys_fs.write_to_disk(0, 0).unwrap();

    let file_size = std::fs::metadata("disk.img").unwrap().len();

    println!("Handled {} files, total image size {} bytes", files.len(), human_readable(file_size));
}
