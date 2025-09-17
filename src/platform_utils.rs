//! 跨平台文件操作工具
//!
//! 这个模块提供了一些工具函数，用于处理跨平台的文件操作问题，
//! 特别是Windows和Unix系统之间的差异。

use std::path::{Path, PathBuf};
use std::fs;
use std::io::{self, Read, Seek};

#[cfg(unix)]
use std::os::unix::fs::FileExt;

#[cfg(windows)]
use std::os::windows::fs::FileExt;

/// 跨平台的目录清理函数
///
/// 安全地删除目录及其所有内容。
pub fn cleanup_db_directory(path: &Path) -> bool {
    if path.exists() {
        match fs::remove_dir_all(path) {
            Ok(_) => true,
            Err(e) => {
                eprintln!("警告: 无法清理目录 {:?}: {}", path, e);
                false
            }
        }
    } else {
        true
    }
}

/// 跨平台的目录准备函数
///
/// 确保目录不存在，然后重新创建它。
pub fn prepare_directory(path: &Path) -> bool {
    // 先清理已存在的目录
    if !cleanup_db_directory(path) {
        return false;
    }

    // 创建新目录
    match fs::create_dir_all(path) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("错误: 无法创建目录 {:?}: {}", path, e);
            false
        }
    }
}

/// 检查路径是否可写
///
/// 通过尝试创建测试文件来检查路径的写入权限。
pub fn is_path_writable(path: &Path) -> bool {
    if !path.exists() {
        if let Err(_) = fs::create_dir_all(path) {
            return false;
        }
    }

    let test_file = path.join(".permission_test");
    match fs::File::create(&test_file) {
        Ok(file) => {
            drop(file); // 确保文件被关闭
            let _ = fs::remove_file(test_file);
            true
        }
        Err(_) => false
    }
}

/// 跨平台的目录同步函数
///
/// 在Unix系统上同步目录，在Windows系统上跳过。
pub fn sync_directory(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        fs::File::open(path)?.sync_all()?;
    }

    #[cfg(windows)]
    {
        // Windows不支持对目录进行文件同步操作，直接跳过
    }

    Ok(())
}

/// 跨平台的read_exact_at实现
///
/// 提供跨平台的文件定位读取功能，针对不同平台进行优化。
pub fn read_exact_at(file: &fs::File, mut buf: &mut [u8], offset: u64) -> std::io::Result<()> {
    // Unix系统：使用原生的pread方法，效率更高
    #[cfg(unix)]
    {
        file.read_exact_at(buf, offset)
    }

    // Windows系统：使用Windows专用的SeekRead方法
    #[cfg(windows)]
    {
        let bytes_read = file.seek_read(buf, offset)?;
        if bytes_read != buf.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "failed to read whole buffer",
            ));
        }
        Ok(())
    }

    // 其他平台：使用通用的seek+read方法作为后备方案
    #[cfg(not(any(unix, windows)))]
    {
        let mut file_clone = file.try_clone()?;
        file_clone.seek(io::SeekFrom::Start(offset))?;
        file_clone.read_exact(buf)
    }
}

/// 为示例程序准备数据库
///
/// 自动清理并创建示例数据库目录。
pub fn setup_example_db(example_name: &str) -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros();
    let counter = COUNTER.fetch_add(1, Ordering::SeqCst);

    let db_path = PathBuf::from(format!("{}_{}_{}_db", example_name, timestamp, counter));

    if !prepare_directory(&db_path) {
        panic!("无法准备示例数据库目录: {:?}", db_path);
    }

    db_path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_db_directory() {
        let path = PathBuf::from("test_cleanup");
        fs::create_dir_all(&path).unwrap();

        assert!(cleanup_db_directory(&path));
        assert!(!path.exists());
    }

    #[test]
    fn test_prepare_directory() {
        let path = PathBuf::from("test_prepare");
        assert!(prepare_directory(&path));
        assert!(path.exists());
    }

    #[test]
    fn test_is_path_writable() {
        let path = PathBuf::from("test_writable");
        assert!(is_path_writable(&path));
        cleanup_db_directory(&path);
    }

    #[test]
    fn test_setup_example_db() {
        let path = setup_example_db("test_setup");
        assert!(path.exists());
        assert!(path.to_str().unwrap().starts_with("test_setup_"));
        assert!(path.to_str().unwrap().ends_with("_db"));

        // 清理
        cleanup_db_directory(&path);
    }
}