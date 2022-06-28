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

use std::env;

pub fn read_var(variable_name: &str, default_value: &str) -> String {
    match env::var(variable_name) {
        Ok(v) => {
            let tr = v.trim();
            if !tr.is_empty() {
                String::from(tr)
            } else {
                String::from(default_value)
            }
        }
        Err(_err) => String::from(default_value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_present() {
        env::set_var("ENV_VAR_PRESENT", "on");

        assert_eq!("on", read_var("ENV_VAR_PRESENT", "off"));

        env::remove_var("ENV_VAR_PRESENT");
    }

    #[test]
    fn test_value_present_trim() {
        env::set_var("ENV_VAR_PRESENT_TRIM", "on ");

        assert_eq!("on", read_var("ENV_VAR_PRESENT_TRIM", "off"));

        env::remove_var("ENV_VAR_PRESENT_TRIM");
    }

    #[test]
    fn test_value_empty() {
        env::set_var("ENV_VAR_EMPTY", "");

        assert_eq!("off", read_var("ENV_VAR_EMPTY", "off"));

        env::remove_var("ENV_VAR_EMPTY");
    }

    #[test]
    fn test_value_absent() {
        assert_eq!("absent", read_var("ENV_VAR_ABSENT", "absent"));
    }
}
