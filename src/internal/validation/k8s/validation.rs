use const_format::concatcp;
use lazy_static::lazy_static;
use regex::Regex;

const QNAME_CHAR_FMT: &str = "[A-Za-z0-9]";
const QNAME_EXT_CHAR_FMT: &str = "[-A-Za-z0-9_.]";
const QUALIFIED_NAME_FMT: &str = concatcp!(
    "(",
    QNAME_CHAR_FMT,
    QNAME_EXT_CHAR_FMT,
    "*)?",
    QNAME_CHAR_FMT
);
const QUALIFIED_NAME_ERR_MSG: &str = "must consist of alphanumeric characters, '-', '_' or '.', and must start and end with an alphanumeric character";
const QUALIFIED_NAME_MAX_LENGTH: usize = 63;

const LABEL_VALUE_FMT: &str = concatcp!("(", QUALIFIED_NAME_FMT, ")?");
const LABEL_VALUE_ERR_MSG: &str = "a valid label must be an empty string or consist of alphanumeric characters, '-', '_' or '.', and must start and end with an alphanumeric character";
const LABEL_VALUE_MAX_LENGTH: usize = 63;

const DNS1123_LABEL_FMT: &str = "[a-z0-9]([-a-z0-9]*[a-z0-9])?";
const DNS1123_LABEL_ERR_MSG: &str = "a lowercase RFC 1123 label must consist of lower case alphanumeric characters or '-', and must start and end with an alphanumeric character";
const DNS1123_LABEL_MAX_LENGTH: usize = 63;

const DNS1123_SUBDOMAIN_FMT: &str = concatcp!(DNS1123_LABEL_FMT, "(\\.", DNS1123_LABEL_FMT, ")*");
const DNS1123_SUBDOMAIN_ERR_MSG: &str = "a lowercase RFC 1123 subdomain must consist of lower case alphanumeric characters, '-' or '.', and must start and end with an alphanumeric character";
const DNS1123_SUBDOMAIN_MAX_LENGTH: usize = 253;

const DNS1035_LABEL_FMT: &str = "[a-z]([-a-z0-9]*[a-z0-9])?";
const DNS1035_LABEL_ERR_MSG: &str = "a DNS-1035 label must consist of lower case alphanumeric characters or '-', start with an alphabetic character, and end with an alphanumeric character";
const DNS1035_LABEL_MAX_LENGTH: usize = 63;

const WILDCARD_DNS1123_SUBDOMAIN_FMT: &str = concatcp!("\\*\\.", DNS1123_SUBDOMAIN_FMT);
const WILDCARD_DNS1123_SUBDOMAIN_ERR_MSG: &str = "a wildcard DNS-1123 subdomain must start with '*.', followed by a valid DNS subdomain, which must consist of lower case alphanumeric characters, '-' or '.' and end with an alphanumeric character";

lazy_static! {
    static ref QUALIFIED_NAME_REGEXP: Regex =
        Regex::new(&format!("^{}$", QUALIFIED_NAME_FMT)).unwrap();
    static ref LABEL_VALUE_REGEXP: Regex = Regex::new(&format!("^{}$", LABEL_VALUE_FMT)).unwrap();
    static ref DNS1123_LABEL_REGEXP: Regex =
        Regex::new(&format!("^{}$", DNS1123_LABEL_FMT)).unwrap();
    static ref DNS1123_SUBDOMAIN_REGEXP: Regex =
        Regex::new(&format!("^{}$", DNS1123_SUBDOMAIN_FMT)).unwrap();
    static ref DNS1035_LABEL_REGEXP: Regex =
        Regex::new(&format!("^{}$", DNS1035_LABEL_FMT)).unwrap();
    static ref WILDCARD_DNS1123_SUBDOMAIN_REGEXP: Regex =
        Regex::new(&format!("^{}$", WILDCARD_DNS1123_SUBDOMAIN_FMT)).unwrap();
}

pub fn is_qualified_name(value: &str) -> Vec<String> {
    let mut errs = Vec::new();
    let parts: Vec<&str> = value.split('/').collect();
    let name: &str;

    match parts.len() {
        1 => {
            name = parts[0];
        }
        2 => {
            let prefix = parts[0];
            name = parts[1];
            if prefix.is_empty() {
                errs.push(format!("prefix part {}", empty_error()));
            } else {
                let msgs = is_dns1123_subdomain(prefix);
                if msgs.is_empty() {
                    errs.extend(prefix_each(&msgs, "prefix part "));
                }
            }
        }
        _ => {
            return vec![format!(
			"a qualified name {} with an optional DNS subdomain prefix and '/' (e.g. 'example.com/MyName')",
			regex_error(QUALIFIED_NAME_ERR_MSG, QUALIFIED_NAME_FMT, &["MyName", "my.name", "123-abc"])
		    )];
        }
    }

    if name.is_empty() {
        errs.push(format!("name part {}", empty_error()));
    } else if name.len() > QUALIFIED_NAME_MAX_LENGTH {
        errs.push(format!(
            "name part {}",
            max_len_error(QUALIFIED_NAME_MAX_LENGTH)
        ));
    }

    if !QUALIFIED_NAME_REGEXP.is_match(name) {
        errs.push(format!(
            "name part {}",
            regex_error(
                QUALIFIED_NAME_ERR_MSG,
                QUALIFIED_NAME_FMT,
                &["MyName", "my.name", "123-abc"]
            )
        ));
    }

    errs
}

