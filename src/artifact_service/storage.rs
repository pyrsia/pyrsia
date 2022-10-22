/*
   Copyright 2021 JFrog Ltd

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

use crate::util::env_util::read_var;
use anyhow::{anyhow, Context, Result};
use lazy_static::lazy_static;
use log::{debug, error, info};
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Read, Write};
use std::panic::UnwindSafe;
use std::path::{Path, PathBuf};

const FILE_EXTENSION: &str = "file";

lazy_static! {
    pub static ref ARTIFACTS_DIR: String = {
        let pyrsia_artifact_path = read_var("PYRSIA_ARTIFACT_PATH", "pyrsia");
        let dev_mode = read_var("DEV_MODE", "off");
        if dev_mode.to_lowercase() == "on" {
            log_static_initialization_failure(
                "Pyrsia Artifact directory",
                std::fs::create_dir_all(&pyrsia_artifact_path).with_context(|| {
                    format!(
                        "Failed to create artifact manager directory {:?} in dev mode",
                        pyrsia_artifact_path
                    )
                }),
            );
        }
        pyrsia_artifact_path
    };
}

fn log_static_initialization_failure<T: UnwindSafe>(
    label: &str,
    result: Result<T, anyhow::Error>,
) -> T {
    let panic_wrapper = std::panic::catch_unwind(|| match result {
        Ok(unwrapped) => unwrapped,
        Err(error) => {
            let msg = format!("Error initializing {}, error is: {}", label, error);
            error!("{}", msg);
            panic!("{}", msg)
        }
    });
    match panic_wrapper {
        Ok(normal) => normal,
        Err(partially_unwound_panic) => {
            error!("Initialization of {} panicked!", label);
            std::panic::resume_unwind(partially_unwound_panic)
        }
    }
}

#[derive(Clone)]
pub struct ArtifactStorage {
    repository_path: PathBuf,
}

impl ArtifactStorage {
    pub fn new<P: AsRef<Path>>(repository_path: P) -> Result<ArtifactStorage, anyhow::Error> {
        let absolute_path = repository_path.as_ref().to_path_buf().canonicalize()?;
        if absolute_path.is_dir() {
            Ok(ArtifactStorage {
                repository_path: absolute_path,
            })
        } else {
            error!(
                "Unable to create ArtifactManager with inaccessible directory: {:?}",
                absolute_path
            );
            Err(anyhow!("Not an accessible directory: {:?}", absolute_path))
        }
    }

    // The base file path (no extension on the file name) that will correspond to this artifact id.
    // The structure of the path is: `repo_root_dir/artifact_id`. This consists of the artifact
    // repository root directory and a file name that is the artifact id. For example:
    // `pyrsia-artifacts/e131322a-0c72-454d-b7a0-dcdb53c1bbdf
    //
    // TODO To support nodes that will store many files, we need a scheme that will start separating
    // files by subdirectories based on the first n bytes of the artifact id.
    fn base_file_path(&self, artifact_id: &str) -> PathBuf {
        let mut path: PathBuf = PathBuf::from(&self.repository_path);
        path.push(artifact_id);
        path
    }

    fn artifact_file_path(&self, artifact_id: &str) -> io::Result<PathBuf> {
        let mut base_file_path: PathBuf = self.base_file_path(artifact_id);
        base_file_path.set_extension(FILE_EXTENSION);
        Ok(base_file_path)
    }

    fn create_artifact_file(&self, artifact_id: &str) -> io::Result<File> {
        let artifact_file_path = self.artifact_file_path(artifact_id)?;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(artifact_file_path)
    }

    /// Push an artifact to this node's local repository.
    /// Parameters are:
    /// * reader — An object that this method will use to read the bytes of the artifact being
    ///            pushed.
    /// * artifact_id — The id that the pushed artifact is expected to have.
    pub fn push_artifact(&self, reader: &mut impl Read, artifact_id: &str) -> io::Result<()> {
        info!(
            "An artifact is being pushed to the artifact manager {}",
            artifact_id
        );

        let artifact_file = self.create_artifact_file(artifact_id)?;
        let mut writer = BufWriter::new(artifact_file);
        io::copy(reader, &mut writer)?;
        writer.flush()
    }

    /// Pull an artifact. The current implementation only looks in the local node's repository.
    pub fn pull_artifact(&self, artifact_id: &str) -> io::Result<impl Read> {
        info!(
            "An artifact is being pulled from the artifact manager {}",
            artifact_id
        );
        let artifact_file_path = self.artifact_file_path(artifact_id)?;
        File::open(artifact_file_path)
    }

    /// List all artifacts found in the repository path.
    /// The current implementation only looks in the local node's repository.
    pub fn list_artifacts(&self) -> Result<Vec<PathBuf>> {
        let root: PathBuf = PathBuf::from(&self.repository_path);
        debug!("Finding stored artifacts");
        if root.is_dir() {
            let vec: Vec<PathBuf> = std::fs::read_dir(root)?
                .filter_map(|entry| {
                    let path = entry.unwrap().path();
                    match path.extension() {
                        Some(ext) if ext.eq(FILE_EXTENSION) => Some(path),
                        _ => None,
                    }
                })
                .collect();
            debug!("There are {} stored artifacts ", vec.len());
            return Ok(vec);
        }
        Ok(Vec::new())
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::util::test_util;
    use std::path::PathBuf;
    use stringreader::StringReader;
    use uuid::Uuid;

    #[test]
    pub fn new_artifact_storage_with_valid_directory() {
        let tmp_dir = test_util::tests::setup();

        ArtifactStorage::new(&tmp_dir).expect("ArtifactStorage should be created.");

        test_util::tests::teardown(tmp_dir);
    }

    const TEST_ARTIFACT_DATA: &str = "Incumbent nonsense text, sesquipedalian and obfuscatory. Exhortations to the mother lode. Dendrites for all.";

    #[test]
    pub fn new_artifact_storage_with_non_existing_directory() {
        assert!(ArtifactStorage::new(PathBuf::from("bogus")).is_err());
    }

    #[test]
    pub fn new_artifact_storage_with_file_as_repository_path() {
        let tmp_dir = test_util::tests::setup();
        let tmp_file_path = PathBuf::from(&tmp_dir).join("sample.file");
        File::create(&tmp_file_path).unwrap();

        assert!(ArtifactStorage::new(&tmp_file_path).is_err());

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    pub fn push_artifact_then_pull_it() {
        let tmp_dir = test_util::tests::setup();

        let mut string_reader = StringReader::new(TEST_ARTIFACT_DATA);
        let artifact_id = Uuid::new_v4().to_string();
        let artifact_storage =
            ArtifactStorage::new(&tmp_dir).expect("Error creating ArtifactManager");

        artifact_storage
            .push_artifact(&mut string_reader, &artifact_id)
            .context("Error from push_artifact")
            .unwrap();

        check_artifact_is_written_correctly(&tmp_dir, &artifact_id).unwrap();

        check_able_to_pull_artifact(&artifact_id, &artifact_storage).unwrap();

        test_util::tests::teardown(tmp_dir);
    }

    fn check_artifact_is_written_correctly(dir_name: &Path, artifact_id: &str) -> Result<()> {
        let mut dir_name = dir_name.to_path_buf();
        dir_name.push(artifact_id);
        dir_name.set_extension(FILE_EXTENSION);
        let content_vec = std::fs::read(dir_name.as_path())
            .context("reading pushed file")
            .unwrap();
        assert_eq!(content_vec.as_slice(), TEST_ARTIFACT_DATA.as_bytes());

        Ok(())
    }

    fn check_able_to_pull_artifact(
        artifact_id: &str,
        artifact_storage: &ArtifactStorage,
    ) -> Result<()> {
        let mut reader = artifact_storage
            .pull_artifact(artifact_id)
            .context("Error from pull_artifact")?;
        let mut read_buffer = String::new();
        reader.read_to_string(&mut read_buffer).unwrap();
        assert_eq!(TEST_ARTIFACT_DATA, read_buffer);

        Ok(())
    }

    #[test]
    pub fn pull_nonexistent_test() {
        let tmp_dir = test_util::tests::setup();

        let artifact_id = Uuid::new_v4().to_string();
        let artifact_storage =
            ArtifactStorage::new(&tmp_dir).expect("Error creating ArtifactManager");
        assert!(artifact_storage.pull_artifact(&artifact_id).is_err());

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    pub fn list_artifacts_test() {
        let tmp_dir = test_util::tests::setup();

        let mut string_reader = StringReader::new(TEST_ARTIFACT_DATA);
        let artifact_id = Uuid::new_v4().to_string();
        let artifact_storage =
            ArtifactStorage::new(&tmp_dir).expect("Error creating ArtifactManager");

        artifact_storage
            .push_artifact(&mut string_reader, &artifact_id)
            .context("Error from push_artifact")
            .unwrap();

        let result = artifact_storage.list_artifacts();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);

        test_util::tests::teardown(tmp_dir);
    }
}
