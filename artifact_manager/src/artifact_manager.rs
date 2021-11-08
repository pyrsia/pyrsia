use std::io;
use std::fs;

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
    //
    // Create a new ArtifactManager that works with artifacts in the given directory
    //
    pub fn new(repository_path: &'static str) -> Result<ArtifactManager, &'static str> {
        if is_accessible_directory(repository_path) {
            Ok(ArtifactManager { repository_path })
        } else {
            Err("Not an accessible directory")
        }
    }

    pub fn push_artifact(reader: &dyn io::Read, hash_algorithm: &str, expected_hash: &[u8]) {

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
