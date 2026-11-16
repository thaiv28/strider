use super::*;
use sha2::{Digest, Sha256};
pub const PERMIT_MAX_BYTES: usize = 15 * 1024 * 1024;
pub fn blob_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) fn valid_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(crate) fn save_blob(data_dir: &Path, hash: &str, bytes: &[u8]) -> Result<PathBuf> {
    if !valid_hash(hash) || blob_hash(bytes) != hash {
        return invalid("Permit hash mismatch");
    }
    let dir = data_dir.join("blobs");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(hash);
    let temp = dir.join(format!(".{}", id()));
    std::fs::write(&temp, bytes)?;
    if let Err(e) = std::fs::rename(&temp, &path) {
        let _ = std::fs::remove_file(&temp);
        return Err(e.into());
    }
    Ok(path)
}
#[derive(Clone, Debug)]
pub struct PermitBlob {
    pub sync_id: String,
    pub trip_id: String,
    pub filename: String,
    pub uploaded_at: String,
    pub mime_type: String,
    pub size_bytes: usize,
    pub sha256: String,
    pub state: String,
}
impl Store {
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
    pub fn blob_bytes(&self, hash: &str) -> Result<Option<Vec<u8>>> {
        if !valid_hash(hash) {
            return invalid("Invalid blob hash");
        }
        match std::fs::read(self.data_dir.join("blobs").join(hash)) {
            Ok(bytes) if blob_hash(&bytes) == hash => Ok(Some(bytes)),
            Ok(_) => Ok(None),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    pub fn permit_blobs(&self) -> Result<Vec<PermitBlob>> {
        rows(&self.connection, "trip_permit")?
            .into_iter()
            .map(|r| {
                let hash = s(&r, "sha256").to_string();
                let queued: bool = self.connection.query_row(
                    "SELECT EXISTS(SELECT 1 FROM permit_upload WHERE trip_id=? AND sha256=?)",
                    params![s(&r, "trip_id"), hash],
                    |r| r.get(0),
                )?;
                let state = if !valid_hash(&hash) || self.blob_bytes(&hash)?.is_none() {
                    "missing".into()
                } else if queued {
                    "local_only".into()
                } else {
                    "uploaded".into()
                };
                Ok(PermitBlob {
                    sync_id: s(&r, "sync_id").into(),
                    trip_id: s(&r, "trip_id").into(),
                    filename: s(&r, "filename").into(),
                    uploaded_at: s(&r, "uploaded_at").into(),
                    mime_type: s(&r, "mime_type").into(),
                    size_bytes: n(&r, "size_bytes") as usize,
                    sha256: hash,
                    state,
                })
            })
            .collect()
    }
    pub fn set_blob_state(&self, hash: &str, state: &str) -> Result<()> {
        if !valid_hash(hash) {
            return invalid("Invalid blob hash");
        }
        self.connection.execute("INSERT INTO blob_ref(sha256,state,path) VALUES(?,?,?) ON CONFLICT(sha256) DO UPDATE SET state=excluded.state,path=excluded.path",params![hash,state,self.data_dir.join("blobs").join(hash).to_string_lossy()])?;
        Ok(())
    }
    pub fn acknowledge_permit_upload(&self, trip: &str, hash: &str) -> Result<()> {
        self.connection.execute(
            "DELETE FROM permit_upload WHERE trip_id=? AND sha256=?",
            params![trip, hash],
        )?;
        self.set_blob_state(hash, "uploaded")
    }
    pub fn receive_blob(&self, hash: &str, bytes: &[u8]) -> Result<()> {
        if bytes.len() > PERMIT_MAX_BYTES {
            return invalid("File exceeds 15 MB.");
        }
        save_blob(&self.data_dir, hash, bytes)?;
        self.set_blob_state(hash, "uploaded")
    }
}
