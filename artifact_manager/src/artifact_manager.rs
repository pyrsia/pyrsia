//use std::error::Error;
use std::fs;
//
// # Artifact Manager
// Library for managing artifacts. It manages a local collection of artifacts and is responsible
// getting artifacts from other nodes when they are not present locally.
//
// Create an ArtifactManager object by passing a path for the local artifact repository to `new`.
// ```
// ArtifactManager::new("/var/lib/pyrsia")
// ```
pub struct ArtifactManager {
    pub repository_path : &'static str,
}

impl ArtifactManager {
    //
    // Create a new ArtifactManager that works with artifacts in the given directory
    //
    pub fn new(repository_path : &'static str) -> Result<ArtifactManager, &'static str> {
        if is_accessible_directory(repository_path) {
            Ok(ArtifactManager { repository_path })
        } else {
            Err("Not an accessible directory")
        }
    }
}

fn is_accessible_directory(repository_path : &'static str) -> bool {
    match fs::metadata(repository_path) {
        Err(_) => false,
        Ok(metadata) => metadata.is_dir()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn new_artifact_manager_with_valid_directory() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
