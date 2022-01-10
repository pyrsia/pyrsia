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
