use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

// Each request reads from the same file generation and stops at its initial length.
// The saved chat is atomically replaced, while the event log grows by appending.
pub(super) fn chunk(
    directory: &Path,
    events: bool,
    offset: u64,
    snapshot: Option<(u64, u64)>,
) -> io::Result<Value> {
    let name = if events {
        "openai-events.jsonl"
    } else {
        "conversation.json"
    };
    let mut file = match File::open(directory.join(name)) {
        Err(error) if events && error.kind() == io::ErrorKind::NotFound => {
            return Ok(json!({"bytes":"", "snapshot":[0,0]}));
        }
        result => result?,
    };
    let metadata = file.metadata()?;
    let (inode, length) = snapshot.unwrap_or((metadata.ino(), metadata.len()));
    if inode != metadata.ino() || metadata.len() < length || offset > length {
        return Err(io::Error::other(
            "Conversation changed while loading. Refresh to try again.",
        ));
    }
    file.seek(SeekFrom::Start(offset))?;
    let count = (length - offset).min(512 * 1024);
    let mut bytes = vec![0; count as usize];
    file.read_exact(&mut bytes)?;
    Ok(json!({"bytes": STANDARD.encode(bytes), "snapshot":[inode, length]}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn snapshot_stops_at_original_length_and_rejects_a_replaced_file() {
        let directory = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        fs::create_dir(&directory).unwrap();
        let file = directory.join("openai-events.jsonl");
        let empty = chunk(&directory, true, 0, None).unwrap();
        assert_eq!(empty["snapshot"], json!([0, 0]));

        let original_size = 512 * 1024 + 7;
        fs::write(&file, vec![b'x'; original_size]).unwrap();
        let first = chunk(&directory, true, 0, None).unwrap();
        let version = serde_json::from_value(first["snapshot"].clone()).unwrap();
        let offset = STANDARD
            .decode(first["bytes"].as_str().unwrap())
            .unwrap()
            .len() as u64;

        use std::io::Write;
        fs::OpenOptions::new()
            .append(true)
            .open(&file)
            .unwrap()
            .write_all(b"more")
            .unwrap();
        let last = chunk(&directory, true, offset, version).unwrap();
        assert_eq!(last["bytes"], STANDARD.encode(b"xxxxxxx"));
        fs::rename(&file, directory.join("old")).unwrap();
        fs::write(&file, b"replacement").unwrap();
        assert!(chunk(&directory, true, offset, version).is_err());
        fs::remove_dir_all(directory).unwrap();
    }
}
