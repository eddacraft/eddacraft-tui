//! Handle-relative no-reparse filesystem primitives for Windows.
//!
//! The Windows analogue of the Unix `openat` / `O_NOFOLLOW` helpers used by
//! policy install and other CLI writers: every path component is opened or
//! created relative to a held parent handle with `OBJ_DONT_REPARSE`, so a
//! concurrent symlink or junction swap cannot redirect create / write / remove
//! outside the intended tree.
//!
//! Unsafe FFI stays here so `anvil-cli` (which forbids `unsafe_code`) can call
//! a safe surface. The ladder matches [`crate::read_safety`]: `NtCreateFile`
//! relative to the prior handle, `OBJ_DONT_REPARSE` on each hop.

use std::ffi::OsStr;
use std::io::{self, ErrorKind};
use std::os::windows::ffi::OsStrExt;
use std::path::{Component, Path};
use std::ptr::null_mut;
use std::time::{SystemTime, UNIX_EPOCH};

use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
use windows_sys::Wdk::Storage::FileSystem::{
    FILE_CREATE, FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN,
    FILE_OPEN_FOR_BACKUP_INTENT, FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
    NtCreateFile,
};
use windows_sys::Win32::Foundation::{
    CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE, UNICODE_STRING,
};
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, CreateFileW, DELETE, FILE_ATTRIBUTE_DIRECTORY,
    FILE_ATTRIBUTE_NORMAL, FILE_DISPOSITION_INFO, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_FLAG_OPEN_REPARSE_POINT, FILE_RENAME_INFO, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, FileDispositionInfo, FileRenameInfo, FlushFileBuffers,
    GetFileInformationByHandle, OPEN_EXISTING, SetFileInformationByHandle, WriteFile,
};
use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

const SYNCHRONIZE: u32 = 0x0010_0000;
const OBJ_CASE_INSENSITIVE: u32 = 0x0000_0040;
const OBJ_DONT_REPARSE: u32 = 0x0000_1000;
const FILE_TRAVERSE: u32 = 0x0000_0020;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;

const STATUS_SUCCESS: i32 = 0;
const STATUS_REPARSE_POINT_ENCOUNTERED: i32 = 0xC000_050B_u32 as i32;
const STATUS_OBJECT_NAME_NOT_FOUND: i32 = 0xC000_0034_u32 as i32;
const STATUS_OBJECT_PATH_NOT_FOUND: i32 = 0xC000_003A_u32 as i32;
const STATUS_OBJECT_NAME_COLLISION: i32 = 0xC000_0035_u32 as i32;
const STATUS_ACCESS_DENIED: i32 = 0xC000_0022_u32 as i32;
const STATUS_NOT_A_DIRECTORY: i32 = 0xC000_0103_u32 as i32;

const ERROR_FILE_NOT_FOUND: i32 = 2;
const ERROR_PATH_NOT_FOUND: i32 = 3;
const ERROR_ACCESS_DENIED: i32 = 5;
const ERROR_DIRECTORY: i32 = 267;
const ERROR_ALREADY_EXISTS: i32 = 183;

/// Access used when we need to create, rename, or delete under a directory.
const DIR_ACCESS_FULL: u32 = GENERIC_READ | GENERIC_WRITE | DELETE | FILE_TRAVERSE | SYNCHRONIZE;
/// Fallback walk access when the volume root refuses write (typical for `C:\`).
const DIR_ACCESS_WALK: u32 = GENERIC_READ | FILE_TRAVERSE | SYNCHRONIZE;
const FILE_ACCESS: u32 = GENERIC_READ | GENERIC_WRITE | DELETE | SYNCHRONIZE;

struct OwnedHandle(HANDLE);

// SAFETY: a Win32 HANDLE is a kernel-object reference safe to move between
// threads; the kernel serialises its own access.
unsafe impl Send for OwnedHandle {}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if self.0 != INVALID_HANDLE_VALUE && !self.0.is_null() {
            // SAFETY: owned handle from CreateFileW / NtCreateFile, closed once.
            unsafe { CloseHandle(self.0) };
        }
    }
}

impl OwnedHandle {
    fn raw(&self) -> HANDLE {
        self.0
    }
}

