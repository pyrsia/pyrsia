use clap::{crate_authors, crate_description, crate_version, App, AppSettings, Arg, ArgMatches};

pub fn cli_parser() -> ArgMatches {
    App::new("pyrsia")
        .author(crate_authors!("\n"))
        .version(crate_version!())
        .about(crate_description!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        // Config subcommand
        .subcommand(
            App::new("config")
                .short_flag('c')
                .long_flag("config")
                .about("Pyrsia config commands")
                .setting(AppSettings::ArgRequiredElseHelp)
                .setting(AppSettings::AllowHyphenValues)
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
            App::new("node")
                .short_flag('n')
                .long_flag("node")
                .about("Node commands")
                .setting(AppSettings::ArgRequiredElseHelp)
                .setting(AppSettings::AllowHyphenValues)
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
