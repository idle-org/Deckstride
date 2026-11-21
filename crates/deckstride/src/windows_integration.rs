//! Per-user Windows shell registration for the portable executable.

#[cfg(any(target_os = "windows", test))]
use std::path::Path;

#[cfg(any(target_os = "windows", test))]
fn open_command(executable: &Path) -> String {
    format!(r#"\"{}\" --open \"%1\""#, executable.display())
}

#[cfg(target_os = "windows")]
pub fn register_pdf_association() -> Result<(), String> {
    use std::{ffi::c_void, os::windows::ffi::OsStrExt, ptr};

    type Hkey = *mut c_void;
    const HKEY_CURRENT_USER: Hkey = 0x8000_0001usize as Hkey;
    const KEY_SET_VALUE: u32 = 0x0002;
    const REG_SZ: u32 = 1;
    const SHCNE_ASSOCCHANGED: i32 = 0x0800_0000;
    const SHCNF_IDLIST: u32 = 0;

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegCreateKeyExW(
            root: Hkey,
            subkey: *const u16,
            reserved: u32,
            class: *mut u16,
            options: u32,
            access: u32,
            security: *const c_void,
            result: *mut Hkey,
            disposition: *mut u32,
        ) -> i32;
        fn RegSetValueExW(
            key: Hkey,
            name: *const u16,
            reserved: u32,
            kind: u32,
            data: *const u8,
            bytes: u32,
        ) -> i32;
        fn RegCloseKey(key: Hkey) -> i32;
    }
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn SHChangeNotify(event: i32, flags: u32, item1: *const c_void, item2: *const c_void);
        fn ShellExecuteW(
            window: *mut c_void,
            operation: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show: i32,
        ) -> isize;
    }

    fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
        value.encode_wide().chain(Some(0)).collect()
    }
    fn set_value(root: Hkey, subkey: &str, name: Option<&str>, value: &str) -> Result<(), String> {
        let subkey = wide(std::ffi::OsStr::new(subkey));
        let name = name.map(|name| wide(std::ffi::OsStr::new(name)));
        let value = wide(std::ffi::OsStr::new(value));
        let mut key = ptr::null_mut();
        // SAFETY: all pointers refer to live, NUL-terminated buffers. The returned
        // handle is closed before this function returns.
        let created = unsafe {
            RegCreateKeyExW(
                root,
                subkey.as_ptr(),
                0,
                ptr::null_mut(),
                0,
                KEY_SET_VALUE,
                ptr::null(),
                &mut key,
                ptr::null_mut(),
            )
        };
        if created != 0 {
            return Err(format!("Windows registry error {created}"));
        }
        // SAFETY: key is valid and value/name remain live for the duration of the call.
        let written = unsafe {
            RegSetValueExW(
                key,
                name.as_ref().map_or(ptr::null(), |name| name.as_ptr()),
                0,
                REG_SZ,
                value.as_ptr().cast(),
                (value.len() * 2) as u32,
            )
        };
        // SAFETY: key was returned by RegCreateKeyExW and is no longer used.
        unsafe { RegCloseKey(key) };
        (written == 0)
            .then_some(())
            .ok_or_else(|| format!("Windows registry error {written}"))
    }

    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let executable_text = executable.to_string_lossy();
    let icon = format!(r#"\"{executable_text}\",0"#);
    let command = open_command(&executable);
    let values = [
        (
            r"Software\Classes\Deckstride.Document",
            None,
            "Deckstride document",
        ),
        (
            r"Software\Classes\Deckstride.Document\DefaultIcon",
            None,
            icon.as_str(),
        ),
        (
            r"Software\Classes\Deckstride.Document\shell\open\command",
            None,
            command.as_str(),
        ),
        (
            r"Software\Deckstride\Capabilities",
            Some("ApplicationName"),
            "Deckstride",
        ),
        (
            r"Software\Deckstride\Capabilities",
            Some("ApplicationDescription"),
            "Fast multi-screen PDF presenter",
        ),
        (
            r"Software\Deckstride\Capabilities\FileAssociations",
            Some(".pdf"),
            "Deckstride.Document",
        ),
        (
            r"Software\RegisteredApplications",
            Some("Deckstride"),
            r"Software\Deckstride\Capabilities",
        ),
    ];
    for (subkey, name, value) in values {
        set_value(HKEY_CURRENT_USER, subkey, name, value)?;
    }
    // SAFETY: association refresh carries no payload for SHCNE_ASSOCCHANGED.
    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, ptr::null(), ptr::null()) };

    let operation = wide(std::ffi::OsStr::new("open"));
    let settings = wide(std::ffi::OsStr::new("ms-settings:defaultapps"));
    // SAFETY: strings are NUL terminated and remain live through the call.
    let opened = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            operation.as_ptr(),
            settings.as_ptr(),
            ptr::null(),
            ptr::null(),
            1,
        )
    };
    if opened <= 32 {
        return Err(format!(
            "Registered successfully, but Windows Settings could not open ({opened})"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn association_command_quotes_executable_and_document() {
        assert_eq!(
            open_command(Path::new(r"C:\Portable Apps\deckstride.exe")),
            r#"\"C:\Portable Apps\deckstride.exe\" --open \"%1\""#
        );
    }
}