/// Create every missing directory component of `path` without following
/// symlink or junction components.
///
/// # Errors
/// Returns an error whose display mentions `symlink` when a reparse point is
/// encountered, or the underlying I/O error for any other failure.
pub fn create_dir_all_nofollow(path: &Path) -> io::Result<()> {
    if path.as_os_str().is_empty() {
        return Ok(());
    }
    let (root, rest) = split_root_and_rest(path)?;
    let mut dir = open_existing_dir(&root, true)?;
    for name in rest {
        dir = open_or_mkdir_at(dir.raw(), &name)?;
    }
    Ok(())
}

/// Atomically write `data` to `path` without following reparse points.
///
/// Parents are created with [`create_dir_all_nofollow`]. The payload is written
/// to a unique temp leaf under a pinned parent handle and renamed into place
/// via `SetFileInformationByHandle(FileRenameInfo)` so a parent swap after the
/// path check cannot redirect the write.
///
/// # Errors
/// Same reparse-refusal contract as [`create_dir_all_nofollow`].
pub fn atomic_write_nofollow(path: &Path, data: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let leaf = path.file_name().ok_or_else(|| {
        io::Error::new(
            ErrorKind::InvalidInput,
            format!("write path has no file name: {}", path.display()),
        )
    })?;
    create_dir_all_nofollow(parent)?;
    let dir = open_existing_dir(parent, true)?;
    atomic_write_at(dir.raw(), leaf, data)
}

/// Remove a file without following reparse path components.
///
/// A leaf reparse point is unlinked itself (not its target). A swapped
/// ancestor fails closed.
///
/// # Errors
/// Same reparse-refusal contract as [`create_dir_all_nofollow`].
pub fn remove_file_nofollow(path: &Path) -> io::Result<()> {
    remove_at(path, false)
}

/// Remove an empty directory without following reparse path components.
///
/// # Errors
/// Same reparse-refusal contract as [`create_dir_all_nofollow`].
pub fn remove_dir_nofollow(path: &Path) -> io::Result<()> {
    remove_at(path, true)
}

/// Read a regular file without following a final-component reparse point.
///
/// # Errors
/// A leaf or ancestor reparse point is refused. Other I/O errors propagate.
pub fn read_nofollow(path: &Path) -> io::Result<Vec<u8>> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let leaf = path.file_name().ok_or_else(|| {
        io::Error::new(
            ErrorKind::InvalidInput,
            format!("read path has no file name: {}", path.display()),
        )
    })?;
    let dir = open_existing_dir(parent, false)?;
    let file = nt_open_at(dir.raw(), leaf, OpenKind::File, FILE_ACCESS, true)?;
    read_handle(file.raw())
}

fn remove_at(path: &Path, is_dir: bool) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let leaf = path.file_name().ok_or_else(|| {
        io::Error::new(
            ErrorKind::InvalidInput,
            format!("remove path has no file name: {}", path.display()),
        )
    })?;
    let dir = open_existing_dir(parent, true)?;
    let kind = if is_dir {
        OpenKind::Directory
    } else {
        OpenKind::Any
    };
    // Open the leaf as-named (`FILE_OPEN_REPARSE_POINT`) so a leaf symlink or
    // junction is deleted itself, matching Unix `unlinkat` (does not follow).
    let leaf_handle = nt_open_at(dir.raw(), leaf, kind, DELETE | SYNCHRONIZE, false)?;
    dispose_handle(leaf_handle.raw())
}

fn split_root_and_rest(path: &Path) -> io::Result<(std::path::PathBuf, Vec<std::ffi::OsString>)> {
    let mut root = std::path::PathBuf::new();
    let mut rest = Vec::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => root.push(prefix.as_os_str()),
            Component::RootDir => root.push(component.as_os_str()),
            Component::Normal(name) => rest.push(name.to_os_string()),
            Component::CurDir => {
                if root.as_os_str().is_empty() && rest.is_empty() {
                    root.push(".");
                }
            }
            Component::ParentDir => {
                return Err(io::Error::new(
                    ErrorKind::InvalidInput,
                    format!("refusing path with parent-dir component {}", path.display()),
                ));
            }
        }
    }
    if root.as_os_str().is_empty() {
        root.push(".");
    }
    Ok((root, rest))
}

