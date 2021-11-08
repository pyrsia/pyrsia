use anyhow::{Context, Result, anyhow};
use std::io;
use std::fs;

/// The types of hash that the artifact manager supports
pub enum Hash {
    SHA256([u8; 32]),
    BLAKE3([u8; 64])
}

impl Hash {
    // The base file path (no extension on the file name) that will correspond to this hash
    fn base_file_path<'a>(&self) -> String {
        match self {
            Hash::SHA256(bytes) => "sha256/".to_owned() + &base64::encode(bytes),
            Hash::BLAKE3(bytes) => "blake3/".to_owned() + &base64::encode(bytes),
        }
    }
}

///
/// # Artifact Manager
/// Library for managing artifacts. It manages a local collection of artifacts and is responsible
/// getting artifacts from other nodes when they are not present locally.
///
///
/// Create an ArtifactManager object by passing a path for the local artifact repository to `new` \
/// like this.
/// `ArtifactManager::new("/var/lib/pyrsia")`
///
/// The Artifact manager will store artifacts under the repository directory. The root directory of
/// the repository will contain directories whose names will be hash algorithms (i.e. `SHA256`,
/// `BLAKE3`, …).
///
/// Each of the hash algorithm directories will contain files whose names consist of the file's
/// hash followed by an extension. For example<br>
/// `68efadf3184f20557aa2bbf4432386eb79836902a1e5aea1ff077e323e6ccbb4.file`
///
/// For now, all files will have the `.file` extension to signify that they are simple files whose
/// contents are the artifact having the same hash as indicated by the file name. Other extensions
/// may be used in the future to indicate that the file has a particular internal structure.
pub struct ArtifactManager {
    pub repository_path: &'static str,
}

impl ArtifactManager {
    /// Create a new ArtifactManager that works with artifacts in the given directory
    pub fn new(repository_path: &'static str) -> Result<ArtifactManager, anyhow::Error> {
        if is_accessible_directory(repository_path) {
            Ok(ArtifactManager { repository_path })
        } else {
            Err(anyhow!("Not an accessible directory: {}", repository_path))
        }
    }

    /// Push an artifact to this node's local repository.
    /// Parameters are:
    /// * reader — An object that this method will use to read the bytes of the artifact being
    ///            pushed.
    /// * expected_hash — The hash value that the pushed artifact is expected to have.
    /// Returns a result that contains the the hash
    pub fn push_artifact<'a>(&self, reader: &dyn io::Read, expected_hash: &'a Hash) -> Result<&'a Hash, anyhow::Error> {
        let base_path = expected_hash.base_file_path();
        unimplemented!();
        //Ok(expected_hash)
    }

    /// Pull an artifact. The current implementation only looks in the local node's repository.
    /// A future
    ///
    pub fn pull_artifact(&self, hash_algorithm: &str, hash: & Hash) -> Result<&dyn io::Read, anyhow::Error> {
        unimplemented!();
    }
}

// return true if the given repository path leads to an accessible directory.
fn is_accessible_directory(repository_path: &'static str) -> bool {
    match fs::metadata(repository_path) {
        Err(_) => false,
        Ok(metadata) => metadata.is_dir()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_artifact_manager_with_valid_directory() {
        let ok:bool = match ArtifactManager::new(".") {
            Ok(_) => true,
            Err(_) => false
        };
        assert!(ok)
    }

    #[test]
    fn new_artifact_manager_with_bad_directory() {
        let ok :bool = match ArtifactManager::new("BoGuS") {
            Ok(_) => false,
            Err(_) => true
        };
        assert!(ok)
    }
}
