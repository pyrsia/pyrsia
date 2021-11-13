///
/// # Artifact Manager
/// Module for managing artifacts. It manages a local collection of artifacts and is responsible
/// getting artifacts from other nodes when they are not present locally.
///
pub mod artifact_manager {
    use log::{debug, error, info, warn}; //log_enabled, Level,
    use path::PathBuf;
    use std::ffi::OsStr;
    use std::fmt::{Arguments, Display, Formatter};
    use std::fs;
    use std::fs::{File, OpenOptions};
    use std::io;
    use std::io::{BufWriter, IoSlice, Read, Write};
    use std::path;
    use std::str::FromStr;

    use anyhow::{anyhow, Context, Error, Result};
    use crypto::digest::Digest;
    use crypto::sha2::{Sha256, Sha512};
    use strum::IntoEnumIterator;
    use strum_macros::{EnumIter, EnumString};

    // We will provide implementations of this trait for each hash algorithm that we support.
    trait Digester {
        fn update_hash(&mut self, input: &[u8]);

        fn finalize_hash(&mut self, hash_buffer: &mut [u8]);

        fn hash_size_in_bytes(&self) -> usize;
    }

    impl Digester for Sha256 {
        fn update_hash(self: &mut Self, input: &[u8]) {
            self.input(&*input);
        }

        fn finalize_hash(self: &mut Self, hash_buffer: &mut [u8]) {
            let mut hash_array: [u8; 32] = [0; 32];
            self.result(&mut hash_array);
            let mut i = 0;
            while i < hash_array.len() {
                hash_buffer[i] = hash_array[i];
                i += 1;
            }
        }

        fn hash_size_in_bytes(&self) -> usize {
            return 256 / 8;
        }
    }

    impl Digester for Sha512 {
        fn update_hash(self: &mut Self, input: &[u8]) {
            self.input(&*input);
        }

        fn finalize_hash(self: &mut Self, hash_buffer: &mut [u8]) {
            let mut hash_array: [u8; 64] = [0; 64];
            self.result(&mut hash_array);
            let mut i = 0;
            while i < 64 {
                hash_buffer[i] = hash_array[i];
                i += 1;
            }
        }

        fn hash_size_in_bytes(&self) -> usize {
            return 512 / 8;
        }
    }