fn open_existing_dir(path: &Path, want_write: bool) -> io::Result<OwnedHandle> {
    let (root, rest) = split_root_and_rest(path)?;
    let mut dir = open_root(&root, want_write)?;
    for name in rest {
        let access = if want_write {
            DIR_ACCESS_FULL
        } else {
            DIR_ACCESS_WALK
        };
        dir = match nt_open_at(dir.raw(), &name, OpenKind::Directory, access, true) {
            Ok(handle) => handle,
            Err(err) if want_write && err.raw_os_error() == Some(ERROR_ACCESS_DENIED) => {
                nt_open_at(dir.raw(), &name, OpenKind::Directory, DIR_ACCESS_WALK, true)?
            }
            Err(err) => return Err(err),
        };
    }
    Ok(dir)
}

fn open_root(root: &Path, want_write: bool) -> io::Result<OwnedHandle> {
    let preferred = if want_write {
        DIR_ACCESS_FULL
    } else {
        DIR_ACCESS_WALK
    };
    match open_root_with_access(root, preferred) {
        Ok(handle) => Ok(handle),
        Err(err) if want_write && err.raw_os_error() == Some(ERROR_ACCESS_DENIED) => {
            open_root_with_access(root, DIR_ACCESS_WALK)
        }
        Err(err) => Err(err),
    }
}

