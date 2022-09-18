use std::{env, error, ffi, fmt, fs, io, path};

#[derive(Debug)]
pub struct HomeDirError;

impl fmt::Display for HomeDirError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unable to find home directory")
    }
}

impl error::Error for HomeDirError {}

impl From<HomeDirError> for io::Error {
    fn from(e: HomeDirError) -> Self {
        Self::new(io::ErrorKind::Other, e)
    }
}

fn get_home_dir() -> Result<path::PathBuf, HomeDirError> {
    let home_dir = match home::home_dir() {
        Some(home_dir) => home_dir,
        None => return Err(HomeDirError),
    };
    Ok(home_dir)
}

fn get_config_dir() -> Result<path::PathBuf, HomeDirError> {
    // first try $FEND_CONFIG_DIR
    if let Some(env_var_config_dir) = env::var_os("FEND_CONFIG_DIR") {
        return Ok(path::PathBuf::from(env_var_config_dir));
    }

    // otherwise try $XDG_CONFIG_HOME/fend/
    if let Some(env_var_xdg_config_dir) = env::var_os("XDG_CONFIG_HOME") {
        let mut res = path::PathBuf::from(env_var_xdg_config_dir);
        res.push("fend");
        return Ok(res);
    }

    // otherwise use $HOME/.config/fend/
    let mut res = get_home_dir()?;
    res.push(".config");
    res.push("fend");
    Ok(res)
}

pub fn get_config_file_location() -> Result<path::PathBuf, HomeDirError> {
    let mut config_path = get_config_dir()?;
    config_path.push("config.toml");
    Ok(config_path)
}

fn get_state_dir() -> Result<path::PathBuf, HomeDirError> {
    // first try $FEND_STATE_DIR
    if let Some(env_var_history_dir) = env::var_os("FEND_STATE_DIR") {
        return Ok(path::PathBuf::from(env_var_history_dir));
    }

    // otherwise try $XDG_STATE_HOME/fend/
    if let Some(env_var_xdg_state_dir) = env::var_os("XDG_STATE_HOME") {
        let mut res = path::PathBuf::from(env_var_xdg_state_dir);
        mark_dir_as_hidden(&res);
        res.push("fend");
        return Ok(res);
    }

    // otherwise use $HOME/.local/state/fend/
    let mut res = get_home_dir()?;
    res.push(".local");
    mark_dir_as_hidden(&res);
    res.push("state");
    res.push("fend");
    Ok(res)
}

pub fn create_state_dir() -> io::Result<()> {
    let state_dir = get_state_dir()?;
    fs::create_dir_all(state_dir)?;
    Ok(())
}

pub fn get_history_file_location() -> Result<path::PathBuf, HomeDirError> {
    let mut history_path = get_state_dir()?;
    history_path.push("history");
    Ok(history_path)
}

pub fn get_cache_dir() -> Result<path::PathBuf, HomeDirError> {
    // first try $FEND_CACHE_DIR
    if let Some(env_var_cache_dir) = env::var_os("FEND_CACHE_DIR") {
        return Ok(path::PathBuf::from(env_var_cache_dir));
    }

    // otherwise try $XDG_CACHE_HOME/fend/
    if let Some(env_var_xdg_cache_dir) = env::var_os("XDG_CACHE_HOME") {
        let mut res = path::PathBuf::from(env_var_xdg_cache_dir);
        mark_dir_as_hidden(&res);
        res.push("fend");
        return Ok(res);
    }

    // otherwise use $HOME/.cache/fend/
    let mut res = get_home_dir()?;
    res.push(".cache");
    mark_dir_as_hidden(&res);
    res.push("fend");
    Ok(res)
}

pub fn create_cache_dir() -> io::Result<path::PathBuf> {
    let cache_dir = get_cache_dir()?;
    fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

fn mark_dir_as_hidden(path: &path::Path) {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return,
    };

    if !metadata.is_dir() {
        return;
    }

    let path = {
        let mut p = ffi::OsString::from("\\\\?\\");
        p.push(path.as_os_str());
        p
    };

    match mark_dir_as_hidden_impl(path.as_os_str()) {
        Ok(()) => (),
        Err(e) => {
            eprintln!("error code: {e}");
        }
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn mark_dir_as_hidden_impl(path: &ffi::OsStr) -> Result<(), u32> {
    use std::os::windows::prelude::*;
    use winapi::um::{
        errhandlingapi::GetLastError, fileapi::SetFileAttributesW, winnt::FILE_ATTRIBUTE_HIDDEN,
    };

    let path = path.encode_wide().chain(Some(0)).collect::<Vec<u16>>();

    unsafe {
        // https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfileattributesw
        let return_code = SetFileAttributesW(path.as_slice().as_ptr(), FILE_ATTRIBUTE_HIDDEN);
        if return_code == 0 {
            return Err(GetLastError());
        }
    }
    Ok(())
}

#[cfg(not(windows))]
#[allow(clippy::unnecessary_wraps)]
fn mark_dir_as_hidden_impl(_path: &ffi::OsStr) -> Result<(), u32> {
    Ok(())
}
