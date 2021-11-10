pub mod artifact_manager {
    use path::PathBuf;
    use std::fmt::Arguments;
    use std::fs;
    use std::fs::{File, OpenOptions};
    use std::io;
    use std::io::{BufWriter, IoSlice, Read, Write};
    use std::path;

    use anyhow::{anyhow, Context, Result};
    use crypto::digest::Digest;
    use crypto::sha2::Sha256;

    // We will provide implementations of this trait for each hash algorithm that we support.
    trait Digester {
        fn update_hash(&mut self, input: &[u8]);

        fn finalize_hash(&mut self, hash_buffer: &mut [u8]);

        fn hash_size_in_bytes(&self) -> u32;
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

        fn hash_size_in_bytes(&self) -> u32 {
            return 32;
        }
    }

    /// The types of hash that the artifact manager supports
    pub enum Hash {
        SHA256([u8; 32])
    }

    impl Hash {
        // The base file path (no extension on the file name) that will correspond to this hash
        fn base_file_path(&self, repo_dir: &'static str) -> PathBuf {
            fn build_base_path(repo_dir: &'static str, dir_name: &str, bytes: &[u8]) -> PathBuf {
                let mut buffer: PathBuf = PathBuf::from(repo_dir);
                buffer.push(dir_name);
                buffer.push(hex::encode(bytes));
                return buffer;
            }
            return match self {
                Hash::SHA256(bytes) => build_base_path(repo_dir, "sha256", bytes),
            };
        }

        fn digest_factory(&self) -> Box<dyn Digester> {
            return match self {
                Hash::SHA256(_) => Box::new(Sha256::new()),
            };
        }
    }

    // This is a decorator for the Write trait that allows the bytes written by the writer to be used to
// compute a hash
    struct WriteHashDecorator<'a> {
        writer: &'a mut dyn Write,
        digester: &'a mut dyn Digester,
    }

    impl<'a> WriteHashDecorator<'a> {
        fn new(writer: &'a mut impl Write, digester: &'a mut impl Digester) -> Self {
            return Self { writer, digester };
        }
    }

    // Decorator logic is supplied only for the methods that we expect to be called by io::copy
    impl<'a> Write for WriteHashDecorator<'a> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let result = self.writer.write(buf);
            match result { // hash just the number of bytes that were actually written. This may be less than the whole buffer.
                Ok(bytes_written) => self.digester.update_hash(&buf[0..bytes_written]),
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
    /// may be used in the future to indicate that the file has a particular internal structure that
    /// Pyrsia needs to know about.
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
        /// Returns true if it created the artifact local or false if the artifact already existed.
        pub fn push_artifact<'a>(&self, reader: &mut impl Read, expected_hash: &Hash) -> Result<bool, anyhow::Error> {
            let mut base_path = expected_hash.base_file_path(self.repository_path);
            // for now all artifacts are unstructured
            base_path.set_extension("file");

            let open_result = OpenOptions::new().write(true)
                .create_new(true)
                .open(base_path.as_path());
            return match open_result {
                Ok(out) => {
                    let mut writer: BufWriter<File> = BufWriter::new(out);
                    io::copy(reader, &mut writer).with_context(|| format!("Error while copying artifact contents to {}", base_path.display()))?;
                    writer.flush().with_context(|| format!("Error while flushing last of artifact contents to {}", base_path.display()))?;
                    Ok(true)
                }
                Err(error) => match error.kind() {
                    io::ErrorKind::AlreadyExists => Ok(false),
                    _ => Err(anyhow!(error))
                }
            };
        }

        /// Pull an artifact. The current implementation only looks in the local node's repository.
        /// A future
        ///
        pub fn pull_artifact(&self, _hash_algorithm: &str, _hash: &Hash) -> Result<&dyn io::Read, anyhow::Error> {
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
}

#[cfg(test)]
mod tests {
    use super::artifact_manager::*;

    #[test]
    fn new_artifact_manager_with_valid_directory() {
        let ok: bool = match ArtifactManager::new(".") {
            Ok(_) => true,
            Err(_) => false
        };
        assert!(ok)
    }

    #[test]
    fn new_artifact_manager_with_bad_directory() {
        let ok: bool = match ArtifactManager::new("BoGuS") {
            Ok(_) => false,
            Err(_) => true
        };
        assert!(ok)
    }
}
