/*
   Copyright 2021 JFrog Ltd

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

/// Peer Quality Metrics
use sysinfo::{NetworkExt, ProcessExt, System, SystemExt};

// peer metric constants
const CPU_STRESS_WEIGHT: f64 = 2_f64;
const NETWORK_STRESS_WEIGHT: f64 = 0.001_f64;
const DISK_STRESS_WEIGHT: f64 = 0.001_f64;

/// Get the local stress metric to advertise to peers
pub fn get_quality_metric() -> f64 {
    let mut qm = get_cpu_stress() * CPU_STRESS_WEIGHT;
    qm += get_network_stress() * NETWORK_STRESS_WEIGHT;
    qm + get_disk_stress() * DISK_STRESS_WEIGHT
}

// This function gets the current CPU load on the system.
fn get_cpu_stress() -> f64 {
    let sys = System::new_all();
    let load_avg = sys.load_average();
    load_avg.one //using the average over the last 1 minute
}

// This function gets the current network load on the system
fn get_network_stress() -> f64 {
    let mut sys = System::new_all();
    sys.refresh_networks_list();
    let networks = sys.networks();

    let mut packets_in = 0;
    let mut packets_out = 0;
    for (_interface_name, network) in networks {
        packets_in += network.received();
        packets_out += network.transmitted();
    }
    (packets_in as f64) + (packets_out as f64)
    //TODO: add network card capabilities to the metric. cards with > network capacity should get a lower stress number.
}

fn get_disk_stress() -> f64 {
    let sys = System::new_all();
    // Sum up the disk usage measured as total read and writes per process:
    let mut total_usage = 0_u64;
    for process in sys.processes().values() {
        let usage = process.disk_usage();
        total_usage = total_usage + usage.total_written_bytes + usage.total_read_bytes;
    }
    total_usage as f64
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;

    #[test]
    fn cpu_load_test() {
        get_cpu_stress();
    }

    #[test]
    fn network_load_test() {
        get_network_stress();
    }

    #[test]
    fn disk_load_test() {
        get_disk_stress();
    }

    #[test]
    fn quality_metric_test() {
        get_quality_metric();
    }
}
