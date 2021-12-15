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

use std::io::BufRead;

// Reads the first line from a BufRead
pub fn first_line<R>(mut rdr: R) -> String
where
    R: BufRead,
{
    let mut first_line: String = String::new();
    rdr.read_line(&mut first_line).expect("Unable to read line");
    first_line
}
