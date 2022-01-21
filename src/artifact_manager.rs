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

extern crate walkdir;

use anyhow::{anyhow, Context, Error, Result};
use log::{debug, error, info, warn}; //log_enabled, Level,
use path::PathBuf;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{BufWriter, Read, Write};
use std::path;
use std::path::Path;
use std::str::FromStr;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};
use walkdir::{DirEntry, WalkDir};

///
/// # Artifact Manager
/// Module for managing artifacts. It manages a local collection of artifacts and is responsible
/// getting artifacts from other nodes when they are not present locally.
/// An artifact is a file that is identified by a hash algorithm and a hash code. To know more about
/// an artifact, we must consult the metadata that refers to the artifact.

// We will provide implementations of this trait for each hash algorithm that we support.
trait Digester {
    fn update_hash(&mut self, input: &[u8]);

    fn finalize_hash(&mut self, hash_buffer: &mut [u8]);

    fn hash_size_in_bytes(&self) -> usize;
}

impl Digester for Sha256 {
    fn update_hash(&mut self, input: &[u8]) {
        self.update(input);
    }

    fn finalize_hash(&mut self, hash_buffer: &mut [u8]) {
        hash_buffer.clone_from_slice(self.clone().finalize().as_slice());
    }

    fn hash_size_in_bytes(&self) -> usize {
        256 / 8
    }
}

impl Digester for Sha512 {
    fn update_hash(&mut self, input: &[u8]) {
        self.update(input);
    }

    fn finalize_hash(&mut self, hash_buffer: &mut [u8]) {
        hash_buffer.clone_from_slice(self.clone().finalize().as_slice());
    }

    fn hash_size_in_bytes(&self) -> usize {
        512 / 8
    }
}

/// The types of hash algorithms that the artifact manager supports

#[derive(EnumIter, Clone, Debug, PartialEq, EnumString, Serialize, Deserialize)]
pub enum HashAlgorithm {
    SHA256,
    SHA512,
}

impl HashAlgorithm {
    /// Translate a string that names a hash algorithm to the enum variant.
    pub fn str_to_hash_algorithm(algorithm_name: &str) -> Result<HashAlgorithm, anyhow::Error> {
        HashAlgorithm::from_str(&algorithm_name.to_uppercase()).with_context(|| {
            format!(
                "{} is not the name of a supported HashAlgorithm.",
                algorithm_name
            )
        })
    }

    fn digest_factory(&self) -> Box<dyn Digester> {
        match self {
            HashAlgorithm::SHA256 => Box::new(Sha256::new()),
            HashAlgorithm::SHA512 => Box::new(Sha512::new()),
        }
    }

    /// Translate a HashAlgorithm to a string.
    pub fn hash_algorithm_to_str(&self) -> &'static str {
        match self {
            HashAlgorithm::SHA256 => "SHA256",
            HashAlgorithm::SHA512 => "SHA512",
        }
    }

    fn hash_length_in_bytes(&self) -> usize {
        match self {
            HashAlgorithm::SHA256 => 256 / 8,
            HashAlgorithm::SHA512 => 512 / 8,
        }
    }
}

impl std::fmt::Display for HashAlgorithm {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(HashAlgorithm::hash_algorithm_to_str(self))
    }
}

pub struct Hash<'a> {
    algorithm: HashAlgorithm,
    bytes: &'a [u8],
}

impl<'a> Hash<'a> {
    pub fn new(algorithm: HashAlgorithm, bytes: &'a [u8]) -> Result<Self, anyhow::Error> {
        let expected_length: usize = algorithm.hash_length_in_bytes();
        if bytes.len() == expected_length {
            Ok(Hash { algorithm, bytes })
        } else {
            Err(anyhow!(format!("The hash value does not have the correct length for the algorithm. The expected length is {} bytes, but the length of the supplied hash is {}.", expected_length, bytes.len())))
        }
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
            Self::ensure_subdirectory_exists(&mut path_buf, algorithm)?;
        }
        Ok(())
    }

    fn ensure_subdirectory_exists(
        path_buf: &mut PathBuf,
        algorithm: HashAlgorithm,
    ) -> Result<(), anyhow::Error> {
        let mut this_buf = path_buf.clone();
        this_buf.push(algorithm.hash_algorithm_to_str());
        info!(
            "Creating directory {}",
            this_buf
                .as_os_str()
                .to_str()
                .unwrap_or("*** Unable to convert artifact directory path to UTF-8!")
        );
        fs::create_dir_all(this_buf.as_os_str())
            .with_context(|| format!("Error creating directory {}", this_buf.display()))?;
        Ok(())
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
    // pyrsia-artifacts/SHA256/68efadf3184f20557aa2bbf4432386eb79836902a1e5aea1ff077e323e6ccbb4
    // TODO To support nodes that will store many files, we need a scheme that will start separating files by subdirectories under the hash algorithm directory based on the first n bytes of the hash value.
    fn base_file_path(&self, repo_dir: &Path) -> PathBuf {
        let mut buffer: PathBuf = PathBuf::from(repo_dir);
        buffer.push(self.algorithm.hash_algorithm_to_str());
        buffer.push(Hash::encode_bytes_as_file_name(self.bytes));
        buffer
    }
}

