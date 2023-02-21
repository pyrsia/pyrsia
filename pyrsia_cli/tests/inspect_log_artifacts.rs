use std::{thread, time};
use testcontainers::images;
use assert_cmd::Command;
use predicates::prelude::predicate;
use reqwest::StatusCode;
use testcontainers::clients::Cli;
use testcontainers::core::WaitFor;

const NAME: &str = "pyrsia/test_node";
const TAG: &str = "latest";
const DEF_PORT: u16 = 7888;
const DEF_TIMEOUT: u64 = 1500;

#[test]
fn run_pyrsia_node_with_status_checks() {
    let docker = Cli::default();
    let generic = images::generic::GenericImage::new(NAME, TAG)
        .with_entrypoint("/tmp/test-listen-only-node-entrypoint.sh")
        .with_exposed_port(DEF_PORT)
        .with_wait_for(WaitFor::millis(DEF_TIMEOUT));

    let node = docker.run(generic);
    let port = node.get_host_port_ipv4(DEF_PORT);

    let mut res = reqwest::blocking::get(&format!("http://127.0.0.1:{}/status", port));

    let mut count: u8 = 0;
    while res.is_err() && count < 25 {
        thread::sleep(time::Duration::from_micros(DEF_TIMEOUT));

        res = reqwest::blocking::get(&format!("http://0.0.0.0:{}/status", port));
        count += 1;
    }

    assert_eq!(res.unwrap().status(), StatusCode::OK);

    let client = reqwest::blocking::Client::new();
    let resp = client.post(&format!("http://0.0.0.0:{}/inspect/docker", port))
        .body("{\"image\": \"alpine:3.16\"}")
        .send()
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    Command::cargo_bin("pyrsia").unwrap()
        .arg("-c")
        .arg("-e")
        .arg("-H")
        .arg("0.0.0.0")
        .arg("-p")
        .arg(port.to_string())
        .assert()
        .stdout(predicate::str::diff("Node configuration saved !!\n"));

    let res = Command::cargo_bin("pyrsia").unwrap()
        .arg("inspect-log")
        .arg("docker")
        .arg("--image")
        .arg("alpine:3.16")
        .assert()
        .stdout(predicate::str::diff("[]\n"));

    println!("Pyrsia: {}",
             String::from_utf8_lossy(res.get_output().stdout.as_slice())
    );
}
