use std::error::Error;
use std::fs;

///
/// # Artifact Manager
/// Library for managing artifacts. It manages a local collection of artifacts and is responsible
/// getting artifacts from other nodes when they are not present locally.
///
/// Create an ArtifactManager object by passing a path for the local artifact repository to `new`.
/// ```
/// new ArtifactManager("/var/lib/pyrsia")
/// '''
pub struct ArtifactManager {
    pub repository_path : string,
}

impl ArtifactManager {
    pub fn new(repository_path : string) -> Result<ArtifactManager, &str> {
        ArtifactManager { repository_path }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
