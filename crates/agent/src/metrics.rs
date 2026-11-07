use std::fs;
use std::thread;
use std::time::Duration;

use shared::util;

use shared::models::{
    CpuUsage, 
    LoadAverage, 
    MemoryUsage, 
    NodeMetrics,
};

use crate::errors::{AgentError, Result};

pub fn get_node_metrics() -> Result<NodeMetrics> {
    let metrics = NodeMetrics {
        cpu_usage: get_cpu_usage_percentage()?,
        memory_usage: get_memory_summary()?,
        load_average: get_load_average()?,
        uptime: get_uptime()?,
    };

    Ok(metrics)
}

fn get_raw_cpu_info() -> Result<String> {
    Ok(fs::read_to_string("/proc/stat")?)
}

fn get_raw_load_average() -> Result<String> {
    Ok(fs::read_to_string("/proc/loadavg")?)
}

// busy = user + nice + system + irq + softirq + steal
// idle = idle + iowait
// total = busy + idle
fn get_cpu_usage() -> Result<CpuUsage> {
    let raw_cpu = get_raw_cpu_info()?;
    parse_cpu_usage(&raw_cpu)
}

fn parse_cpu_usage(raw_cpu: &str) -> Result<CpuUsage> {
    let cpu_summary = raw_cpu
        .lines()
        .next()
        .ok_or(AgentError::MissingCpuField)?;

    let cpu_summary_vec: Vec<&str> = cpu_summary.split_whitespace().collect();

    let idle: f64 = cpu_summary_vec
        .get(4)
        .ok_or(AgentError::MissingCpuField)?
        .parse()?;
    let iowait: f64 = cpu_summary_vec
        .get(5)
        .ok_or(AgentError::MissingCpuField)?
        .parse()?;

    let user: f64 = cpu_summary_vec
        .get(1)
        .ok_or(AgentError::MissingCpuField)?
        .parse()?;
    let nice: f64 = cpu_summary_vec
        .get(2)
        .ok_or(AgentError::MissingCpuField)?
        .parse()?;
    let system: f64 = cpu_summary_vec
        .get(3)
        .ok_or(AgentError::MissingCpuField)?
        .parse()?;
    let irq: f64 = cpu_summary_vec
        .get(6)
        .ok_or(AgentError::MissingCpuField)?
        .parse()?;
    let softirq: f64 = cpu_summary_vec
        .get(7)
        .ok_or(AgentError::MissingCpuField)?
        .parse()?;
    let steal: f64 = cpu_summary_vec
        .get(8)
        .ok_or(AgentError::MissingCpuField)?
        .parse()?;
    let _guest: f64 = cpu_summary_vec
        .get(9)
        .ok_or(AgentError::MissingCpuField)?
        .parse()?;
    let _guest_nice: f64 = cpu_summary_vec
        .get(10)
        .ok_or(AgentError::MissingCpuField)?
        .parse()?;

    // not including guest + guest_nice because according to docs, it's included
    // in user and nice already (so essentially redundant)
    let busy = user + nice + system + irq + softirq + steal;
    let idle = idle + iowait;

    let total = busy + idle;

    Ok(
        CpuUsage { busy, total }
    )   
}

// usage = busy_delta / total_delta × 100
pub fn get_cpu_usage_percentage() -> Result<f64> {
    let t1 = get_cpu_usage()?;
    thread::sleep(Duration::from_secs(1));
    let t2 = get_cpu_usage()?;

    Ok(
        (t2.busy - t1.busy) / (t2.total - t1.total) * 100.0
    )
}

fn get_raw_memory() -> Result<String> {
    Ok(fs::read_to_string("/proc/meminfo")?)
}

// mem_total -> Total physical RAM available to machine
// mem_free -> Completely unused memory (usually small to prevent waste)
// mem_available -> How much memory 'could' machine give to applications
//
// used = mem_total - mem_available
pub fn get_memory_summary() -> Result<MemoryUsage> {
    let raw_memory = get_raw_memory()?;
    let raw_memory_vec = util::convert_to_vec(raw_memory, '\n');

    let mem_total = raw_memory_vec
        .get(0)
        .ok_or(AgentError::MissingMemoryField)?;
    let mem_free = raw_memory_vec
        .get(1)
        .ok_or(AgentError::MissingMemoryField)?;
    let mem_available = raw_memory_vec
        .get(2)
        .ok_or(AgentError::MissingMemoryField)?;

    // regex worthy?
    let mem_total_vec = util::convert_to_vec(mem_total.clone().replace("  ", ""), ' ');
    let _mem_free_vec = util::convert_to_vec(mem_free.clone().replace("  ", ""), ' ');
    let mem_available_vec = util::convert_to_vec(mem_available.clone().replace("  ", ""), ' ');

    let mem_total: f64 = mem_total_vec
        .get(1)
        .ok_or(AgentError::MissingMemoryField)?
        .parse()?;
    let mem_available: f64 = mem_available_vec
        .get(1)
        .ok_or(AgentError::MissingMemoryField)?
        .parse()?;

    let mem_used = mem_total - mem_available;

    Ok(
        MemoryUsage {
            used_kb: mem_used,
            total_kb: mem_total,
        }
    )
}

fn get_raw_uptime() -> Result<String> {
    Ok(fs::read_to_string("/proc/uptime")?)
}

pub fn get_uptime() -> Result<f64> {
    let raw_uptime = get_raw_uptime()?;
    let uptime_vec = util::convert_to_vec(raw_uptime, ' ');
    Ok(
        uptime_vec
        .get(0)
        .ok_or(AgentError::MissingUptimeField)?
        .parse()?
    )
}

pub fn get_load_average() -> Result<LoadAverage> {
    let raw_load_avg = get_raw_load_average()?;
    let load_avg_vec = util::convert_to_vec(raw_load_avg, ' ');

    let min_1: f64 = load_avg_vec
        .get(0)
        .ok_or(AgentError::MissingLoadAverageField)?
        .parse()?;
    let min_5: f64 = load_avg_vec
        .get(1)
        .ok_or(AgentError::MissingLoadAverageField)?
        .parse()?;
    let min_15: f64 = load_avg_vec
        .get(2)
        .ok_or(AgentError::MissingLoadAverageField)?
        .parse()?;

    Ok(
        LoadAverage {
            min_1,
            min_5,
            min_15,
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cpu_usage_success() {
        let raw_cpu = "cpu 100 20 30 400 50 10 5 2 3 4 1\n";

        let result = parse_cpu_usage(raw_cpu).unwrap();

        assert_eq!(result.busy, 167.0);
        assert_eq!(result.total, 617.0);
    }

    #[test]
    fn parse_cpu_missing_fields() {
        let raw_cpu = "cpu 100 20 30 400 50";

        let result = parse_cpu_usage(raw_cpu);

        assert!(matches!(result, Err(AgentError::MissingCpuField)));
    }
}
