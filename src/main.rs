use std::collections::HashMap;
use std::env;

mod mc_blank;
mod tcp_proxy;
mod tcp_proxy_v2;
mod udp_proxy;

fn parse_args() -> HashMap<String, String> {
    let mut args_map = HashMap::new();
    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        if let Some(rest) = arg.strip_prefix("--") {
            if let Some((key, value)) = rest.split_once('=') {
                args_map.insert(key.to_string(), value.to_string());
                i += 1;
                continue;
            }

            if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                args_map.insert(rest.to_string(), args[i + 1].clone());
                i += 2;
                continue;
            }

            args_map.insert(rest.to_string(), String::new());
            i += 1;
            continue;
        }

        i += 1;
    }

    args_map
}

fn required<'a>(args: &'a HashMap<String, String>, key: &str) -> &'a str {
    args.get(key)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| panic!("missing --{key}"))
}

fn main() {
    let args = parse_args();
    let proxy_type = required(&args, "type");

    match proxy_type {
        "tcp" => {
            let bind = required(&args, "bind");
            let target = required(&args, "target");
            tcp_proxy::run(bind, target);
        }
        "udp" => {
            let bind = required(&args, "bind");
            let target = required(&args, "target");
            udp_proxy::run(bind, target);
        }
        "tcpv2" => {
            let bind = required(&args, "bind");
            let target = required(&args, "target");
            tcp_proxy_v2::run(bind, target);
        }
        "mc" => {
            let bind = required(&args, "bind");
            let data = args.get("data").cloned().filter(|value| !value.is_empty());
            mc_blank::run(bind, data);
        }
        _ => panic!("unknown --type: {proxy_type}"),
    }
}