impl Display for Hash<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "{}:{}",
            self.algorithm.hash_algorithm_to_str(),
            hex::encode(self.bytes)
        ))
    }
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
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let result = self.writer.write(buf);
        if let Ok(bytes_written) = result {
            self.digester.update_hash(&buf[..bytes_written])
        };
        result
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        if self.writer.write(buf).is_ok() {
            self.digester.update_hash(buf)
        }
        Ok(())
    }
}

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
/// may be used in the future to indicate that the file has a particular internal structure that
/// Pyrsia needs to know about.
pub struct ArtifactManager {
    pub repository_path: PathBuf,
}

impl<'a> ArtifactManager {
    /// Create a new ArtifactManager that works with artifacts in the given directory
    pub fn new(repository_path: &str) -> Result<ArtifactManager, anyhow::Error> {
        let absolute_path = Path::new(repository_path).canonicalize()?;
        if is_accessible_directory(&absolute_path) {
            Hash::ensure_directories_for_hash_algorithms_exist(&absolute_path)?;
            info!(
                "Creating an ArtifactManager with a repository in {}",
                absolute_path.display()
            );
            Ok(ArtifactManager {
                repository_path: absolute_path,
            })
        } else {
            Self::inaccessible_repo_directory_error(repository_path)
        }
    }

    fn inaccessible_repo_directory_error(repository_path: &str) -> Result<ArtifactManager, Error> {
        error!(
            "Unable to create ArtifactManager with inaccessible directory: {}",
            repository_path
        );
        Err(anyhow!("Not an accessible directory: {}", repository_path))
    }

    pub fn artifacts_count(&self, repository_path: &str) -> Result<usize, Error> {
        let mut total_files = 0;

        for file in WalkDir::new(repository_path)
            .into_iter()
            .filter_entry(|path| Self::is_not_hidden(path))
            .filter_map(|file| file.ok())
        {
            if file.metadata().unwrap().is_file() {
                total_files += 1;
            }
        }
        Ok(total_files)
    }

    fn is_not_hidden(entry: &DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .map(|s| entry.depth() == 0 || !s.starts_with('.'))
            .unwrap_or(false)
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
    ) -> Result<bool, anyhow::Error> {
        info!(
            "An artifact is being pushed to the artifact manager {}",
            expected_hash
        );
        let base_path = self.file_path_for_new_artifact(expected_hash);
        debug!("Pushing artifact to {}", base_path.display());
        // Write to a temporary name that won't be mistaken for a valid file. If the hash checks out, rename it to the base name; otherwise delete it.
        let tmp_path = tmp_path_from_base(&base_path);

        match Self::create_artifact_file(&tmp_path) {
            Err(error) => file_creation_error(&tmp_path, error),
            Ok(out) => {
                debug!("hash is {}", expected_hash);
                let mut hash_buffer: [u8; 128] = [0; 128];
                let actual_hash =
                    &*Self::do_push(reader, expected_hash, &tmp_path, out, &mut hash_buffer)?;
                if actual_hash == expected_hash.bytes {
                    Self::rename_to_permanent(expected_hash, base_path, &tmp_path)
                } else {
                    Self::handle_wrong_hash(expected_hash, tmp_path, actual_hash)
                }
            }
        }
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
    ) -> Result<bool, Error> {
        fs::remove_file(tmp_path.clone()).with_context(|| {
            format!(
                "Attempted to remove {} because its content has the wrong hash.",
                tmp_path.to_str().unwrap()
            )
        })?;
        warn!("Contents of artifact did not have the expected hash value of {}. The actual hash was {}:{}", expected_hash, expected_hash.algorithm, hex::encode(actual_hash));
        Err(anyhow!("Contents of artifact did not have the expected hash value of {}. The actual hash was {}:{}", expected_hash, expected_hash.algorithm, hex::encode(actual_hash)))
    }

    fn rename_to_permanent(
        expected_hash: &Hash,
        base_path: PathBuf,
        tmp_path: &Path,
    ) -> Result<bool, anyhow::Error> {
        fs::rename(tmp_path.to_path_buf(), base_path.clone()).with_context(|| {
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
        Ok(true)
    }

    fn file_path_for_new_artifact(&self, expected_hash: &Hash) -> PathBuf {
        let mut base_path: PathBuf = expected_hash.base_file_path(&self.repository_path);
        // for now all artifacts are unstructured
        base_path.set_extension("file");
        base_path
    }

    fn do_push<'b>(
        reader: &mut impl Read,
        expected_hash: &Hash,
        path: &Path,
        out: File,
        hash_buffer: &'b mut [u8; 128],
    ) -> Result<&'b [u8], Error> {
        let mut buf_writer: BufWriter<File> = BufWriter::new(out);
        let mut digester = expected_hash.algorithm.digest_factory();
        let mut writer = WriteHashDecorator::new(&mut buf_writer, &mut digester);

        Self::copy_from_reader_to_writer(reader, path, &mut writer)
            .with_context(|| format!("Error writing contents of {}", expected_hash))?;
        Ok(Self::actual_hash(hash_buffer, &mut digester))
    }

    fn actual_hash<'b>(
        hash_buffer: &'b mut [u8; 128],
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
        io::copy(reader, &mut writer).with_context(|| {
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

    /// Pull an artifact. The current implementation only looks in the local node's repository.
    /// A future
    ///
    pub fn pull_artifact(&self, hash: &Hash) -> Result<File, anyhow::Error> {
        info!(
            "An artifact is being pulled from the artifact manager {}",
            hash
        );
        let mut base_path: PathBuf = hash.base_file_path(&self.repository_path);
        // for now all artifacts are unstructured
        base_path.set_extension("file");
        debug!("Pulling artifact from {}", base_path.display());
        File::open(base_path.as_path())
            .with_context(|| format!("{} not found.", base_path.display()))
    }
}

fn file_creation_error(base_path: &Path, error: std::io::Error) -> Result<bool, Error> {
    error!("I/O error {} on {}", error, base_path.display());
    match error.kind() {
        io::ErrorKind::AlreadyExists => Ok(false),
        _ => Err(anyhow!(error.to_string())),
    }
    .with_context(|| format!("Error creating file {}", base_path.display()))
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
    tmp_buf.set_file_name(format!("X{}", file_name.to_str().unwrap()));
    tmp_buf
}

// return true if the given repository path leads to an accessible directory.
fn is_accessible_directory(repository_path: &Path) -> bool {
    match fs::metadata(repository_path) {
        Err(_) => false,
        Ok(metadata) => metadata.is_dir(),
    }
}

#[cfg(test)]
mod tests {
    use crate::artifact_manager::{ArtifactManager, Hash, HashAlgorithm};
    use anyhow::{anyhow, Context};
    use env_logger::Target;
    use log::{info, LevelFilter};
    use std::fs;
    use std::io::Read;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use stringreader::StringReader;

    pub use super::*;

    #[cfg(test)]
    #[ctor::ctor]
    fn init() {
        let _ignore = env_logger::builder()
            .is_test(true)
            .target(Target::Stdout)
            .filter(None, LevelFilter::Debug)
            .try_init();
    }

    #[test]
    pub fn new_artifact_manager_with_valid_directory() {
        let dir_name = "TmpX";
        fs::create_dir(dir_name).expect(&format!("Unable to create temp directory {}", dir_name));
        info!("Created directory for valid directory test: {}", dir_name);
        let ok: bool = match ArtifactManager::new(dir_name) {
            Ok(artifact_manager) => {
                info!(
                    "Artifact manager created with repo directory {}",
                    artifact_manager.repository_path.display()
                );
                let mut sha256_path = artifact_manager.repository_path.clone();
                sha256_path.push(HashAlgorithm::SHA256.hash_algorithm_to_str());
                let meta256 = fs::metadata(sha256_path.as_path()).expect(
                    format!("unable to get metadata for {}", sha256_path.display()).as_str(),
                );
                assert!(meta256.is_dir());

                let mut sha512_path = artifact_manager.repository_path.clone();
                sha512_path.push(HashAlgorithm::SHA512.hash_algorithm_to_str());
                let meta512 = fs::metadata(sha512_path.as_path()).expect(
                    format!("unable to get metadata for {}", sha512_path.display()).as_str(),
                );
                assert!(meta512.is_dir());
                fs::remove_dir_all(artifact_manager.repository_path.as_path()).expect(&format!(
                    "unable to remove temp directory {}",
                    artifact_manager.repository_path.display()
                ));
                true
            }
            Err(_) => false,
        };
        assert!(ok)
    }

    const TEST_ARTIFACT_DATA: &str = "Incumbent nonsense text, sesquipedalian and obfuscatory. Exhortations to the mother lode. Dendrites for all.";
    const TEST_ARTIFACT_HASH: [u8; 32] = [
        0x6b, 0x29, 0xf2, 0xf1, 0xe5, 0x02, 0x4c, 0x41, 0x95, 0x06, 0xe9, 0x50, 0x3e, 0x02, 0x4b,
        0x3d, 0x8a, 0x5a, 0x08, 0xb6, 0xf6, 0xd5, 0x5b, 0x68, 0x88, 0x66, 0x79, 0x52, 0xd1, 0x04,
        0x15, 0x54,
    ];
    const WRONG_ARTIFACT_HASH: [u8; 32] = [
        0x2d, 0x8c, 0x2f, 0x6d, 0x97, 0x8c, 0xa2, 0x17, 0x12, 0xb5, 0xf6, 0xde, 0x36, 0xc9, 0xd3,
        0x1f, 0xa8, 0xe9, 0x6a, 0x4f, 0xa5, 0xd8, 0xff, 0x8b, 0x01, 0x88, 0xdf, 0xb9, 0xe7, 0xc1,
        0x71, 0xbb,
    ];

    #[test]
    pub fn new_artifact_manager_with_bad_directory() {
        let ok: bool = match ArtifactManager::new("BoGuS") {
            Ok(_) => false,
            Err(_) => true,
        };
        assert!(ok)
    }

    #[test]
    pub fn happy_push_pull_test() -> Result<(), anyhow::Error> {
        let mut string_reader = StringReader::new(TEST_ARTIFACT_DATA);
        let hash = Hash::new(HashAlgorithm::SHA256, &TEST_ARTIFACT_HASH)?;
        let dir_name = tmp_dir_name("tmp");
        debug!("tmp dir: {}", dir_name);
        fs::create_dir(dir_name.clone())
            .context(format!("Error creating directory {}", dir_name.clone()))?;
        let am: ArtifactManager =
            ArtifactManager::new(dir_name.as_str()).context("Error creating ArtifactManager")?;
        am.push_artifact(&mut string_reader, &hash)
            .context("Error from push_artifact")?;

        let mut path_buf = PathBuf::from(dir_name.clone());
        path_buf.push("SHA256");
        path_buf.push(hex::encode(TEST_ARTIFACT_HASH));
        path_buf.set_extension("file");
        let content_vec = fs::read(path_buf.as_path()).context("reading pushed file")?;
        assert_eq!(content_vec.as_slice(), TEST_ARTIFACT_DATA.as_bytes());

        let mut reader = am
            .pull_artifact(&hash)
            .context("Error from pull_artifact")?;
        let mut read_buffer = String::new();
        reader.read_to_string(&mut read_buffer)?;
        assert_eq!(TEST_ARTIFACT_DATA, read_buffer);

        fs::remove_dir_all(dir_name.clone())
            .context(format!("Error removing directory {}", dir_name))?;
        Ok(())
    }

    #[test]
    pub fn push_wrong_hash_test() -> Result<(), anyhow::Error> {
        let mut string_reader = StringReader::new(TEST_ARTIFACT_DATA);
        let hash_algorithm = HashAlgorithm::str_to_hash_algorithm("SHA256")?;
        let hash = Hash::new(hash_algorithm, &WRONG_ARTIFACT_HASH)?;
        let dir_name = tmp_dir_name("TmpW");
        debug!("tmp dir: {}", dir_name);
        fs::create_dir(dir_name.clone())
            .context(format!("Error creating directory {}", dir_name.clone()))?;
        let am: ArtifactManager =
            ArtifactManager::new(dir_name.as_str()).context("Error creating ArtifactManager")?;
        let ok = match am
            .push_artifact(&mut string_reader, &hash)
            .context("Error from push_artifact")
        {
            Ok(_) => Err(anyhow!(
                "push_artifact should have returned an error because of the wrong hash"
            )),
            Err(_) => Ok(()),
        };
        fs::remove_dir_all(dir_name.clone())
            .expect(&format!("unable to remove temp directory {}", dir_name));
        ok
    }

    #[test]
    pub fn pull_nonexistent_test() -> Result<(), anyhow::Error> {
        let hash = Hash::new(HashAlgorithm::SHA256, &WRONG_ARTIFACT_HASH)?;
        let dir_name = tmp_dir_name("TmpR");
        debug!("tmp dir: {}", dir_name);
        fs::create_dir(dir_name.clone())
            .context(format!("Error creating directory {}", dir_name.clone()))?;
        let am: ArtifactManager =
            ArtifactManager::new(dir_name.as_str()).context("Error creating ArtifactManager")?;
        let ok = match am.pull_artifact(&hash).context("Error from push_artifact") {
            Ok(_) => Err(anyhow!(
                "pull_artifact should have failed with nonexistent hash."
            )),
            Err(_) => Ok(()),
        };
        fs::remove_dir_all(dir_name.clone())
            .expect(&format!("unable to remove temp directory {}", dir_name));
        ok
    }

    pub fn tmp_dir_name(prefix: &str) -> String {
        return format!(
            "{}{}",
            prefix,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis()
        );
    }
}
