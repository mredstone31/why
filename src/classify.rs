pub fn classify_line(line: &str) -> Option<&'static str> {
    // File-related syscalls
    if line.contains("open(")
        || line.contains("openat(")
        || line.contains("read(")
        || line.contains("write(")
        || line.contains("close(")
        || line.contains("stat(")
        || line.contains("lstat(")
        || line.contains("newfstatat(")
        || line.contains("access(")
    {
        return Some("file");
    }

    // Network-related syscalls
    if line.contains("socket(")
        || line.contains("connect(")
        || line.contains("accept(")
        || line.contains("sendto(")
        || line.contains("recvfrom(")
        || line.contains("sendmsg(")
        || line.contains("recvmsg(")
    {
        return Some("network");
    }

    // Process-related syscalls
    if line.contains("execve(")
        || line.contains("clone(")
        || line.contains("fork(")
        || line.contains("vfork(")
        || line.contains("wait4(")
    {
        return Some("process");
    }

    // Waiting / blocking / sleeping
    if line.contains("poll(")
        || line.contains("ppoll(")
        || line.contains("select(")
        || line.contains("pselect6(")
        || line.contains("epoll_wait(")
        || line.contains("nanosleep(")
        || line.contains("clock_nanosleep(")
        || line.contains("futex(")
    {
        return Some("wait");
    }

    None
}

pub fn extract_duration(line: &str) -> Option<f64> {
    let start = line.rfind('<')?;
    let end = line.rfind('>')?;

    if end <= start {
        return None;
    }

    let time_text = &line[start + 1..end];
    time_text.parse::<f64>().ok()
}

pub fn extract_syscall_name(line: &str) -> Option<String> {
    if line.starts_with("+++ ") || line.starts_with("--- ") {
        return None;
    }

    let paren_index = line.find('(')?;
    let before_paren = &line[..paren_index];
    let syscall_name = before_paren.split_whitespace().last()?;

    if syscall_name.is_empty() {
        return None;
    }

    Some(syscall_name.to_string())
}