use std::collections::HashSet;

pub fn squeeze_json(string: String) -> String {
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
            if last_comma {
                result.push(' ');
            }
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

pub fn parse_ballot(s: String, alternatives: &HashSet<String>) -> Option<rcvs::Ballot<String>> {
    let mut ballot = rcvs::Ballot::new();
    let mut state = ParseState::Begin;
    for c in s.chars() {
        if c.is_whitespace() {
            continue;
        }
        match state {
            ParseState::Begin => {
                if c != '[' {
                    return None;
                } else {
                    state = ParseState::OpenEntry;
                }
            }
            ParseState::OpenEntry => {
                if c == ']' {
                    state = ParseState::Done;
                } else if c != '(' {
                    return None;
                } else {
                    state = ParseState::Name(String::new());
                }
            }
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
            }
            ParseState::Low(s, n) => {
                if c == ',' {
                    state = ParseState::High(s, n, 0);
                } else if let Some(d) = c.to_digit(10) {
                    state = ParseState::Low(s, 10 * n + d as u64);
                } else {
                    return None;
                }
            }
            ParseState::High(s, m, n) => {
                if c == ')' {
                    ballot.insert(s.to_string(), m, n);
                    state = ParseState::CloseEntry;
                } else if let Some(d) = c.to_digit(10) {
                    state = ParseState::High(s, m, 10 * n + d as u64);
                } else {
                    return None;
                }
            }
            ParseState::CloseEntry => {
                if c == ']' {
                    state = ParseState::Done;
                } else if c == ',' {
                    state = ParseState::OpenEntry;
                } else {
                    return None;
                }
            }
            ParseState::Done => {
                return None;
            }
        };
    }
    Some(ballot)
}
