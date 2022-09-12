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

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
pub mod tests {
    use std::env;
    use std::fs;
    use std::path;

    pub fn setup() -> path::PathBuf {
        let tmp_dir = tempfile::tempdir()
            .expect("could not create temporary directory")
            .into_path();

        env::set_var("PYRSIA_ARTIFACT_PATH", tmp_dir.to_str().unwrap());
        env::set_var("DEV_MODE", "on");

        tmp_dir
    }

    pub fn teardown(tmp_dir: path::PathBuf) {
        if tmp_dir.exists() {
            fs::remove_dir_all(&tmp_dir)
                .unwrap_or_else(|_| panic!("unable to remove test directory {:?}", tmp_dir));
        }

        env::remove_var("PYRSIA_ARTIFACT_PATH");
        env::remove_var("DEV_MODE");
    }
}
