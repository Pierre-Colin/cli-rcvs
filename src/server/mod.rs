mod data;
mod thread_pool;

use std::{
    collections::HashSet,
    fs,
    io::prelude::*,
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
};

use super::election_info;
use super::util;

fn handle_connection(mut stream: TcpStream,
                     election: &Mutex<rcvs::Election<String>>,
                     alternatives: &HashSet<String>,
                     data: &str,
                     peers: &Mutex<HashSet<std::net::IpAddr>>)
{
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
        let buffer = util::read_packet(&mut stream, 2048)
            .expect("Failed to receive ballot data");
        let ballot =
            data::parse_ballot(buffer.to_string(), &alternatives).unwrap();

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

pub fn run(matches: &clap::ArgMatches) {
    let filename = matches.value_of("ELECTION").unwrap();
    let data = Arc::new(data::squeeze_json(fs::read_to_string(filename)
        .expect("Failed to open election.json")));
    let structure: election_info::Election = serde_json::from_str(&data)
        .expect("Failed to parse JSON data");
    println!("{}", structure);

    let ip_address = format!("0.0.0.0:{}", matches.value_of("port").unwrap());
    println!("Opening TCP listener on {}", ip_address);
    let listener = TcpListener::bind(ip_address).unwrap();
    let peers = Arc::new(Mutex::new(HashSet::<std::net::IpAddr>::new()));

    let alternatives = Arc::new({
        let mut temp = HashSet::new();
        for x in structure.iter() {
            temp.insert(x.name());
        }
        temp
    });
    let election = Arc::new(Mutex::new(rcvs::Election::<String>::new()));

    let pool = thread_pool::ThreadPool::new(4).unwrap();

    for stream in listener.incoming() {
        let alternatives = Arc::clone(&alternatives);
        let data = Arc::clone(&data);
        let election = Arc::clone(&election);
        let peers = Arc::clone(&peers);

        pool.run(move || {
            handle_connection(stream.unwrap(),
                              &election,
                              &alternatives,
                              &data,
                              &peers);
        });
    }

    println!("Closing TCP listener");
}
