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

use clap::{arg, command, AppSettings, ArgMatches, Command};

pub fn cli_parser() -> ArgMatches {
    command!()
        .subcommand_required(true)
        .arg_required_else_help(true)
        .global_setting(AppSettings::DeriveDisplayOrder)
        // Config subcommand
        .subcommands(vec![
            Command::new("config")
                .short_flag('c')
                .about("Pyrsia config commands")
                .arg_required_else_help(true)
                .disable_version_flag(true)
                .args(&[
                    arg!(--add      "Adds a node configuration"),
                    arg!(--edit     "Edits a node configuration"),
                    arg!(--remove   "Removes the stored node configuration").visible_alias("rm"),
                    arg!(--show     "Shows the stored node configuration"),
                ]),
            // Node subcommand
            Command::new("node")
                .short_flag('n')
                .about("Node commands")
                .arg_required_else_help(true)
                .disable_version_flag(true)
                .args(&[
                    arg!(--ping       "Pings configured pyrsia node"),
                    arg!(-s --status     "Shows node information"),
                    arg!(-l --list       "Shows list of connected Peers").visible_alias("ls"),
                ]),
        ])
        .get_matches()
}