fn open_root_with_access(root: &Path, access: u32) -> io::Result<OwnedHandle> {
    let wide: Vec<u16> = root.as_os_str().encode_wide().chain([0]).collect();
    // SAFETY: `wide` is a NUL-terminated UTF-16 path; the returned handle is
    // owned by `OwnedHandle` and closed in `Drop`.
    let handle = unsafe {
        CreateFileW(
            wide.as_ptr(),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        return Err(io::Error::last_os_error());
    }
    let owned = OwnedHandle(handle);
    if is_reparse(owned.raw())? {
        return Err(reparse_error(root));
    }
    Ok(owned)
}

fn is_reparse(handle: HANDLE) -> io::Result<bool> {
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: `info` is a valid out struct; `handle` is a live file handle.
    let ok = unsafe { GetFileInformationByHandle(handle, &mut info) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0)
}

fn open_or_mkdir_at(parent: HANDLE, name: &OsStr) -> io::Result<OwnedHandle> {
    match nt_open_at(parent, name, OpenKind::Directory, DIR_ACCESS_FULL, true) {
        Ok(handle) => Ok(handle),
        Err(err)
            if err.raw_os_error() == Some(ERROR_FILE_NOT_FOUND)
                || err.raw_os_error() == Some(ERROR_PATH_NOT_FOUND) =>
        {
            match nt_create_dir_at(parent, name) {
                Ok(handle) => Ok(handle),
                Err(err) if err.raw_os_error() == Some(ERROR_ALREADY_EXISTS) => {
                    nt_open_at(parent, name, OpenKind::Directory, DIR_ACCESS_FULL, true)
                }
                Err(err) => Err(err),
            }
        }
        Err(err) if err.raw_os_error() == Some(ERROR_ACCESS_DENIED) => {
            nt_open_at(parent, name, OpenKind::Directory, DIR_ACCESS_WALK, true)
        }
        Err(err) => Err(err),
    }
}

fn nt_create_dir_at(parent: HANDLE, name: &OsStr) -> io::Result<OwnedHandle> {
    nt_create(
        parent,
        name,
        DIR_ACCESS_FULL,
        FILE_CREATE,
        FILE_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_FOR_BACKUP_INTENT,
        FILE_ATTRIBUTE_DIRECTORY,
        true,
    )
}

#[derive(Clone, Copy)]
enum OpenKind {
    Directory,
    File,
    Any,
}

fn nt_open_at(
    parent: HANDLE,
    name: &OsStr,
    kind: OpenKind,
    access: u32,
    refuse_reparse: bool,
) -> io::Result<OwnedHandle> {
    let mut create_options = FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_FOR_BACKUP_INTENT;
    match kind {
        OpenKind::Directory => create_options |= FILE_DIRECTORY_FILE,
        OpenKind::File => create_options |= FILE_NON_DIRECTORY_FILE,
        OpenKind::Any => {}
    }
    if !refuse_reparse {
        create_options |= FILE_OPEN_REPARSE_POINT;
    }
    nt_create(
        parent,
        name,
        access,
        FILE_OPEN,
        create_options,
        FILE_ATTRIBUTE_NORMAL,
        refuse_reparse,
    )
}

fn nt_create(
    parent: HANDLE,
    name: &OsStr,
    access: u32,
    disposition: u32,
    create_options: u32,
    attributes: u32,
    refuse_reparse: bool,
) -> io::Result<OwnedHandle> {
    let mut wide: Vec<u16> = name.encode_wide().collect();
    if wide.is_empty() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "empty path component",
        ));
    }
    let byte_len = u16::try_from(wide.len() * 2)
        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "path component too long"))?;
    let mut unicode = UNICODE_STRING {
        Length: byte_len,
        MaximumLength: byte_len,
        Buffer: wide.as_mut_ptr(),
    };

    // SAFETY: OBJECT_ATTRIBUTES is a plain C struct; `ObjectName` borrows
    // `unicode`, which borrows `wide`; all three outlive NtCreateFile.
    let mut attrs: OBJECT_ATTRIBUTES = unsafe { std::mem::zeroed() };
    attrs.Length = u32::try_from(std::mem::size_of::<OBJECT_ATTRIBUTES>())
        .expect("OBJECT_ATTRIBUTES size fits u32");
    attrs.RootDirectory = parent;
    attrs.ObjectName = &mut unicode;
    attrs.Attributes = OBJ_CASE_INSENSITIVE;
    if refuse_reparse {
        attrs.Attributes |= OBJ_DONT_REPARSE;
    }

    let mut handle: HANDLE = null_mut();
    let mut iosb: IO_STATUS_BLOCK = unsafe { std::mem::zeroed() };
    // SAFETY: `handle` / `iosb` / `attrs` are valid out/in pointers held for
    // the call; `wide` backs `unicode` backs `attrs.ObjectName`.
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            access,
            &attrs,
            &mut iosb,
            null_mut(),
            attributes,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            disposition,
            create_options,
            null_mut(),
            0,
        )
    };
    drop(wide);

    match status {
        STATUS_SUCCESS => {
            if handle == INVALID_HANDLE_VALUE || handle.is_null() {
                return Err(io::Error::other("NtCreateFile returned a null handle"));
            }
            Ok(OwnedHandle(handle))
        }
        STATUS_REPARSE_POINT_ENCOUNTERED => Err(reparse_error(Path::new(name))),
        STATUS_OBJECT_NAME_NOT_FOUND => Err(io::Error::from_raw_os_error(ERROR_FILE_NOT_FOUND)),
        STATUS_OBJECT_PATH_NOT_FOUND => Err(io::Error::from_raw_os_error(ERROR_PATH_NOT_FOUND)),
        STATUS_OBJECT_NAME_COLLISION => Err(io::Error::from_raw_os_error(ERROR_ALREADY_EXISTS)),
        STATUS_ACCESS_DENIED => Err(io::Error::from_raw_os_error(ERROR_ACCESS_DENIED)),
        STATUS_NOT_A_DIRECTORY => Err(io::Error::from_raw_os_error(ERROR_DIRECTORY)),
        other => Err(io::Error::other(format!(
            "NtCreateFile failed: NTSTATUS {other:#010x}"
        ))),
    }
}

