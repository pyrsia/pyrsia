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

use error_chain::error_chain;

error_chain! {
    errors {
        FileIO {
            description("Error performing file IO")
                display("Unable to perform file IO")
        }
        Serialization {
            description("Error serializing structure")
                display("Unable to serialize structure")
        }
        Deserialization {
            description("Error deserializing structure")
                display("Unable to deserialize structure")
        }
        Websocket {
            description("Failure handling websocket client")
                display("Unable to handle websocket client")
        }
        HTTP {
            description("Failure in HTTP transfer")
                display("Unable to complete HTTP transfer")
        }
        Parse {
            description("Failure parsing input")
            display("Could not parse input")
        }
    }
}
