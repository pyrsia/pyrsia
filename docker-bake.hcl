group "default" {
    targets = ["node"]
}

// Update the Cargo.lock file after making changes to Cargo.toml
target "updatelock" {
    target = "updatelock"
    output = ["./"]
}

// Build a Docker image for Pyrsia node
target "node" {
    target = "node"
    tags = ["pyrsia/node"]
}
