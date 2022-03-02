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

use clap::{crate_authors, crate_description, crate_version, Arg, ArgMatches, Command};

pub fn cli_parser() -> ArgMatches {
    Command::new("pyrsia")
        .author(crate_authors!("\n"))
        .version(crate_version!())
        .about(crate_description!())
        .subcommand_required(true)
        .arg_required_else_help(true)
        // Config subcommand
        .subcommand(
            Command::new("config")
                .short_flag('c')
                .long_flag("config")
                .about("Pyrsia config commands")
                .arg_required_else_help(true)
                .allow_hyphen_values(true)
                .arg(
                    Arg::new("add")
                        .short('a')
                        .long("add")
                        .help("Adds a node configuration")
                        .takes_value(true),
                )
                .arg(
                    Arg::new("edit")
                        .long("edit")
                        .short('e')
                        .help("Edits a node configuration")
                        .takes_value(true),
                )
                .arg(
                    Arg::new("remove")
                        .long("remove")
                        .short('r')
                        .help("Removes the stored node configuration"),
                )
                .arg(
                    Arg::new("show")
                        .long("show")
                        .short('s')
                        .help("Shows the stored node configuration"),
                ),
        )
        // Node subcommand
        .subcommand(
            Command::new("node")
                .short_flag('n')
                .long_flag("node")
                .about("Node commands")
                .arg_required_else_help(true)
                .allow_hyphen_values(true)
                .arg(
                    Arg::new("ping")
                        .short('p')
                        .long("ping")
                        .help("Ping configured pyrsia node"),
                )
                .arg(
                    Arg::new("status")
                        .long("status")
                        .short('s')
                        .help("Shows node information"),
                )
                .arg(
                    Arg::new("list")
                        .short('l')
                        .help("Shows list of connected Peers"),
                ),
        )
        .get_matches()
}
