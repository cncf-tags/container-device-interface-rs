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
    let metadata = std::fs::metadata(path)?;
    let file_type = metadata.file_type();

    let (dev_type, major, minor) = if file_type.is_block_device() {
        (
            DeviceType::Block.to_string(),
            libc::major(metadata.rdev()),
            libc::minor(metadata.rdev()),
        )
    } else if file_type.is_char_device() {
        (
            DeviceType::Char.to_string(),
            libc::major(metadata.rdev()),
            libc::minor(metadata.rdev()),
        )
    } else if file_type.is_fifo() {
        (DeviceType::Fifo.to_string(), 0, 0)
    } else {
        return Err(Error::new(ErrorKind::InvalidInput, "It's not a device node").into());
    };

    Ok((dev_type, major.into(), minor.into()))
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
        assert_eq!(dev_node.node.major, Some(2));
        assert_eq!(dev_node.node.minor, Some(2));
    }

    #[test]
    fn test_fill_missing_info_block_device_large_major_minor() {
        if !is_root() {
            println!("INFO: skipping, needs root");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let block_device_path = temp_dir
            .path()
            .join("block_device_large")
            .display()
            .to_string();

        let dev = libc::makedev(259, 513) as u64;
        let res = create_device(&block_device_path, 0o666, dev, "b");
        assert!(res.is_ok(), "Failed to create block device: {:?}", res);

        let mut dev_node = DeviceNode {
            node: CDIDeviceNode {
                path: block_device_path,
                ..Default::default()
            },
        };

        assert!(dev_node.fill_missing_info().is_ok());

        assert_eq!(dev_node.node.r#type, Some(DeviceType::Block.to_string()));
        assert_eq!(dev_node.node.major, Some(259));
        assert_eq!(dev_node.node.minor, Some(513));
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

    // No root needed: /dev/null exists everywhere as char 1:3.
    #[test]
    #[cfg_attr(
        miri,
        ignore = "miri's stat shim does not populate rdev; /dev/null major/minor read as 0"
    )]
    fn device_info_from_char_device_without_root() {
        let (typ, major, minor) = device_info_from_path("/dev/null").unwrap();
        assert_eq!(typ, "c");
        assert_eq!((major, minor), (1, 3));
    }

    // No root needed: mkfifo is an unprivileged syscall.
    #[test]
    #[cfg_attr(miri, ignore = "mkfifo is FFI miri cannot emulate")]
    fn device_info_from_fifo_without_root() {
        let temp_dir = TempDir::new().unwrap();
        let fifo = temp_dir.path().join("fifo");
        let path_c = CString::new(fifo.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(path_c.as_ptr(), 0o600) }, 0);

        let (typ, major, minor) = device_info_from_path(&fifo).unwrap();
        assert_eq!(typ, "p");
        assert_eq!((major, minor), (0, 0));
    }

    #[test]
    fn device_info_rejects_regular_files() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("plain");
        std::fs::File::create(&file).unwrap();
        let err = device_info_from_path(&file).unwrap_err();
        assert!(err.to_string().contains("not a device node"));
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

    #[test]
    fn create_device_reports_mknod_failures() {
        if !is_root() {
            println!("INFO: skipping, needs root");
            return;
        }
        // mknod in a nonexistent directory fails: the error branch of the
        // helper every root test depends on.
        assert!(create_device("/nonexistent-dir/dev", 0o666, 0x0101, "b").is_err());
    }
}
