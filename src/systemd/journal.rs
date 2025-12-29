// Journalctl log streaming

use crate::error::{Result, SysproxError};
use crate::events::AppEvent;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct LogLine {
    pub timestamp: String,
    pub message: String,
    pub priority: Option<u8>,
    pub raw_line: String,
    pub is_live: bool,
}

#[derive(Debug)]
pub struct JournalReader {
    process: Option<Child>,
    _service_name: String,
}

impl JournalReader {
    /// Start streaming logs for a service
    pub async fn stream_logs(
        service_name: String,
        tx: mpsc::Sender<AppEvent>,
        follow: bool,
        min_priority: Option<u8>,
        since: Option<String>,
        until: Option<String>,
    ) -> Result<Self> {
        let mut args: Vec<String> = vec![
            "-u".to_string(),
            service_name.clone(),
            "--output=short-precise".to_string(),
            "--no-pager".to_string(),
            "-n".to_string(),
            "100".to_string(), // Last 100 lines
        ];

        if follow {
            args.push("-f".to_string()); // Follow mode
        }

        // Add priority filter
        if let Some(priority) = min_priority {
            args.push("-p".to_string());
            args.push(priority.to_string());
        }

        // Add time filters
        if let Some(since_val) = since {
            args.push("--since".to_string());
            args.push(since_val);
        }

        if let Some(until_val) = until {
            args.push("--until".to_string());
            args.push(until_val);
        }

        let mut child = Command::new("journalctl");
        for arg in &args {
            child.arg(arg);
        }
        let mut child = child
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SysproxError::Journal(format!("Failed to spawn journalctl: {}", e)))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| SysproxError::Journal("Failed to capture stdout".to_string()))?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| SysproxError::Journal("Failed to capture stderr".to_string()))?;

        // Spawn task to read stdout lines
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut lines_seen = 0;

            while let Ok(Some(line)) = lines.next_line().await {
                lines_seen += 1;
                
                // Parse the log line
                let log_line = parse_log_line(&line, follow && lines_seen > 100);
                
                if tx_clone.send(AppEvent::LogLineParsed(log_line)).await.is_err() {
                    break;
                }
            }
        });

        // Spawn task to read stderr (for error messages)
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                // Send stderr as error log lines (prefixed)
                let error_line = format!("[ERROR] {}", line);
                if tx.send(AppEvent::LogLine(error_line)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Self {
            process: Some(child),
            _service_name: service_name,
        })
    }

    /// Stop streaming logs
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut process) = self.process.take() {
            process
                .kill()
                .await
                .map_err(|e| SysproxError::Journal(format!("Failed to kill process: {}", e)))?;
        }
        Ok(())
    }
}

impl Drop for JournalReader {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            // Best effort cleanup
            let _ = process.start_kill();
        }
    }
}

fn parse_log_line(line: &str, is_live: bool) -> LogLine {
    // short-precise format: "timestamp hostname process[pid]: message"
    // Example: "Dec 25 10:30:15.123456 server nginx[1234]: Server started"
    
    let raw_line = line.to_string();
    
    // Try to extract timestamp (first 3 words + microseconds)
    let parts: Vec<&str> = line.splitn(5, ' ').collect();
    let (timestamp, message) = if parts.len() >= 4 {
        // Reconstruct timestamp from first 4 parts
        let timestamp = format!("{} {} {} {}", parts[0], parts[1], parts[2], parts[3]);
        let message = parts.get(4).unwrap_or(&"").to_string();
        (timestamp, message)
    } else {
        (line.to_string(), String::new())
    };
    
    // Try to extract priority from the message (systemd sometimes includes <priority>)
    let priority = if message.starts_with('<') && message.contains('>') {
        message[1..message.find('>').unwrap_or(1)]
            .parse::<u8>()
            .ok()
    } else {
        None
    };
    
    LogLine {
        timestamp,
        message,
        priority,
        raw_line,
        is_live,
    }
}
