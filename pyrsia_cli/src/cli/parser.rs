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

use clap::{arg, command, crate_version, AppSettings, ArgMatches, Command};
use const_format::formatcp;

pub fn cli_parser() -> ArgMatches {
    let version_string: &str = formatcp!("{} ({})", crate_version!(), env!("VERGEN_GIT_SHA"));
    command!()
        .arg_required_else_help(true)
        .global_setting(AppSettings::DeriveDisplayOrder)
        .propagate_version(false)
        // Config subcommand
        .subcommands(vec![
            Command::new("config")
                .short_flag('c')
                .about("Pyrsia config commands")
                .arg_required_else_help(true)
                .args(&[
                    arg!(-a --add      "Adds a node configuration"),
                    arg!(-e --edit     "Edits a node configuration"),
                    arg!(-r --remove   "Removes the stored node configuration").visible_alias("rm"),
                    arg!(-s --show     "Shows the stored node configuration"),
                ]),
            Command::new("list")
                .short_flag('l')
                .about("Shows list of connected Peers"),
            Command::new("ping").about("Pings configured pyrsia node"),
            Command::new("status")
                .short_flag('s')
                .about("Shows node information"),
        ])
        .version(version_string)
        .get_matches()
}
