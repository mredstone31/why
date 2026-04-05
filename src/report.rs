use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use colored::Colorize;

use crate::classify::{classify_line, extract_duration, extract_syscall_name};
use crate::insights::{build_insights, diagnosis_for, hint_for};

pub const CATEGORIES: [&str; 4] = ["file", "network", "process", "wait"];

#[derive(Debug)]
pub struct AnalysisReport {
    pub counts: HashMap<&'static str, usize>,
    pub times: HashMap<&'static str, f64>,
    pub total_traced_time: f64,
    pub top_category_by_time: &'static str,
    pub top_syscalls: Vec<(String, usize)>,
}

pub fn analyze_trace(contents: &str) -> AnalysisReport {
    let mut counts: HashMap<&'static str, usize> = HashMap::new();
    let mut times: HashMap<&'static str, f64> = HashMap::new();
    let mut syscall_counts: HashMap<String, usize> = HashMap::new();

    for line in contents.lines() {
        if let Some(category) = classify_line(line) {
            *counts.entry(category).or_insert(0) += 1;

            if let Some(syscall_time) = extract_duration(line) {
                *times.entry(category).or_insert(0.0) += syscall_time;
            }
        }

        if let Some(syscall_name) = extract_syscall_name(line) {
            *syscall_counts.entry(syscall_name).or_insert(0) += 1;
        }
    }

    let total_traced_time: f64 = times.values().sum();

    let mut top_category_by_time = "none";
    let mut top_time = 0.0;

    for (category, time_spent) in &times {
        if *time_spent > top_time {
            top_time = *time_spent;
            top_category_by_time = category;
        }
    }

    let mut top_syscalls: Vec<(String, usize)> = syscall_counts.into_iter().collect();
    top_syscalls.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    top_syscalls.truncate(5);

    AnalysisReport {
        counts,
        times,
        total_traced_time,
        top_category_by_time,
        top_syscalls,
    }
}

pub fn print_report(
    command_text: &str,
    exit_code: i32,
    wall_time: Duration,
    kept_trace_path: Option<&Path>,
    timed_out: bool,
    timeout_secs: u64,
    report: &AnalysisReport,
) {
    println!("{}", "=== why report ===".bright_cyan().bold());
    println!();

    println!("Command: {}", command_text);
    if timed_out {
        println!(
            "Status: {}",
            format!("timed out after {}s", timeout_secs).red().bold()
        );
    } else {
        println!("Exit code: {}", format_exit_code(exit_code));
    }
    println!("Wall time: {:.2?}", wall_time);
    match kept_trace_path {
    Some(path) => println!("Trace file: {}", path.display().to_string().dimmed()),
    None => println!("Trace file: {}", "temporary (auto-deleted)".dimmed()),
}

    println!();
    print_section("Summary");

    for category in CATEGORIES {
        let count = report.counts.get(category).unwrap_or(&0);
        let time_spent = report.times.get(category).unwrap_or(&0.0);

        let percent = if report.total_traced_time > 0.0 {
            (*time_spent / report.total_traced_time) * 100.0
        } else {
            0.0
        };

        let line = format!(
            "{}: {} calls, {:.6}s, {:.1}%",
            capitalize(category),
            count,
            time_spent,
            percent
        );

        if category == report.top_category_by_time {
            println!("{}", highlight_top_category(category, &line));
        } else {
            println!("{}", line);
        }
    }

    println!();
    print_section("Diagnosis");
    println!("{}", diagnosis_for(report.top_category_by_time).green());

    println!();
    print_section("Hint");
    println!("{}", hint_for(report.top_category_by_time).yellow());

    let file_calls = *report.counts.get("file").unwrap_or(&0);
    let network_calls = *report.counts.get("network").unwrap_or(&0);
    let process_calls = *report.counts.get("process").unwrap_or(&0);
    let wait_calls = *report.counts.get("wait").unwrap_or(&0);

    let file_time = *report.times.get("file").unwrap_or(&0.0);
    let network_time = *report.times.get("network").unwrap_or(&0.0);
    let process_time = *report.times.get("process").unwrap_or(&0.0);
    let wait_time = *report.times.get("wait").unwrap_or(&0.0);

    let insights = build_insights(
        file_calls,
        network_calls,
        process_calls,
        wait_calls,
        file_time,
        network_time,
        process_time,
        wait_time,
        report.total_traced_time,
    );

    println!();
    print_section("Insights");
    for insight in insights {
        println!("- {}", insight);
    }

    if !report.top_syscalls.is_empty() {
        println!();
        print_section("Top syscalls");
        for (name, count) in &report.top_syscalls {
            println!("{} {}", name, format!("({})", count).dimmed());
        }
    }
}

fn print_section(title: &str) {
    println!("{}", title.bright_cyan().bold());
    println!("{}", "-".repeat(title.len()).bright_cyan());
}

fn format_exit_code(exit_code: i32) -> colored::ColoredString {
    if exit_code == 0 {
        exit_code.to_string().normal()
    } else {
        exit_code.to_string().red().bold()
    }
}

fn highlight_top_category(category: &str, line: &str) -> colored::ColoredString {
    match category {
        "wait" => line.red().bold(),
        "network" => line.magenta().bold(),
        "file" => line.blue().bold(),
        "process" => line.yellow().bold(),
        _ => line.normal(),
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}