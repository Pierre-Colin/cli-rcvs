use std::{error::Error, fmt, io::prelude::*, iter, net::TcpStream};

use dialoguer::{theme::ColorfulTheme, Input, Select};

use super::election_info;
use super::util;

#[derive(Debug)]
enum ClientError {
    ParseError,
    BadRank,
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.description())
    }
}

impl Error for ClientError {
    fn description(&self) -> &str {
        match self {
            ClientError::ParseError => "Parsing failed",
            ClientError::BadRank => "Rank is incorrect",
        }
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

fn format_choice(entry: &(election_info::Choice, Option<rcvs::Rank>)) -> String {
    let (alternative, rank) = entry;
    if let Some(rank) = rank {
        format!(
            "{}, {}: {}",
            alternative.name(),
            rank,
            alternative.description()
        )
    } else {
        format!(
            "{}, UNRANKED: {}",
            alternative.name(),
            alternative.description()
        )
    }
}

fn parse_rank(string: String) -> Result<rcvs::Rank, ClientError> {
    let mut in_high = false;
    let mut low = 0u64;
    let mut high = 0u64;
    for c in string.chars() {
        if let Some(d) = c.to_digit(10) {
            let var = if in_high { &mut high } else { &mut low };
            *var = 10 * *var + d as u64;
        } else if c == '-' {
            if !in_high {
                in_high = true;
            } else {
                return Err(ClientError::ParseError);
            }
        } else {
            return Err(ClientError::ParseError);
        }
    }
    if !in_high {
        Ok(rcvs::Rank::new(low, low).unwrap())
    } else if let Some(rank) = rcvs::Rank::new(low, high) {
        Ok(rank)
    } else {
        Err(ClientError::BadRank)
    }
}

fn vec_to_ballot(
    alternatives: Vec<(election_info::Choice, Option<rcvs::Rank>)>,
) -> rcvs::Ballot<String> {
    let mut ballot = rcvs::Ballot::<String>::new();
    for (choice, rank) in alternatives.into_iter() {
        if let Some(rank) = rank {
            ballot.insert(choice.name().to_string(), rank.low(), rank.high());
        }
    }
    ballot
}

fn ballot_wizzard(structure: election_info::Election) -> rcvs::Ballot<String> {
    let mut alternatives: Vec<(election_info::Choice, Option<rcvs::Rank>)> =
        structure.iter().map(|x| (x.clone(), None)).collect();
    let mut done = false;
    while !done {
        let choices: Vec<String> = iter::once("Done".to_string())
            .chain(alternatives.iter().map(format_choice))
            .collect();
        if let Ok(i) = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Pick an alternative")
            .default(0)
            .items(&choices)
            .interact()
        {
            if i > 0 {
                println!("{}", choices[i]);
                let rank: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Enter new rank (U for unranked)")
                    .interact()
                    .unwrap();
                if rank == "U" {
                    alternatives[i - 1].1 = None;
                } else {
                    match parse_rank(rank) {
                        Ok(rank) => alternatives[i - 1].1 = Some(rank),
                        Err(what) => eprintln!("error: {}", what),
                    }
                }
            } else {
                done = true;
            }
        }
    }
    vec_to_ballot(alternatives)
}

pub fn run(matches: &clap::ArgMatches) {
    let ip_address = matches.value_of("SERVER").unwrap();
    match TcpStream::connect(ip_address) {
        Ok(mut stream) => {
            let data = util::read_packet(&mut stream, 2048).expect("Failed to read JSON data");
            if data == "VOTED" {
                println!("Already voted.");
            } else {
                let structure: election_info::Election =
                    serde_json::from_str(&data).expect("Failed to parse JSON data");
                println!("{}", structure);
                let ballot = ballot_wizzard(structure);
                let ballot = election_info::serialize_ballot(ballot);
                println!("{}", ballot);
                stream
                    .write(&ballot.into_bytes())
                    .expect("Failed to send ballot data");
                stream.flush().unwrap();
            }
        }
        Err(what) => {
            eprintln!("error: {}", what);
            std::process::exit(1);
        }
    }
}
