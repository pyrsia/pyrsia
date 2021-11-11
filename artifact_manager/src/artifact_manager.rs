///
/// # Artifact Manager
/// Module for managing artifacts. It manages a local collection of artifacts and is responsible
/// getting artifacts from other nodes when they are not present locally.
///
pub mod artifact_manager {
    use path::PathBuf;
    use std::fmt::{Arguments, Formatter};
    use std::fs;
    use std::fs::{File, OpenOptions};
    use std::io;
    use std::io::{BufWriter, IoSlice, Read, Write};
    use std::path;

    use anyhow::{anyhow, Context, Error, Result};
    use crypto::digest::Digest;
    use crypto::sha2::Sha256;

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
            while i < 32 {
                hash_buffer[i] = hash_array[i];
                i += 1;
            }
        }

        fn hash_size_in_bytes(&self) -> usize {
            return 32;
        }
    }

    /// The types of hash that the artifact manager supports
    pub enum Hash {
        SHA256([u8; 32])
    }

    impl Hash {
        ///////////////////////////////////////////////////////////////
        // Add another constant for each new member of Hash.         //
        // Don't forget to add the contstant to the following array. //
        const SHA256_DIR: &'static str = "sha256";

        ////////////////////////////////////////////////////////////////////////
        // When there is a new Hash enum sure to add the directory names here //
        ////////////////////////////////////////////////////////////////////////
        const ALGORITHM_NAMES: [&'static str;1] = [Hash::SHA256_DIR];

        fn ensure_directories_for_hash_algorithms_exist(repository_path: &str) -> Result<(), anyhow::Error >{
            let mut path_buf = PathBuf::new();
            path_buf.push(repository_path);
            for name in Hash::ALGORITHM_NAMES {
                let mut this_buf = path_buf.clone();
                this_buf.push(name);
                fs::create_dir_all( this_buf.as_os_str()).with_context(|| format!("Error creating directory {}", this_buf.display()))?;
            }
            Ok(())

        }

        fn encode_bytes_as_file_name(bytes: &[u8]) -> String {
            hex::encode(bytes)
        }

        // The base file path (no extension on the file name) that will correspond to this hash
        fn base_file_path(&self, repo_dir: &str) -> PathBuf {
            fn build_base_path(repo_dir: &str, dir_name: &str, bytes: &[u8]) -> PathBuf {
                let mut buffer: PathBuf = PathBuf::from(repo_dir);
                buffer.push(dir_name);
                buffer.push(Hash::encode_bytes_as_file_name(bytes));
                return buffer;
            }
            return match self {
                Hash::SHA256(bytes) => build_base_path(repo_dir, self.algorithm_directory_name(), bytes),
            }
        }

        fn digest_factory(&self) -> impl Digester {
            return match self {
                Hash::SHA256(_) => Sha256::new(),
            };
        }

        fn algorithm_directory_name(&self) -> &'static str {
            return match self {
                Hash::SHA256(_) => Hash::SHA256_DIR
            }
        }
    }

    impl std::fmt::Display for Hash {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                Hash::SHA256(bytes) => write!(f, "sha256({})", Hash::encode_bytes_as_file_name(bytes))
            }
        }
    }

    // This is a decorator for the Write trait that allows the bytes written by the writer to be
    // used to compute a hash
    struct WriteHashDecorator<'a> {
        writer: &'a mut dyn Write,
        digester: &'a mut dyn Digester,
    }

    impl<'a> WriteHashDecorator<'a> {
        fn new(writer: &'a mut impl Write, digester: &'a mut impl Digester) -> Self {
            return WriteHashDecorator { writer, digester };
        }
    }

    // Decorator logic is supplied only for the methods that we expect to be called by io::copy
    impl<'a> Write for WriteHashDecorator<'a> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let result = self.writer.write(buf);
            match result { // hash just the number of bytes that were actually written. This may be less than the whole buffer.
                Ok(bytes_written) => self.digester.update_hash(&buf[..bytes_written]),
                _ => {}
            }
            return result;
        }

        fn write_vectored(&mut self, _bufs: &[IoSlice<'_>]) -> io::Result<usize> {
            unimplemented!()
        }

        // fn is_write_vectored(&self) -> bool {
        //     unimplemented!()
        // }

        fn flush(&mut self) -> io::Result<()> {
            return self.writer.flush();
        }

        fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
            let result = self.writer.write(buf);
            match result { // hash just the number of bytes that were actually written. This may be less than the whole buffer.
                Ok(_) => self.digester.update_hash(buf),
                _ => {}
            }
            return Ok(());
        }

        // fn write_all_vectored(&mut self, bufs: &mut [IoSlice<'_>]) -> io::Result<()> {
        //     unimplemented!()
        // }

        fn write_fmt(&mut self, _fmt: Arguments<'_>) -> io::Result<()> {
            unimplemented!()
        }

        fn by_ref(&mut self) -> &mut Self where Self: Sized {
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
        /// Returns true if it created the artifact local or false if the artifact already existed.
        pub fn push_artifact(&self, reader: &mut impl Read, expected_hash: &Hash) -> Result<bool, anyhow::Error> {
            let mut base_path = expected_hash.base_file_path(self.repository_path);
            // for now all artifacts are unstructured
            base_path.set_extension("file");

            let open_result = OpenOptions::new().write(true)
                .create_new(true)
                .open(base_path.as_path());
            return match open_result {
                Ok(out) => {
                    println!("hash is {}", expected_hash);
                    Self::do_push(reader, expected_hash, base_path, out).with_context(|| format!("Error writing contents of {}", expected_hash))
                }
                Err(error) => match error.kind() {
                    io::ErrorKind::AlreadyExists => Ok(false),
                    _ => Err(anyhow!(error))
                }.with_context(|| format!("Error creating file {}", base_path.display()))
            };
        }

        fn do_push(reader: &mut impl Read, expected_hash: &Hash, base_path: PathBuf, out: File) -> Result<bool, Error> {
            let mut buf_writer: BufWriter<File> = BufWriter::new(out);
            let digester = &mut expected_hash.digest_factory();
            let mut writer = WriteHashDecorator::new(&mut buf_writer, digester);
            io::copy(reader, &mut writer).with_context(|| format!("Error while copying artifact contents to {}", base_path.display()))?;
            writer.flush().with_context(|| format!("Error while flushing last of artifact contents to {}", base_path.display()))?;
            let mut hash_buffer: [u8;128] = [0; 128];
            let buffer_slice: &mut[u8] = &mut hash_buffer[..digester.hash_size_in_bytes()];
            digester.finalize_hash( buffer_slice);

            Ok(true)
        }

        /// Pull an artifact. The current implementation only looks in the local node's repository.
        /// A future
        ///
        pub fn pull_artifact(&self, _hash_algorithm: &str, _hash: &Hash) -> Result<&dyn io::Read, anyhow::Error> {
            unimplemented!();
        }
    }

    // return true if the given repository path leads to an accessible directory.
    fn is_accessible_directory(repository_path: &str) -> bool {
        match fs::metadata(repository_path) {
            Err(_) => false,
            Ok(metadata) => metadata.is_dir()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::artifact_manager::*;
    use stringreader::StringReader;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use anyhow::Context;

    #[test]
    fn new_artifact_manager_with_valid_directory() {
        let ok: bool = match ArtifactManager::new(".") {
            Ok(_) => true,
            Err(_) => false
        };
        assert!(ok)
    }

    const TEST_ARTIFACT_DATA: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";
    const TEST_ARTIFACT_HASH: [u8;32] = [0x2d, 0x8c, 0x2f, 0x6d, 0x97, 0x8c, 0xa2, 0x17, 0x12, 0xb5, 0xf6, 0xde, 0x36, 0xc9, 0xd3, 0x1f, 0xa8, 0xe9, 0x6a, 0x4f, 0xa5, 0xd8, 0xff, 0x8b, 0x01, 0x88, 0xdf, 0xb9, 0xe7, 0xc1, 0x71, 0xbb];

    #[test]
    fn new_artifact_manager_with_bad_directory() {
        let ok: bool = match ArtifactManager::new("BoGuS") {
            Ok(_) => false,
            Err(_) => true
        };
        assert!(ok)
    }

    #[test]
    fn happy_push_test() -> Result<(), anyhow::Error> {
        let mut string_reader = StringReader::new(TEST_ARTIFACT_DATA);
        let hash = Hash::SHA256(TEST_ARTIFACT_HASH);
        let dir_name = tmp_dir_name();
        println!("tmp dir: {}", dir_name);
        fs::create_dir(dir_name.clone()).context(format!("Error creating directory {}", dir_name.clone()))?;
        let am = ArtifactManager::new(dir_name.as_str()).context("Error creating ArtifactManager")?;
        am.push_artifact(&mut string_reader, &hash).context("Error from push_artifact")?;
        fs::remove_dir_all(dir_name.clone()).context(format!("Error removing directory {}", dir_name))?;
        Ok(())
    }

    fn tmp_dir_name() -> String {
        return format!("{}{}","tmp",
            SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis());
    }
}
