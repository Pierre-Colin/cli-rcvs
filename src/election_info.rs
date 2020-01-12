use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Choice {
    name: String,
    description: String,
}

impl fmt::Display for Choice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.description)
    }
}

impl Choice {
    pub fn name(&self) -> String {
        self.name.to_string()
    }

    pub fn description(&self) -> String {
        self.description.to_string()
    }
}

#[derive(Serialize, Deserialize)]
pub struct Election {
    title: String,
    question: String,
    alternatives: Vec<Choice>,
}

impl fmt::Display for Election {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "\t{}\n{}", self.title, self.question)?;
        for alternative in self.alternatives.iter() {
            writeln!(f, "\t{}", alternative)?;
        }
        Ok(())
    }
}

impl Election {
    pub fn iter(&self) -> std::slice::Iter<Choice> {
        self.alternatives.iter()
    }
}

pub fn serialize_ballot(ballot: rcvs::Ballot<String>) -> String {
    let mut r = "[".to_string();
    let mut first = true;
    for (alternative, rank) in ballot.into_iter() {
        if first {
            first = false;
        } else {
            r.push_str(", ");
        }
        r.push_str(&format!(
            "({}, {}, {})",
            alternative,
            rank.low(),
            rank.high()
        ));
    }
    r.push(']');
    r
}