fn atomic_write_at(parent: HANDLE, leaf: &OsStr, data: &[u8]) -> io::Result<()> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let mut last_err: Option<io::Error> = None;

    for attempt in 0..32u32 {
        let temp_name = format!(
            ".anvil-write-{}-{}-{attempt}.tmp",
            std::process::id(),
            nanos
        );
        let temp = OsStr::new(&temp_name);
        let file = match nt_create(
            parent,
            temp,
            FILE_ACCESS,
            FILE_CREATE,
            FILE_NON_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT | FILE_OPEN_FOR_BACKUP_INTENT,
            FILE_ATTRIBUTE_NORMAL,
            true,
        ) {
            Ok(file) => file,
            Err(err) if err.raw_os_error() == Some(ERROR_ALREADY_EXISTS) => continue,
            Err(err) => return Err(err),
        };

        if let Err(err) = write_all_handle(file.raw(), data) {
            let _ = dispose_handle(file.raw());
            return Err(err);
        }

        match rename_at(file.raw(), parent, leaf) {
            Ok(()) => return Ok(()),
            Err(err) => {
                let _ = dispose_handle(file.raw());
                last_err = Some(err);
            }
        }
    }

    Err(last_err.unwrap_or_else(|| {
        io::Error::other("could not allocate a unique temp file under the parent directory")
    }))
}

fn write_all_handle(handle: HANDLE, mut data: &[u8]) -> io::Result<()> {
    while !data.is_empty() {
        let chunk = u32::try_from(data.len()).unwrap_or(u32::MAX);
        let mut written: u32 = 0;
        // SAFETY: `handle` is a live write handle; `data` is a valid buffer of
        // `chunk` bytes; `&mut written` is a valid out param; null OVERLAPPED
        // matches the synchronous handle.
        let ok = unsafe { WriteFile(handle, data.as_ptr(), chunk, &mut written, null_mut()) };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        if written == 0 {
            return Err(io::Error::new(
                ErrorKind::WriteZero,
                "WriteFile wrote zero bytes",
            ));
        }
        data = &data[written as usize..];
    }
    // SAFETY: `handle` is a live write handle opened for synchronous I/O.
    let flushed = unsafe { FlushFileBuffers(handle) };
    if flushed == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn read_handle(handle: HANDLE) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let mut read: u32 = 0;
        // SAFETY: `handle` is a live read handle; `chunk` is a valid mutable
        // buffer; `&mut read` is a valid out param; null OVERLAPPED = sync IO.
        let ok = unsafe {
            windows_sys::Win32::Storage::FileSystem::ReadFile(
                handle,
                chunk.as_mut_ptr(),
                chunk.len() as u32,
                &mut read,
                null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        if read == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..read as usize]);
    }
    Ok(buf)
}

