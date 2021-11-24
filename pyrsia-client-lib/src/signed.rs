/// This trait should be implemented by all structs that contain signed data.<br><br>
///
/// Pyrsia needs to manage a variety of data related to software artifacts. It will store all of
/// this data as JSON.  The reason for using JSON is to promote interoperability. If Pyrsia is
/// successful people will write their own implementations of Pyrsia nodes. For this reason, we
/// choose standard [JSON](https://www.json.org/json-en.html) .<br>
///<br>
/// Pyrsia nodes may store a substantial amount of data. Even if there are gigabytes of data, we
/// will expect the data to be found and retrieved quickly. For this reason we plan to put JSON
/// documents in a document store that will index them by values in their fields. In order for a
/// document store to be able to index JSON documents, they must be in the form of clear text,
/// rather than being in an encoded or compressed form.<br>
///<BR>
/// All JSON that Pyrsia manages must be signed so that we can attribute it to a source and be
/// confident that it has not been modified since it was signed. Based on the above rationale, we
/// have these requirements for how to sign JSON documents:
///
/// * We must be able to sign standard JSON that is represented as a UFT-8 string.
/// * After we have signed a JSON document, it must still be standard JSON.
/// * The fields and overall structure of the original JSON document must be unchanged by the fact that it is signed.
///
///
pub trait Signed {}
