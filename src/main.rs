use std::collections::HashMap;
use std::env;
use std::thread;

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

fn optional<'a>(args: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
    args.get(key)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
}

fn parse_port(value: &str) -> u16 {
    let port = value
        .parse::<u16>()
        .unwrap_or_else(|_| panic!("invalid port: {value}"));
    if port == 0 {
        panic!("port must be greater than 0: {value}");
    }
    port
}

fn parse_ports(spec: &str) -> Vec<u16> {
    let mut ports = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            panic!("empty port segment in: {spec}");
        }

        if let Some((start, end)) = part.split_once('-') {
            let start = parse_port(start.trim());
            let end = parse_port(end.trim());
            if start > end {
                panic!("invalid port range {part}: start is greater than end");
            }
            for port in start..=end {
                ports.push(port);
            }
            continue;
        }

        ports.push(parse_port(part));
    }

    if ports.is_empty() {
        panic!("empty port spec: {spec}");
    }

    ports
}

fn expand_endpoints(spec: &str) -> Vec<String> {
    let (host, port_spec) = spec
        .rsplit_once(':')
        .unwrap_or_else(|| panic!("endpoint must be in host:port format: {spec}"));

    if host.is_empty() {
        panic!("missing host in endpoint: {spec}");
    }

    parse_ports(port_spec)
        .into_iter()
        .map(|port| format!("{host}:{port}"))
        .collect()
}

fn pair_bind_targets(bind: &str, target: &str) -> Vec<(String, String)> {
    let binds = expand_endpoints(bind);
    let targets = expand_endpoints(target);

    if binds.len() == targets.len() {
        return binds.into_iter().zip(targets).collect();
    }

    if binds.len() == 1 {
        let bind = binds[0].clone();
        return targets.into_iter().map(|target| (bind.clone(), target)).collect();
    }

    if targets.len() == 1 {
        let target = targets[0].clone();
        return binds.into_iter().map(|bind| (bind, target.clone())).collect();
    }

    panic!(
        "bind/target size mismatch: {} binds and {} targets",
        binds.len(),
        targets.len()
    );
}

fn run_parallel(tasks: Vec<Box<dyn FnOnce() + Send>>) {
    let mut handles = Vec::new();
    for task in tasks {
        handles.push(thread::spawn(task));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

fn main() {
    let args = parse_args();
    let proxy_type = required(&args, "type");
    match proxy_type {
        "tcp" => {
            let bind = required(&args, "bind");
            let target = required(&args, "target");
            let pairs = pair_bind_targets(bind, target);
            run_parallel(
                pairs
                    .into_iter()
                    .map(|(bind, target)| Box::new(move || tcp_proxy::run(&bind, &target)) as Box<dyn FnOnce() + Send>)
                    .collect(),
            );
        }
        "udp" => {
            let bind = required(&args, "bind");
            let target = required(&args, "target");
            let pairs = pair_bind_targets(bind, target);
            run_parallel(
                pairs
                    .into_iter()
                    .map(|(bind, target)| Box::new(move || udp_proxy::run(&bind, &target)) as Box<dyn FnOnce() + Send>)
                    .collect(),
            );
        }
        "tcpv2" => {
            let bind = required(&args, "bind");
            let target = required(&args, "target");
            let pairs = pair_bind_targets(bind, target);
            run_parallel(
                pairs
                    .into_iter()
                    .map(|(bind, target)| Box::new(move || tcp_proxy_v2::run(&bind, &target)) as Box<dyn FnOnce() + Send>)
                    .collect(),
            );
        }
        "mc" => {
            let bind = required(&args, "bind");
            let data = optional(&args, "data");
            let binds = expand_endpoints(bind);
            let data = data.map(str::to_string);
            run_parallel(
                binds
                    .into_iter()
                    .map(|bind| {
                        let data = data.clone();
                        Box::new(move || mc_blank::run(&bind, data.as_deref())) as Box<dyn FnOnce() + Send>
                    })
                    .collect(),
            );
        }
        _ => panic!("unknown --type: {proxy_type}"),
    }
}
