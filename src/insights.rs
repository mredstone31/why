pub fn diagnosis_for(category: &str) -> &'static str {
    match category {
        "file" => "This command mostly spent its traced time touching files.",
        "network" => "This command mostly spent its traced time doing network work.",
        "process" => "This command mostly spent its traced time creating or waiting on processes.",
        "wait" => "This command mostly spent its traced time waiting or sleeping.",
        _ => "This command does not have enough classified data yet.",
    }
}

pub fn hint_for(category: &str) -> &'static str {
    match category {
        "file" => "This often means lots of file reads, writes, metadata checks, or directory scanning.",
        "network" => "This often means connecting, sending, receiving, or other socket-heavy work.",
        "process" => "This often means spawning child processes or waiting for them to finish.",
        "wait" => "This often means sleep, blocking I/O, waiting on sockets, or waiting on locks/events.",
        _ => "Try another command with more visible activity.",
    }
}

pub fn build_insights(
    file_calls: usize,
    network_calls: usize,
    process_calls: usize,
    wait_calls: usize,
    file_time: f64,
    network_time: f64,
    process_time: f64,
    wait_time: f64,
    total_traced_time: f64,
) -> Vec<String> {
    let mut insights = Vec::new();

    if total_traced_time <= 0.0 {
        insights.push("Not enough traced time data yet.".to_string());
        return insights;
    }

    let file_percent = (file_time / total_traced_time) * 100.0;
    let network_percent = (network_time / total_traced_time) * 100.0;
    let process_percent = (process_time / total_traced_time) * 100.0;
    let wait_percent = (wait_time / total_traced_time) * 100.0;

    if wait_percent >= 60.0 {
        insights.push("This command spent most of its traced time waiting.".to_string());
    }

    if file_percent >= 40.0 {
        insights.push("This command looked file-heavy.".to_string());
    }

    if network_percent >= 20.0 {
        insights.push("Noticeable network activity was detected.".to_string());
    }

    if process_percent >= 20.0 {
        insights.push("A meaningful chunk of time went into process-related work.".to_string());
    }

    if process_calls >= 5 {
        insights.push("This command spawned or waited on many processes.".to_string());
    }

    if network_calls > 0 && wait_percent >= 40.0 {
        insights.push("A lot of the waiting may be network-related.".to_string());
    }

    if file_calls >= 100 {
        insights.push("This command touched a lot of files or file metadata.".to_string());
    }

    if wait_calls == 1 && wait_percent >= 80.0 {
        insights.push("This looks a lot like sleep or one big blocking wait.".to_string());
    }

    insights
}