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

pub fn parse_ballot(s: String, alternatives: &HashSet<String>) -> Option<rcvs::Ballot<String>> {
    let re = regex::Regex::new(
        r" *\[ *(?P<inner>\( *([A-Za-z]+) *, *(\d+) *, *(\d+) *\)( *, *\( *[A-Za-z]+ *, *\d+ *, *\d+ *\))*)?\] *",
    )
    .expect("Failed to compile regular expression");
    let inner =
        regex::Regex::new(r"\((?P<alternative>[A-Za-z]+) *, *(?P<lower>\d+) *, *(?P<upper>\d+)\)")
            .expect("Failed to compile regular expression");
    let captures = re.captures(&s).unwrap();
    let capture = captures.name("inner")?;
    let mut ballot = rcvs::Ballot::new();
    for inner_capture in inner.captures_iter(&capture.as_str()) {
        if !alternatives.contains(&inner_capture["alternative"]) {
            return None;
        }
        ballot.insert(
            inner_capture["alternative"].to_string(),
            inner_capture["lower"].parse::<u64>().unwrap(),
            inner_capture["upper"].parse::<u64>().unwrap(),
        );
    }
    Some(ballot)
}
