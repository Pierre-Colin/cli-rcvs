use std::{
    collections::HashSet,
    fs,
    io::prelude::*,
    net::TcpListener,
};

use super::election_info;
use super::util;

fn squeeze_json(string: String) -> String {
    let mut result = String::with_capacity(string.len());
    let mut indent = false;
    let mut last_comma = false;
    for c in string.chars() {
        if indent && !c.is_whitespace() {
            result.push(c);
            indent = false;
            last_comma = c == ',';
        } else if !indent && c != '\n' {
            result.push(c);
            last_comma = c == ',';
        } else if c == '\n' {
            if last_comma { result.push(' '); }
            indent = true;
            last_comma = false;
        }
    }
    result
}

#[derive(Debug)]
enum ParseState {
    Begin,
    OpenEntry,
    Name(String),
    Low(String, u64),
    High(String, u64, u64),
    CloseEntry,
    Done,
}

fn parse_ballot(s: String, alternatives: &HashSet<String>)
    -> Option<rcvs::Ballot<String>>
{
    let mut ballot = rcvs::Ballot::new();
    let mut state = ParseState::Begin;
    for c in s.chars() {
        if c.is_whitespace() { continue; }
        match state {
            ParseState::Begin => {
                if c != '[' {
                    return None;
                } else {
                    state = ParseState::OpenEntry;
                }
            },
            ParseState::OpenEntry => {
                if c == ']' {
                    state = ParseState::Done;
                } else if c != '(' {
                    return None;
                } else {
                    state = ParseState::Name(String::new());
                }
            },
            ParseState::Name(s) => {
                if c == ',' {
                    if alternatives.contains(&s) {
                        state = ParseState::Low(s, 0);
                    } else {
                        return None;
                    }
                } else if c.is_alphanumeric() {
                    let mut temp = s;
                    temp.push(c);
                    state = ParseState::Name(temp);
                } else {
                    return None;
                }
            },
            ParseState::Low(s, n) => {
                if c == ',' {
                    state = ParseState::High(s, n, 0);
                } else if let Some(d) = c.to_digit(10) {
                    state = ParseState::Low(s, 10 * n + d as u64);
                } else {
                    return None;
                }
            },
            ParseState::High(s, m, n) => {
                if c == ')' {
                    ballot.insert(s.to_string(), m, n);
                    state = ParseState::CloseEntry;
                } else if let Some(d) = c.to_digit(10) {
                    state = ParseState::High(s, m, 10 * n + d as u64);
                } else {
                    return None;
                }
            },
            ParseState::CloseEntry => {
                if c == ']' {
                    state = ParseState::Done;
                } else if c == ',' {
                    state = ParseState::OpenEntry;
                } else {
                    return None;
                }
            },
            ParseState::Done => { return None; },
        };
    }
    Some(ballot)
}

pub fn run(matches: &clap::ArgMatches) {
    let filename = matches.value_of("ELECTION").unwrap();
    let data = squeeze_json(fs::read_to_string(filename)
        .expect("Failed to open election.json"));
    let structure: election_info::Election = serde_json::from_str(&data)
        .expect("Failed to parse JSON data");
    println!("{}", structure);

    let ip_address = format!("0.0.0.0:{}", matches.value_of("port").unwrap());
    println!("Opening TCP listener on {}", ip_address);
    let listener = TcpListener::bind(ip_address).unwrap();
    let mut peers = HashSet::<std::net::IpAddr>::new();

    let alternatives = {
        let mut temp = HashSet::new();
        for x in structure.iter() {
            temp.insert(x.name());
        }
        temp
    };
    let mut election = rcvs::Election::<String>::new();

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();
        let ip = stream.peer_addr().unwrap().ip();
        if peers.contains(&ip) {
            println!("Connection from {} rejected", ip);
            stream.write(b"VOTED").unwrap();
            stream.flush().unwrap();
        } else {
            println!("Connection from {} granted", ip);
            stream.write(data.as_bytes()).unwrap();
            stream.flush().unwrap();
            let buffer = util::read_packet(&mut stream, 2048)
                .expect("Failed to receive ballot data");

            let ballot = parse_ballot(buffer.to_string(), &alternatives).unwrap();
            election.cast(ballot);
            //peers.insert(ip);
        }

        println!("{}", election);
    }

    println!("Closing TCP listener");
}
