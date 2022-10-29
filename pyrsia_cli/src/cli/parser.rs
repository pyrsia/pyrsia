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

use clap::{arg, command, crate_version, AppSettings, ArgGroup, ArgMatches, Command};
use const_format::formatcp;

pub fn cli_parser() -> ArgMatches {
    let version_string: &str = formatcp!("{} ({})", crate_version!(), env!("VERGEN_GIT_SHA"));
    command!()
        .arg_required_else_help(true)
        .global_setting(AppSettings::DeriveDisplayOrder)
        .propagate_version(false)
        // Config subcommand
        .subcommands(vec![
            Command::new("authorize")
                .about("Add an authorized node")
                .arg_required_else_help(true)
                .args(&[
                    arg!(-p --peer <PEER_ID>      "Peer ID of the node to authorize"),
                ]),
            Command::new("build")
                .short_flag('b')
                .about("Request a new build")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommands(vec![
                    Command::new("docker")
                        .about("Request a new build for a Docker image")
                        .arg_required_else_help(true)
                        .args(&[
                            arg!(--image <IMAGE> "The docker image to download (e.g. alpine:3.15.3 or alpine@sha256:1e014f84205d569a5cc3be4e108ca614055f7e21d11928946113ab3f36054801"),
                        ]),
                    Command::new("maven")
                        .about("Request a new build for a maven artifact")
                        .arg_required_else_help(true)
                        .args(&[
                            arg!(--gav <GAV> "The maven GAV (e.g. org.myorg:my-artifact:1.1.0)"),
                        ]),
                ]),
            Command::new("config")
                .short_flag('c')
                .about("Configure Pyrsia")
                .arg_required_else_help(true)
                .subcommands(vec![
                    Command::new("edit")
                        .short_flag('e')
                        .about("Edits a node configuration")
                        .arg_required_else_help(true)
                        .group(ArgGroup::new("pyrsia_node_config").multiple(true))
                        .args([
                            arg!(-H --host <HOST> "Hostname").group("pyrsia_node_config"),
                            arg!(-p --port <PORT> "Port number").group("pyrsia_node_config"),
                            arg!(-d --diskspace <DISK_SPACE> "Disk space to be allocated to pyrsia node").group("pyrsia_node_config"),
                        ])
                ]),
                /*.args(&[
                    arg!(-a --add      "Adds a node configuration"),
                    arg!(-e --edit     "Edits a node configuration"),
                    arg!(-r --remove   "Removes the stored node configuration").visible_alias("rm"),
                    arg!(-s --show     "Shows the stored node configuration"),
                ]),*/
            Command::new("inspect-log")
                .about("Show transparency logs")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommands(vec![
                    Command::new("docker")
                        .about("Show transparency logs for a Docker image")
                        .arg_required_else_help(true)
                        .args(&[
                            arg!(--image <IMAGE> "The docker image (e.g. alpine:3.15.3 or alpine@sha256:1e014f84205d569a5cc3be4e108ca614055f7e21d11928946113ab3f36054801"),
                        ]),
                    Command::new("maven")
                        .about("Show transparency logs for a maven artifact")
                        .arg_required_else_help(true)
                        .args(&[
                            arg!(--gav <GAV> "The maven GAV (e.g. org.myorg:my-artifact:1.1.0)"),
                        ]),
                ]),
            Command::new("list")
                .short_flag('l')
                .about("Show a list of connected peers"),
            Command::new("ping").about("Pings configured pyrsia node"),
            Command::new("status")
                .short_flag('s')
                .about("Show information about the Pyrsia node"),
        ])
        .version(version_string)
        .get_matches()
}
