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

use crate::artifact_service::service::{Digester, Hash, HashAlgorithm};
use crate::util::env_util::read_var;
use anyhow::{anyhow, bail, Context, Error, Result};
use lazy_static::lazy_static;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use std::panic::UnwindSafe;
use std::path::{Path, PathBuf};
use strum::IntoEnumIterator;
use walkdir::{DirEntry, WalkDir};

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

fn encode_bytes_as_file_name(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

// The base file path (no extension on the file name) that will correspond to this hash.
// The structure of the path is
// repo_root_dir/hash_algorithm/hash
// This consists of the artifact repository root directory, a directory whose name is the
// algorithm used to compute the hash and a file name that is the hash, encoded as hex
// (base64 is more compact, but hex is easier for troubleshooting). For example
// pyrsia-artifacts/SHA256/680fade3184f20557aa2bbf4432386eb79836902a1e5aea1ff077e323e6cab34
// TODO To support nodes that will store many files, we need a scheme that will start separating files by subdirectories under the hash algorithm directory based on the first n bytes of the hash value.
fn base_file_path(hash: &Hash, repo_dir: &Path) -> PathBuf {
    let mut buffer: PathBuf = PathBuf::from(repo_dir);
    buffer.push(hash.algorithm.hash_algorithm_to_str());
    buffer.push(encode_bytes_as_file_name(&hash.bytes));
    buffer
}

// It is possible, though unlikely, for SHA512, SHA3_512 and BLAKE3 to generate the same
// hash for different content. Separating files by algorithm avoids this type of collision.
// This function ensures that there is a directory under the repository root for each one of
// the supported hash algorithms.
fn ensure_directories_for_hash_algorithms_exist(
    repository_path: &Path,
) -> Result<(), anyhow::Error> {
    let mut path_buf = PathBuf::new();
    path_buf.push(repository_path);
    for algorithm in HashAlgorithm::iter() {
        ensure_subdirectory_exists(&path_buf, algorithm)?;
    }
    Ok(())
}

fn ensure_subdirectory_exists(
    path_buf: &Path,
    algorithm: HashAlgorithm,
) -> Result<(), anyhow::Error> {
    let mut this_buf = path_buf.to_path_buf();
    this_buf.push(algorithm.hash_algorithm_to_str());
    info!(
        "Creating directory {}",
        this_buf
            .as_os_str()
            .to_str()
            .unwrap_or("*** Unable to convert artifact directory path to UTF-8!")
    );
    std::fs::create_dir_all(this_buf.as_os_str())
        .with_context(|| format!("Error creating directory {}", this_buf.display()))?;
    Ok(())
}

// This is a decorator for the Write trait that allows the bytes written by the writer to be
// used to compute a hash
struct WriteHashDecorator<'a> {
    writer: &'a mut dyn Write,
    digester: &'a mut Box<dyn Digester>,
}

impl<'a> WriteHashDecorator<'a> {
    fn new(writer: &'a mut impl Write, digester: &'a mut Box<dyn Digester>) -> Self {
        WriteHashDecorator { writer, digester }
    }
}

