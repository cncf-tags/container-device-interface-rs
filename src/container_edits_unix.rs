use std::{
    fmt,
    io::{Error, ErrorKind},
    os::unix::fs::{FileTypeExt, MetadataExt},
    path::Path,
};

use anyhow::Result;

pub enum DeviceType {
    Block,
    Char,
    Fifo,
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DeviceType::Block => "b",
            DeviceType::Char => "c",
            DeviceType::Fifo => "p",
        };
        write!(f, "{}", s)
    }
}

// deviceInfoFromPath takes the path to a device and returns its type, major and minor device numbers.
// It was adapted from https://github.com/opencontainers/runc/blob/v1.1.9/libcontainer/devices/device_unix.go#L30-L69
pub fn device_info_from_path<P: AsRef<Path>>(path: P) -> Result<(String, i64, i64)> {
    let major = |dev: u64| -> i64 { (dev >> 8) as i64 & 0xff };
    let minor = |dev: u64| -> i64 { dev as i64 & 0xff };

    let metadata = std::fs::metadata(path)?;
    let file_type = metadata.file_type();

    let (dev_type, major, minor) = if file_type.is_block_device() {
        (
            DeviceType::Block.to_string(),
            major(metadata.rdev()),
            minor(metadata.rdev()),
        )
    } else if file_type.is_char_device() {
        (
            DeviceType::Char.to_string(),
            major(metadata.rdev()),
            minor(metadata.rdev()),
        )
    } else if file_type.is_fifo() {
        (DeviceType::Fifo.to_string(), 0, 0)
    } else {
        return Err(Error::new(ErrorKind::InvalidInput, "It's not a device node").into());
    };

    Ok((dev_type, major, minor))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::ffi::CString;

    use anyhow::Result;
    use nix::libc::{self, dev_t, mknodat, mode_t, S_IFBLK, S_IFCHR, S_IFIFO};
    use tempfile::TempDir;

    use crate::{container_edits::DeviceNode, specs::config::DeviceNode as CDIDeviceNode};

    fn is_root() -> bool {
        unsafe { libc::geteuid() == 0 }
    }

    fn create_device(path: &str, mode: mode_t, dev: u64, dev_type: &str) -> Result<()> {
        let path_c = CString::new(path)?;

        // Set the appropriate mode for block or char or fifo device
        let mode = match dev_type {
            "b" => mode | S_IFBLK,
            "c" => mode | S_IFCHR,
            "p" => mode | S_IFIFO,
            _ => 0,
        };

        // Create the device
        let res = unsafe { mknodat(libc::AT_FDCWD, path_c.as_ptr(), mode, dev as dev_t) };
        if res < 0 {
            println!("create device with path: {:?} failed", path);
            return Err(nix::Error::last().into());
        }

        println!("create device with path: {:?} successfully", path);
        Ok(())
    }

    #[test]
    fn test_fill_missing_info_block_device() {
        if !is_root() {
            println!("INFO: skipping, needs root");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let block_device_path = temp_dir.path().join("block_device").display().to_string();

        // Create a block device
        let res = create_device(&block_device_path, 0o666, 0x0101, "b");
        assert!(res.is_ok(), "Failed to create block device: {:?}", res);

        let mut dev_node = DeviceNode {
            node: CDIDeviceNode {
                path: block_device_path,
                ..Default::default()
            },
        };

        assert!(dev_node.fill_missing_info().is_ok());

        assert_eq!(dev_node.node.r#type, Some(DeviceType::Block.to_string()));
        assert_eq!(dev_node.node.major, Some(1));
        assert_eq!(dev_node.node.minor, Some(1));
    }

    #[test]
    fn test_fill_missing_info_char_device() {
        if !is_root() {
            println!("INFO: skipping, needs root");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let char_device_path = temp_dir.path().join("char_device").display().to_string();

        // Create a character device
        let res = create_device(&char_device_path, 0o666, 0x0202, "c");
        assert!(res.is_ok(), "Failed to create char device: {:?}", res);

        let mut dev_node = DeviceNode {
            node: CDIDeviceNode {
                path: char_device_path,
                ..Default::default()
            },
        };

        assert!(dev_node.fill_missing_info().is_ok());

        assert_eq!(dev_node.node.r#type, Some(DeviceType::Char.to_string()));
        assert_eq!(dev_node.node.major, Some(1));
        assert_eq!(dev_node.node.minor, Some(2));
    }

    #[test]
    fn test_fill_missing_info_fifo() {
        if !is_root() {
            println!("INFO: skipping which needs root");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let fifo_device_path = temp_dir.path().join("fifo_device").display().to_string();

        // Create a character device
        let res = create_device(&fifo_device_path, 0o666, 0x0, "p");
        assert!(res.is_ok(), "Failed to create fifo device: {:?}", res);

        let mut dev_node = DeviceNode {
            node: CDIDeviceNode {
                path: fifo_device_path,
                ..Default::default()
            },
        };

        dev_node.fill_missing_info().unwrap();

        assert_eq!(dev_node.node.r#type, Some(DeviceType::Fifo.to_string()));
        assert_eq!(dev_node.node.major, None);
        assert_eq!(dev_node.node.minor, None);
    }

    #[test]
    fn test_fill_missing_info_regular_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("regular_file");
        std::fs::File::create(&file_path).unwrap();

        let mut dev_node = DeviceNode {
            node: CDIDeviceNode {
                path: file_path.to_string_lossy().to_string(),
                ..Default::default()
            },
        };

        let result = dev_node.fill_missing_info();
        assert!(result.is_err());
    }
}