fn rename_at(file: HANDLE, parent: HANDLE, leaf: &OsStr) -> io::Result<()> {
    let name: Vec<u16> = leaf.encode_wide().collect();
    if name.is_empty() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "rename destination has no file name",
        ));
    }
    let name_bytes = name.len() * 2;
    let header = std::mem::offset_of!(FILE_RENAME_INFO, FileName);
    let size = header + name_bytes;
    let mut buf = vec![0u8; size.max(std::mem::size_of::<FILE_RENAME_INFO>())];
    // SAFETY: `buf` is large enough for FILE_RENAME_INFO plus the extra
    // wide-name bytes; fields are written before the call; the name copy
    // stays inside `buf`.
    unsafe {
        let info = buf.as_mut_ptr().cast::<FILE_RENAME_INFO>();
        (*info).Anonymous.ReplaceIfExists = true;
        (*info).RootDirectory = parent;
        (*info).FileNameLength = u32::try_from(name_bytes)
            .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "rename name too long"))?;
        std::ptr::copy_nonoverlapping(name.as_ptr(), (*info).FileName.as_mut_ptr(), name.len());
    }
    // SAFETY: `file` is a live handle with DELETE access; `buf` holds a
    // well-formed FILE_RENAME_INFO for `size` bytes.
    let ok = unsafe {
        SetFileInformationByHandle(
            file,
            FileRenameInfo,
            buf.as_ptr().cast(),
            u32::try_from(size)
                .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "rename info too large"))?,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn dispose_handle(handle: HANDLE) -> io::Result<()> {
    let info = FILE_DISPOSITION_INFO { DeleteFile: true };
    // SAFETY: `handle` is a live handle with DELETE access; `info` is a
    // well-formed FILE_DISPOSITION_INFO.
    let ok = unsafe {
        SetFileInformationByHandle(
            handle,
            FileDispositionInfo,
            std::ptr::from_ref(&info).cast(),
            u32::try_from(std::mem::size_of::<FILE_DISPOSITION_INFO>()).expect("disposition fits"),
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn reparse_error(component: &Path) -> io::Error {
    io::Error::other(format!(
        "refusing path through symlink {}: resolve the symlink and re-run",
        component.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn plant_junction(link: &Path, target: &Path) -> bool {
        std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .status()
            .is_ok_and(|status| status.success())
    }

    #[test]
    fn create_dir_all_nofollow_creates_nested() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("a").join("b").join("c");
        create_dir_all_nofollow(&target).expect("create");
        assert!(target.is_dir());
    }

    #[test]
    fn create_dir_all_nofollow_refuses_junction_component() {
        let outside = tempfile::tempdir().expect("outside");
        let root = tempfile::tempdir().expect("root");
        let link = root.path().join("escape");
        assert!(
            plant_junction(&link, outside.path()),
            "mklink /J creates a junction without privilege"
        );
        let target = link.join("child");
        let err = create_dir_all_nofollow(&target).expect_err("junction component");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("symlink"),
            "error should mention symlink: {msg}"
        );
        assert!(
            !outside.path().join("child").exists(),
            "must not create directories through the junction"
        );
    }

    #[test]
    fn atomic_write_nofollow_creates_and_overwrites() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("nested").join("file.txt");
        atomic_write_nofollow(&path, b"one").expect("create");
        assert_eq!(std::fs::read(&path).unwrap(), b"one");
        atomic_write_nofollow(&path, b"two").expect("overwrite");
        assert_eq!(std::fs::read(&path).unwrap(), b"two");
    }

    #[test]
    fn atomic_write_nofollow_refuses_junctioned_parent() {
        let outside = tempfile::tempdir().expect("outside");
        let root = tempfile::tempdir().expect("root");
        let parent = root.path().join("parent");
        assert!(
            plant_junction(&parent, outside.path()),
            "mklink /J creates a junction without privilege"
        );
        let path = parent.join("leaked.txt");
        let err = atomic_write_nofollow(&path, b"secret").expect_err("junctioned parent");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("symlink"),
            "error should mention symlink: {msg}"
        );
        assert!(
            !outside.path().join("leaked.txt").exists(),
            "must not write through the junction"
        );
    }

    #[test]
    fn remove_file_nofollow_removes_real_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("doomed.txt");
        std::fs::write(&path, b"x").unwrap();
        remove_file_nofollow(&path).expect("remove");
        assert!(!path.exists());
    }

    #[test]
    fn remove_file_nofollow_refuses_junctioned_parent() {
        let outside = tempfile::tempdir().expect("outside");
        let marker = outside.path().join("victim.txt");
        std::fs::write(&marker, b"keep").unwrap();
        let root = tempfile::tempdir().expect("root");
        let parent = root.path().join("parent");
        assert!(
            plant_junction(&parent, outside.path()),
            "mklink /J creates a junction without privilege"
        );
        let err = remove_file_nofollow(&parent.join("victim.txt")).expect_err("junctioned parent");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("symlink"),
            "error should mention symlink: {msg}"
        );
        assert_eq!(std::fs::read(&marker).unwrap(), b"keep");
    }

    #[test]
    fn read_nofollow_reads_real_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("data.txt");
        std::fs::write(&path, b"payload").unwrap();
        assert_eq!(read_nofollow(&path).expect("read"), b"payload");
    }

    #[test]
    fn split_root_rejects_parent_dir() {
        let err = split_root_and_rest(Path::new("foo/../bar")).expect_err("parent");
        let msg = format!("{err:#}");
        assert!(msg.contains("parent-dir"), "{msg}");
    }

    #[test]
    fn split_root_keeps_drive_and_rest() {
        let path = PathBuf::from(r"C:\Users\tmp\file");
        let (root, rest) = split_root_and_rest(&path).expect("split");
        assert!(
            root.as_os_str().to_string_lossy().contains(':'),
            "root should keep the drive: {}",
            root.display()
        );
        assert_eq!(
            rest.iter()
                .map(|s| s.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            vec!["Users", "tmp", "file"]
        );
    }
}