// Decorator logic is supplied only for the methods that we expect to be called by io::copy
impl<'a> Write for WriteHashDecorator<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let result = self.writer.write(buf);
        if let Ok(bytes_written) = result {
            self.digester.update_hash(&buf[..bytes_written])
        };
        result
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        if self.writer.write(buf).is_ok() {
            self.digester.update_hash(buf)
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct ArtifactStorage {}

impl ArtifactStorage {
    pub fn new() -> Result<ArtifactStorage, anyhow::Error> {
        let absolute_path = get_repository_path()?;
        if is_accessible_directory(&absolute_path) {
            ensure_directories_for_hash_algorithms_exist(&absolute_path)?;

            Ok(ArtifactStorage {})
        } else {
            error!(
                "Unable to create ArtifactManager with inaccessible directory: {}",
                ARTIFACTS_DIR.as_str()
            );
            Err(anyhow!(
                "Not an accessible directory: {}",
                ARTIFACTS_DIR.as_str()
            ))
        }
    }

    // TODO After we restructure the directories to scale, counting files becomes an expensive operation. Provide this as an estimate, an async operation or both.
    pub fn artifacts_count(&self) -> Result<usize, Error> {
        let mut total_files = 0;

        let repository_path = get_repository_path()?;
        for entry in WalkDir::new(repository_path)
            .into_iter()
            .filter_entry(is_directory_or_artifact_file)
            .filter_map(|file| file.ok())
        {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    total_files += 1;
                }
            }
        }
        Ok(total_files)
    }

    pub fn artifacts_count_bydir(&self) -> Result<HashMap<String, usize>, Error> {
        let mut dirs_map: HashMap<String, usize> = HashMap::new();

        let repository_path = get_repository_path()?;
        for file in WalkDir::new(repository_path)
            .into_iter()
            .filter_entry(is_directory_or_artifact_file)
            .filter_map(|file| file.ok())
        {
            let path = file.path().display().to_string();

            let result_1 = path.rfind('/');
            let mut dir_1 = "";

            match result_1 {
                Some(x) => {
                    dir_1 = &path[0..x];
                }
                None => (),
            }

            if !dir_1.is_empty() {
                let len = dir_1.len();
                let result = dir_1.rfind('/');
                match result {
                    Some(x) => {
                        *dirs_map.entry(dir_1[x + 1..len].to_string()).or_insert(0) += 1;
                    }
                    None => (),
                }
            }
        }
        Ok(dirs_map)
    }

    /// List all artifacts that are known locally.
    pub fn list_artifacts(&self) -> Result<Vec<PathBuf>, Error> {
        let mut artifacts = Vec::new();

        let repository_path = get_repository_path()?;
        for entry in WalkDir::new(repository_path)
            .into_iter()
            .filter_entry(is_directory_or_artifact_file)
            .filter_map(|file| file.ok())
        {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    artifacts.push(entry.into_path());
                }
            }
        }

        Ok(artifacts)
    }

    /// Calculate the repository size by recursively adding size of each directory inside it.
    /// Returns the size
    pub fn space_used(&self) -> Result<u64, Error> {
        let repository_path = get_repository_path()?;
        fs_extra::dir::get_size(repository_path.as_os_str())
            .context("Error while calculating the size of artifact manager")
    }

    fn file_path_for_new_artifact(&self, expected_hash: &Hash) -> std::io::Result<PathBuf> {
        let repository_path = get_repository_path()?;
        let mut base_path: PathBuf = base_file_path(expected_hash, &repository_path);
        // for now all artifacts are unstructured
        base_path.set_extension(FILE_EXTENSION);
        Ok(base_path)
    }

    /// Push an artifact to this node's local repository.
    /// Parameters are:
    /// * reader — An object that this method will use to read the bytes of the artifact being
    ///            pushed.
    /// * expected_hash — The hash value that the pushed artifact is expected to have.
    /// Returns true if it created the artifact local or false if the artifact already existed.
    pub fn push_artifact(
        &self,
        reader: &mut impl Read,
        expected_hash: &Hash,
    ) -> Result<(), anyhow::Error> {
        info!(
            "An artifact is being pushed to the artifact manager {}",
            expected_hash
        );
        let base_path = self.file_path_for_new_artifact(expected_hash)?;
        debug!("Pushing artifact to {}", base_path.display());
        // Write to a temporary name that won't be mistaken for a valid file. If the hash checks out, rename it to the base name; otherwise delete it.
        let tmp_path = tmp_path_from_base(&base_path);

        let out = create_artifact_file(&tmp_path)?;
        debug!("hash is {}", expected_hash);
        let mut hash_buffer = [0; HASH_BUFFER_SIZE];
        let actual_hash = &*do_push(reader, expected_hash, &tmp_path, out, &mut hash_buffer)?;
        if actual_hash == expected_hash.bytes {
            rename_to_permanent(expected_hash, &base_path, &tmp_path)
        } else {
            handle_wrong_hash(expected_hash, tmp_path, actual_hash)
        }
    }

    /// Pull an artifact. The current implementation only looks in the local node's repository.
    pub fn pull_artifact(&self, hash: &Hash) -> Result<File, anyhow::Error> {
        info!(
            "An artifact is being pulled from the artifact manager {}",
            hash
        );
        let repository_path = get_repository_path()?;
        let mut base_path: PathBuf = base_file_path(hash, &repository_path);
        // for now all artifacts are unstructured
        base_path.set_extension(FILE_EXTENSION);
        debug!("Pulling artifact from {}", base_path.display());
        File::open(base_path.as_path())
            .with_context(|| format!("{} not found.", base_path.display()))
    }
}

