use std::{
    ffi::OsStr,
    fs::rename,
    io::{Error, ErrorKind},
    path::Path,
};

use anyhow::Result;

pub fn merge<T>(v1: &mut Option<Vec<T>>, v2: &Option<Vec<T>>) -> Option<Vec<T>>
where
    T: Clone,
{
    let mut result = v1.clone().map(|mut vec| {
        if let Some(ref other) = v2 {
            vec.extend(other.iter().cloned());
        }
        vec
    });

    if result.is_none() {
        result.clone_from(v2);
    }

    result
}

// rename src to dst, both relative to the directory dir. If dst already exists
// refuse renaming with an error unless overwrite is explicitly asked for.
pub fn rename_in<P: AsRef<Path>, Q: AsRef<Path>>(
    dir: P,
    src: Q,
    dst: Q,
    overwrite: bool,
) -> Result<()> {
    let src_path = dir.as_ref().join(src);
    let dst_path = dir.as_ref().join(dst);

    if !overwrite && dst_path.exists() {
        return Err(Error::new(ErrorKind::AlreadyExists, "destination already exists").into());
    }

    rename(src_path, &dst_path)?;

    Ok(())
}

pub fn is_cdi_spec(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .map(|ext| ext.eq_ignore_ascii_case("json") || ext.eq_ignore_ascii_case("yaml"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn test_merge_none_none() {
        let mut v1: Option<Vec<i32>> = None;
        let v2: Option<Vec<i32>> = None;

        let result = merge(&mut v1, &v2);
        assert!(result.is_none());
    }

    #[test]
    fn test_merge_some_none() {
        let mut v1 = Some(vec![1, 2, 3]);
        let v2: Option<Vec<i32>> = None;

        let result = merge(&mut v1, &v2);
        assert_eq!(result, Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_merge_none_some() {
        let mut v1: Option<Vec<i32>> = None;
        let v2 = Some(vec![4, 5, 6]);

        let result = merge(&mut v1, &v2);
        assert_eq!(result, Some(vec![4, 5, 6]));
    }

    #[test]
    fn test_merge_some_some() {
        let mut v1 = Some(vec![1, 2, 3]);
        let v2 = Some(vec![4, 5, 6]);

        let result = merge(&mut v1, &v2);
        assert_eq!(result, Some(vec![1, 2, 3, 4, 5, 6]));
    }

    #[test]
    fn test_rename_in_success() {
        let tmp_dir = TempDir::new().unwrap();
        let dir_path = tmp_dir.path().to_path_buf();
        let mut src_file = File::create(dir_path.join("src.txt")).unwrap();
        let dst_file = dir_path.join("dst.txt");

        let _ = src_file.write_all(b"Hello, CDI-rs!");

        rename_in(&dir_path, "src.txt", "dst.txt", false).unwrap();
        assert!(dst_file.exists());
        assert!(!Path::new(&dir_path).join("src.txt").exists());
    }

    #[test]
    fn test_rename_in_overwrite() {
        let tmp_dir = TempDir::new().unwrap();
        let dir_path = tmp_dir.path().to_path_buf();

        let mut src_file = File::create(dir_path.join("src.txt")).unwrap();
        let mut dst_file = File::create(dir_path.join("dst.txt")).unwrap();

        let _ = src_file.write_all(b"Hello, CDI-rs!");
        let _ = dst_file.write_all(b"Goodbye, CDI-rs!");

        rename_in(&dir_path, "src.txt", "dst.txt", true).unwrap();
        assert_eq!(
            fs::read_to_string(dir_path.join("dst.txt")).unwrap(),
            "Hello, CDI-rs!"
        );
        assert!(!Path::new(&dir_path).join("src.txt").exists());
    }

    #[test]
    fn test_rename_in_no_overwrite() {
        let tmp_dir = TempDir::new().unwrap();
        let dir_path = tmp_dir.path().to_path_buf();

        let mut src_file = File::create(dir_path.join("src.txt")).unwrap();
        let mut dst_file = File::create(dir_path.join("dst.txt")).unwrap();

        let _ = src_file.write_all(b"Hello, CDI-rs!");
        let _ = dst_file.write_all(b"Goodbye, CDI-rs!");

        let result = rename_in(&dir_path, "src.txt", "dst.txt", false);
        assert!(result.is_err());
        assert!(Path::new(&dir_path).join("src.txt").exists());
        assert!(Path::new(&dir_path).join("dst.txt").exists());
    }
}
