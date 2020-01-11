mod data;
mod thread_pool;

use std::{
    collections::HashSet,
    fs,
    io::prelude::*,
    net::{TcpListener, TcpStream},
    sync::{atomic, Arc, Mutex},
    thread,
};

use dialoguer::{theme::ColorfulTheme, Select};

use super::election_info;
use super::util;

fn handle_connection(
    mut stream: TcpStream,
    election: &Mutex<rcvs::Election<String>>,
    alternatives: &HashSet<String>,
    data: &str,
    peers: &Mutex<HashSet<std::net::IpAddr>>,
) {
    let ip = stream.peer_addr().unwrap().ip();
    let already_voted = peers.lock().unwrap().contains(&ip);
    if already_voted {
        println!("Connection from {} rejected", ip);
        stream.write(b"VOTED").unwrap();
        stream.flush().unwrap();
    } else {
        println!("Connection from {} granted", ip);
        stream.write(data.as_bytes()).unwrap();
        stream.flush().unwrap();
        let buffer = util::read_packet(&mut stream, 2048).expect("Failed to receive ballot data");
        let ballot = data::parse_ballot(buffer.to_string(), &alternatives).unwrap();

        let mut election = election.lock().unwrap();
        if peers.lock().unwrap().insert(ip) {
            election.cast(ballot);
        } else {
            println!("Double-vote detected for {}", ip);
            stream.write(b"VOTED").unwrap();
            stream.flush().unwrap();
        }

        println!("{}", election);
    }
}

fn status(election: &Mutex<rcvs::Election<String>>) {
    let g = {
        let election = election.lock().unwrap();
        println!("{}", election);
        election.get_duel_graph()
    };

    println!("{}", g);

    if let Some(x) = g.get_source() {
        println!("Condorcet winner: {}", x);
    }

    if let Some(x) = g.get_sink() {
        println!("Condorcet loser: {}", x);
    }

    match g.get_optimal_strategy() {
        Ok(p) => println!(
            "Optimal strategy: {}\nWinner: {}",
            p,
            p.play(&mut rand::thread_rng()).unwrap()
        ),
        Err(what) => eprintln!("error: Failed to compute strategy: {}", what),
    }
}

fn list_peers(peers: &HashSet<std::net::IpAddr>) {
    println!("Peers:");
    for ip in peers {
        println!("\t- {}", ip);
    }
}

pub fn run(matches: &clap::ArgMatches) {
    let filename = matches.value_of("ELECTION").unwrap();
    let data = Arc::new(data::squeeze_json(
        fs::read_to_string(filename).expect("Failed to open election.json"),
    ));
    let structure: election_info::Election =
        serde_json::from_str(&data).expect("Failed to parse JSON data");
    println!("{}", structure);

    let ip_address = format!("0.0.0.0:{}", matches.value_of("port").unwrap());
    println!("Opening TCP listener on {}", ip_address);
    let listener = TcpListener::bind(ip_address).unwrap();

    // Set listener to nonblocking if possible
    listener.set_nonblocking(true).unwrap_or_else(|_| {
        eprintln!(
            "error: Failed to set listener to nonblocking.\n
                   \tThere will be no way of gracefully exit the server."
        );
    });

    let peers = Arc::new(Mutex::new(HashSet::<std::net::IpAddr>::new()));
    let peers_cloned = Arc::clone(&peers);

    let alternatives = Arc::new({
        let mut temp = HashSet::new();
        for x in structure.iter() {
            temp.insert(x.name());
        }
        temp
    });
    let election = Arc::new(Mutex::new(rcvs::Election::<String>::new()));
    for alternative in alternatives.iter() {
        election.lock().unwrap().add_alternative(&alternative);
    }
    let election_cloned = Arc::clone(&election);

    let stop = Arc::new(atomic::AtomicBool::new(false));
    let stop_cloned = Arc::clone(&stop);

    let canary = Arc::new(atomic::AtomicBool::new(false));
    let canary_cloned = Arc::clone(&canary);

    thread::spawn(move || {
        let pool = thread_pool::ThreadPool::new(4, canary_cloned).unwrap();

        loop {
            if let Ok((stream, _)) = listener.accept() {
                let alternatives = Arc::clone(&alternatives);
                let data = Arc::clone(&data);
                let election = Arc::clone(&election_cloned);
                let peers = Arc::clone(&peers_cloned);

                pool.run(move || {
                    handle_connection(stream, &election, &alternatives, &data, &peers);
                });
            } else if stop_cloned.load(atomic::Ordering::Relaxed) {
                break;
            }
            thread::yield_now();
        }
    });

    while !stop.load(atomic::Ordering::Relaxed) {
        if let Ok(i) = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Server options")
            .default(0)
            .items(&["Election status", "Peer list", "Close election"])
            .interact()
        {
            match i {
                0 => status(&election),
                1 => list_peers(&peers.lock().unwrap()),
                2 => stop.store(true, atomic::Ordering::Relaxed),
                _ => eprintln!("error: Incorrect choice"),
            }
        }
    }

    println!("Waiting for pending connections to end...");
    while !canary.load(atomic::Ordering::Relaxed) {
        thread::yield_now();
    }

    println!("\n\tELECTION RESULTS\n");
    status(&election);
}