#[allow(dead_code)]
fn is_valid_label_value(value: &str) -> Vec<String> {
    let mut errs = Vec::new();
    if value.len() > LABEL_VALUE_MAX_LENGTH {
        errs.push(max_len_error(LABEL_VALUE_MAX_LENGTH));
    }
    if !LABEL_VALUE_REGEXP.is_match(value) {
        errs.push(regex_error(
            LABEL_VALUE_ERR_MSG,
            LABEL_VALUE_FMT,
            &["MyValue", "my_value", "12345"],
        ));
    }
    errs
}

#[allow(dead_code)]
fn is_dns1123_label(value: &str) -> Vec<String> {
    let mut errs = Vec::new();
    if value.len() > DNS1123_LABEL_MAX_LENGTH {
        errs.push(max_len_error(DNS1123_LABEL_MAX_LENGTH));
    }
    if !DNS1123_LABEL_REGEXP.is_match(value) {
        errs.push(regex_error(
            DNS1123_LABEL_ERR_MSG,
            DNS1123_LABEL_FMT,
            &["my-name", "123-abc"],
        ));
    }
    errs
}

fn is_dns1123_subdomain(value: &str) -> Vec<String> {
    let mut errs = Vec::new();
    if value.len() > DNS1123_SUBDOMAIN_MAX_LENGTH {
        errs.push(max_len_error(DNS1123_SUBDOMAIN_MAX_LENGTH));
    }
    if !DNS1123_SUBDOMAIN_REGEXP.is_match(value) {
        errs.push(regex_error(
            DNS1123_SUBDOMAIN_ERR_MSG,
            DNS1123_SUBDOMAIN_FMT,
            &["example.com"],
        ));
    }
    errs
}

#[allow(dead_code)]
fn is_dns1035_label(value: &str) -> Vec<String> {
    let mut errs = Vec::new();
    if value.len() > DNS1035_LABEL_MAX_LENGTH {
        errs.push(max_len_error(DNS1035_LABEL_MAX_LENGTH));
    }
    if !DNS1035_LABEL_REGEXP.is_match(value) {
        errs.push(regex_error(
            DNS1035_LABEL_ERR_MSG,
            DNS1035_LABEL_FMT,
            &["my-name", "abc-123"],
        ));
    }
    errs
}

#[allow(dead_code)]
fn is_wildcard_dns1123_subdomain(value: &str) -> Vec<String> {
    let mut errs = Vec::new();
    if value.len() > DNS1123_SUBDOMAIN_MAX_LENGTH {
        errs.push(max_len_error(DNS1123_SUBDOMAIN_MAX_LENGTH));
    }
    if !WILDCARD_DNS1123_SUBDOMAIN_REGEXP.is_match(value) {
        errs.push(regex_error(
            WILDCARD_DNS1123_SUBDOMAIN_ERR_MSG,
            WILDCARD_DNS1123_SUBDOMAIN_FMT,
            &["*.example.com"],
        ));
    }
    errs
}

fn max_len_error(max_length: usize) -> String {
    format!("must be no more than {} characters", max_length)
}

fn regex_error(err_msg: &str, regex_fmt: &str, examples: &[&str]) -> String {
    if examples.is_empty() {
        format!("{} (regex used for validation is '{}')", err_msg, regex_fmt)
    } else {
        let examples_str = examples
            .iter()
            .map(|&e| format!("'{}'", e))
            .collect::<Vec<String>>()
            .join(" or ");
        format!(
            "{} (e.g. {}. regex used for validation is '{}')",
            err_msg, examples_str, regex_fmt
        )
    }
}

fn empty_error() -> &'static str {
    "must be non-empty"
}

fn prefix_each(msgs: &[String], prefix: &str) -> Vec<String> {
    msgs.iter()
        .map(|msg| format!("{}{}", prefix, msg))
        .collect()
}

#[allow(dead_code)]
fn inclusive_range_error(lo: usize, hi: usize) -> String {
    format!("must be between {} and {}, inclusive", lo, hi)
}
