//! Canonical JSON rendering and atomic context output.

use std::{
    fs,
    io::{self, Write},
    path::Path,
};

use tempfile::NamedTempFile;

use crate::{error::AgyError, response::Document as ResponseDocument};

pub(crate) fn emit(response: &ResponseDocument, output: Option<&Path>) -> Result<(), AgyError> {
    let document = render(response)?;
    if let Some(path) = output {
        write_atomic(path, document.as_bytes())?;
        let mut stderr = io::stderr().lock();
        writeln!(stderr, "wrote {}", path.display()).map_err(|_| AgyError::OutputWrite)
    } else {
        let mut stdout = io::stdout().lock();
        stdout
            .write_all(document.as_bytes())
            .and_then(|()| stdout.write_all(b"\n"))
            .map_err(|_| AgyError::OutputWrite)
    }
}

fn render(response: &ResponseDocument) -> Result<String, AgyError> {
    let value = serde_json::to_value(response).map_err(|_| AgyError::OutputInvalid)?;
    serde_json::to_string_pretty(&value).map_err(|_| AgyError::OutputInvalid)
}

fn write_atomic(path: &Path, document: &[u8]) -> Result<(), AgyError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|_| AgyError::OutputWrite)?;
    let mut temporary = NamedTempFile::new_in(parent).map_err(|_| AgyError::OutputWrite)?;
    temporary
        .write_all(document)
        .and_then(|()| temporary.write_all(b"\n"))
        .and_then(|()| temporary.flush())
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|_| AgyError::OutputWrite)?;
    temporary.persist(path).map_err(|_| AgyError::OutputWrite)?;
    Ok(())
}