fn get_repository_path() -> std::io::Result<PathBuf> {
    PathBuf::from(ARTIFACTS_DIR.as_str()).canonicalize()
}

// return true if the given repository path leads to an accessible directory.
fn is_accessible_directory(repository_path: &Path) -> bool {
    match std::fs::metadata(repository_path) {
        Err(_) => false,
        Ok(metadata) => metadata.is_dir(),
    }
}

// Return a temporary file name to use for the file until we have verified that the hash is correct.
// The temporary file name is guaranteed to be as unique as the hash and not to be mistaken for a
// file whose name is its has code.
//
// The reason for doing this is so that a file whose actual hash is not equal to the expected
// hash will not be found in the local repository from the time it is created and not fully
// written until the time its hash is verified. After that, the file is renamed to its permanent
// name that will match the actual hash value.
fn tmp_path_from_base(base: &Path) -> PathBuf {
    let mut tmp_buf = base.to_path_buf();
    let file_name: &OsStr = base.file_name().unwrap();
    tmp_buf.set_file_name(format!("l0-{}", file_name.to_str().unwrap()));
    tmp_buf
}

fn is_directory_or_artifact_file(entry: &DirEntry) -> bool {
    let not_hidden = entry
        .file_name()
        .to_str()
        .map(|s| entry.depth() == 0 || !s.starts_with('.'))
        .unwrap_or(false);
    not_hidden
        && (entry.file_type().is_dir()
            || entry
                .path()
                .extension()
                .map(|extension| extension == OsString::from(FILE_EXTENSION).as_os_str())
                .unwrap_or(false))
}

fn create_artifact_file(tmp_path: &Path) -> std::io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(tmp_path)
}

fn handle_wrong_hash(
    expected_hash: &Hash,
    tmp_path: PathBuf,
    actual_hash: &[u8],
) -> Result<(), Error> {
    std::fs::remove_file(tmp_path.clone()).with_context(|| {
        format!(
            "Attempted to remove {} because its content has the wrong hash.",
            tmp_path.to_str().unwrap()
        )
    })?;
    let msg = format!("Contents of artifact did not have the expected hash value of {}. The actual hash was {}:{}",
                      expected_hash, expected_hash.algorithm, hex::encode(actual_hash));
    warn!("{}", msg);
    bail!("{}", msg)
}

fn rename_to_permanent(
    expected_hash: &Hash,
    base_path: &Path,
    tmp_path: &Path,
) -> Result<(), anyhow::Error> {
    std::fs::rename(tmp_path, base_path).with_context(|| {
        format!(
            "Attempting to rename from temporary file name{} to permanent{}",
            tmp_path.to_str().unwrap(),
            base_path.to_str().unwrap()
        )
    })?;
    debug!(
        "Artifact has the expected hash available locally {}",
        expected_hash
    );
    Ok(())
}

fn do_push<'b>(
    reader: &mut impl Read,
    expected_hash: &Hash,
    path: &Path,
    out: File,
    hash_buffer: &'b mut [u8; HASH_BUFFER_SIZE],
) -> Result<&'b [u8], Error> {
    let mut buf_writer: BufWriter<File> = BufWriter::new(out);
    let mut digester = expected_hash.algorithm.digest_factory();
    let mut writer = WriteHashDecorator::new(&mut buf_writer, &mut digester);

    copy_from_reader_to_writer(reader, path, &mut writer)
        .with_context(|| format!("Error writing contents of {}", expected_hash))?;
    Ok(actual_hash(hash_buffer, &mut digester))
}

const HASH_BUFFER_SIZE: usize = 128;

fn actual_hash<'b>(
    hash_buffer: &'b mut [u8; HASH_BUFFER_SIZE],
    digester: &mut Box<dyn Digester>,
) -> &'b mut [u8] {
    let buffer_slice: &mut [u8] = &mut hash_buffer[..digester.hash_size_in_bytes()];
    digester.finalize_hash(buffer_slice);
    buffer_slice
}

fn copy_from_reader_to_writer(
    reader: &mut impl Read,
    path: &Path,
    mut writer: &mut WriteHashDecorator,
) -> Result<(), Error> {
    std::io::copy(reader, &mut writer).with_context(|| {
        format!(
            "Error while copying artifact contents to {}",
            path.display()
        )
    })?;
    writer.flush().with_context(|| {
        format!(
            "Error while flushing last of artifact contents to {}",
            path.display()
        )
    })
}