    /// The types of hash algorithms that the artifact manager supports
    #[derive(EnumIter, Debug, PartialEq, EnumString)]
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
            return match self {
                HashAlgorithm::SHA256 => "SHA256",
                HashAlgorithm::SHA512 => "SHA512",
            };
        }

        fn hash_length_in_bytes(&self) -> usize {
            return match self {
                HashAlgorithm::SHA256 => 256 / 8,
                HashAlgorithm::SHA512 => 512 / 8,
            };
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
            repository_path: &str,
        ) -> Result<(), anyhow::Error> {
            let mut path_buf = PathBuf::new();
            path_buf.push(repository_path);
            for algorithm in HashAlgorithm::iter() {
                let mut this_buf = path_buf.clone();
                this_buf.push(algorithm.hash_algorithm_to_str());
                fs::create_dir_all(this_buf.as_os_str())
                    .with_context(|| format!("Error creating directory {}", this_buf.display()))?;
            }
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
        fn base_file_path(&self, repo_dir: &str) -> PathBuf {
            let mut buffer: PathBuf = PathBuf::from(repo_dir);
            buffer.push(self.algorithm.hash_algorithm_to_str());
            buffer.push(Hash::encode_bytes_as_file_name(self.bytes));
            return buffer;
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
            return WriteHashDecorator { writer, digester };
        }
    }

    // Decorator logic is supplied only for the methods that we expect to be called by io::copy
    impl<'a> Write for WriteHashDecorator<'a> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let result = self.writer.write(buf);
            match result {
                // hash just the number of bytes that were actually written. This may be less than the whole buffer.
                Ok(bytes_written) => self.digester.update_hash(&buf[..bytes_written]),
                _ => {}
            }
            return result;
        }

        fn write_vectored(&mut self, _bufs: &[IoSlice<'_>]) -> io::Result<usize> {
            unimplemented!()
        }

        fn flush(&mut self) -> io::Result<()> {
            return self.writer.flush();
        }

        fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
            let result = self.writer.write(buf);
            match result {
                // hash just the number of bytes that were actually written. This may be less than the whole buffer.
                Ok(_) => self.digester.update_hash(buf),
                _ => {}
            }
            return Ok(());
        }

        fn write_fmt(&mut self, _fmt: Arguments<'_>) -> io::Result<()> {
            unimplemented!()
        }

        fn by_ref(&mut self) -> &mut Self
        where
            Self: Sized,
        {
            unimplemented!()
        }
    }

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
    /// may be used in the future to indicate that the file has a particular internal structure that
    /// Pyrsia needs to know about.
    pub struct ArtifactManager<'a> {
        pub repository_path: &'a str,
    }

    impl<'a> ArtifactManager<'a> {
        /// Create a new ArtifactManager that works with artifacts in the given directory
        pub fn new(repository_path: &str) -> Result<ArtifactManager, anyhow::Error> {
            if is_accessible_directory(repository_path) {
                Hash::ensure_directories_for_hash_algorithms_exist(repository_path)?;
                info!(
                    "Creating an ArtifactManager with a repostitory in {}",
                    repository_path
                );
                Ok(ArtifactManager { repository_path })
            } else {
                error!(
                    "Unable to create ArtifactManager with inaccessible directory: {}",
                    repository_path
                );
                Err(anyhow!("Not an accessible directory: {}", repository_path))
            }
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
            let mut base_path: PathBuf = expected_hash.base_file_path(self.repository_path);
            // for now all artifacts are unstructured
            base_path.set_extension("file");
            debug!("Pushing artifact to {}", base_path.display());
            // Write to a temporary name that won't be mistaken for a valid file. If the hash checks out, rename it to the base name; otherwise delete it.
            let tmp_path = tmp_path_from_base(&base_path);

            let open_result = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(tmp_path.as_path());
            return match open_result {
                Err(error) => Self::file_creation_error(&tmp_path, error),
                Ok(out) => {
                    println!("hash is {}", expected_hash);
                    let mut hash_buffer: [u8; 128] = [0; 128];
                    // file out is implicitly closed by being borrowed by the following function call and then being dropped in the function call.
                    let actual_hash =
                        &*Self::do_push(reader, expected_hash, &tmp_path, out, &mut hash_buffer)
                            .with_context(|| {
                                format!("Error writing contents of {}", expected_hash)
                            })?;
                    if actual_hash == expected_hash.bytes {
                        fs::rename(tmp_path.clone(), base_path.clone()).with_context(|| {
                            format!(
                                "Attempting to rename from temporary file name{} to permanent{}",
                                tmp_path.to_str().unwrap(),
                                base_path.to_str().unwrap()
                            )
                        })?;
                        debug!(
                            "Artifact has the expected hash and is available locally {}",
                            expected_hash
                        );
                        Ok(true)
                    } else {
                        fs::remove_file(tmp_path.clone()).with_context(|| {
                            format!(
                                "Attempted to remove {} because its content has the wrong hash.",
                                tmp_path.to_str().unwrap()
                            )
                        })?;
                        warn!("Contents of artifact did not have the expected hash value of {}. The actual hash was {}:{}", expected_hash, expected_hash.algorithm, hex::encode(actual_hash));
                        Err(anyhow!("Contents of artifact did not have the expected hash value of {}. The actual hash was {}:{}", expected_hash, expected_hash.algorithm, hex::encode(actual_hash)))
                    }
                }
            };
        }

        fn file_creation_error(base_path: &PathBuf, error: std::io::Error) -> Result<bool, Error> {
            match error.kind() {
                io::ErrorKind::AlreadyExists => Ok(false),
                _ => Err(anyhow!(error)),
            }
            .with_context(|| format!("Error creating file {}", base_path.display()))
        }

        fn do_push<'b>(
            reader: &mut impl Read,
            expected_hash: &Hash,
            path: &PathBuf,
            out: File,
            hash_buffer: &'b mut [u8; 128],
        ) -> Result<&'b [u8], Error> {
            let mut buf_writer: BufWriter<File> = BufWriter::new(out);
            let mut digester = expected_hash.algorithm.digest_factory();
            let mut writer = WriteHashDecorator::new(&mut buf_writer, &mut digester);
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
            })?;

            // Check actual hash
            let buffer_slice: &mut [u8] = &mut hash_buffer[..digester.hash_size_in_bytes()];
            digester.finalize_hash(buffer_slice);
            Ok(buffer_slice)
        }

        /// Pull an artifact. The current implementation only looks in the local node's repository.
        /// A future
        ///
        pub fn pull_artifact(&self, hash: &Hash) -> Result<File, anyhow::Error> {
            info!(
                "An artifact is being pulled from the artifact manager {}",
                hash
            );
            let mut base_path: PathBuf = hash.base_file_path(self.repository_path);
            // for now all artifacts are unstructured
            base_path.set_extension("file");
            debug!("Pushing artifact from {}", base_path.display());
            File::open(base_path.as_path())
                .with_context(|| format!("{} not found.", base_path.display()))
        }
    }

    // Return a temporary file name that we will use for the file until we have verified that the
    // hash is correct. The temporary file name is guaranteed to be as unique as the hash and not
    // to be mistaken for a file whose name is its has code.
    //
    // The reason for doing this is so that a file whose actual hash is not equal to the expected
    // hash will not be found in the local repository from the time it is created and not fully
    // written until the time its hash is verified. After that, the file is renamed to its permanent
    // name that will match the actual hash value.
    fn tmp_path_from_base(base: &PathBuf) -> PathBuf {
        let mut tmp_buf = base.clone();
        let file_name: &OsStr = base.file_name().unwrap();
        tmp_buf.set_file_name(format!("X{}", file_name.to_str().unwrap()));
        tmp_buf
    }

    // return true if the given repository path leads to an accessible directory.
    fn is_accessible_directory(repository_path: &str) -> bool {
        match fs::metadata(repository_path) {
            Err(_) => false,
            Ok(metadata) => metadata.is_dir(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::artifact_manager::*;
    use anyhow::{anyhow, Context};
    use env_logger::Target;
    use log::LevelFilter;
    use std::fs;
    use std::io::Read;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use stringreader::StringReader;

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
    fn new_artifact_manager_with_valid_directory() {
        let dir_name = "tmpx";
        let _ignore = fs::remove_dir_all(dir_name);
        fs::create_dir(dir_name).expect(&format!("Unable to create temp directory {}", dir_name));
        let ok: bool = match ArtifactManager::new(dir_name) {
            Ok(_) => true,
            Err(_) => false,
        };
        let meta256 = fs::metadata(format!("{}/sha256", dir_name))
            .expect("unable to get metadata for sha256");
        assert!(meta256.is_dir());
        let meta512 = fs::metadata(format!("{}/sha512", dir_name))
            .expect("unable to get metadata for sha512");
        assert!(meta512.is_dir());
        fs::remove_dir_all(dir_name)
            .expect(&format!("unable to remove temp directory {}", dir_name));
        assert!(ok)
    }

    const TEST_ARTIFACT_DATA: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";
    const TEST_ARTIFACT_HASH: [u8; 32] = [
        0x2d, 0x8c, 0x2f, 0x6d, 0x97, 0x8c, 0xa2, 0x17, 0x12, 0xb5, 0xf6, 0xde, 0x36, 0xc9, 0xd3,
        0x1f, 0xa8, 0xe9, 0x6a, 0x4f, 0xa5, 0xd8, 0xff, 0x8b, 0x01, 0x88, 0xdf, 0xb9, 0xe7, 0xc1,
        0x71, 0xbb,
    ];
    const WRONG_ARTIFACT_HASH: [u8; 32] = [
        0x3d, 0xdc, 0x2f, 0x6d, 0x97, 0x8c, 0xa2, 0x17, 0x12, 0xb5, 0xf6, 0xde, 0x36, 0xc9, 0xd3,
        0x1f, 0xa8, 0xe9, 0x6a, 0x4f, 0xa5, 0xd8, 0xff, 0x8b, 0x01, 0x88, 0xdf, 0xb9, 0xe7, 0xc1,
        0x71, 0xbb,
    ];

    #[test]
    fn new_artifact_manager_with_bad_directory() {
        let ok: bool = match ArtifactManager::new("BoGuS") {
            Ok(_) => false,
            Err(_) => true,
        };
        assert!(ok)
    }

    #[test]
    fn happy_push_pull_test() -> Result<(), anyhow::Error> {
        let mut string_reader = StringReader::new(TEST_ARTIFACT_DATA);
        let hash = Hash::new(HashAlgorithm::SHA256, &TEST_ARTIFACT_HASH)?;
        let dir_name = tmp_dir_name("tmp");
        println!("tmp dir: {}", dir_name);
        fs::create_dir(dir_name.clone())
            .context(format!("Error creating directory {}", dir_name.clone()))?;
        let am =
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
    fn push_wrong_hash_test() -> Result<(), anyhow::Error> {
        let mut string_reader = StringReader::new(TEST_ARTIFACT_DATA);
        let hash = Hash::new(HashAlgorithm::SHA256, &WRONG_ARTIFACT_HASH)?;
        let dir_name = tmp_dir_name("tmpw");
        println!("tmp dir: {}", dir_name);
        fs::create_dir(dir_name.clone())
            .context(format!("Error creating directory {}", dir_name.clone()))?;
        let am =
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
    fn pull_nonexistant_test() -> Result<(), anyhow::Error> {
        let hash = Hash::new(HashAlgorithm::SHA256, &WRONG_ARTIFACT_HASH)?;
        let dir_name = tmp_dir_name("tmpr");
        println!("tmp dir: {}", dir_name);
        fs::create_dir(dir_name.clone())
            .context(format!("Error creating directory {}", dir_name.clone()))?;
        let am =
            ArtifactManager::new(dir_name.as_str()).context("Error creating ArtifactManager")?;
        let ok = match am.pull_artifact(&hash).context("Error from push_artifact") {
            Ok(_) => Err(anyhow!(
                "pull_artifact should have failed with nonexistant hash."
            )),
            Err(_) => Ok(()),
        };
        fs::remove_dir_all(dir_name.clone())
            .expect(&format!("unable to remove temp directory {}", dir_name));
        ok
    }

    fn tmp_dir_name(prefix: &str) -> String {
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
