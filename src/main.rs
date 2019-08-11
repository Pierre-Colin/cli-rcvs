mod election_info;
mod client;
mod server;
mod util;

fn main() {
    let matches = clap::App::new("CLI Randomized Condorcet Voting System")
        .version("0.1")
        .author("Pierre Colin")
        .about("Carries out RCVS elections over TCP")
        .subcommand(clap::SubCommand::with_name("server")
            .about("Hosts the election over TCP")
            .arg(clap::Arg::with_name("port")
                .long("port")
                .short("p")
                .default_value("7878")
                .help("Sets the port to host the election on")
            )
            .arg(clap::Arg::with_name("ELECTION")
                .required(true)
                .default_value("election.json")
                .help("Sets the election JSON file")
            )
        )
        .subcommand(clap::SubCommand::with_name("client")
            .about("Casts vote into remote server")
            .arg(clap::Arg::with_name("SERVER")
                .required(true)
                .help("IP address and port of server")
            )
        ).get_matches();
    match matches.subcommand() {
        ("server", Some(sub_m)) => server::run(sub_m),
        ("client", Some(sub_m)) => client::run(sub_m),
        _ => {
            eprintln!("error: Invalid subcommand");
            std::process::exit(1);
        },
    }
}
