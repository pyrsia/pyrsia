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
use std::thread;
use std::time;
use sysinfo::{NetworkExt, ProcessExt, System, SystemExt};

// peer metric constants
const CPU_STRESS_WEIGHT: f64 = 2_f64;
const NETWORK_STRESS_WEIGHT: f64 = 0.001_f64;
const DISK_STRESS_WEIGHT: f64 = 0.001_f64;

#[derive(Default)]
pub struct PeerMetrics {
    system: System,
}

impl PeerMetrics {
    pub fn new() -> Self {
        let mut peer_metrics = Self {
            system: System::new_all(),
        };
        peer_metrics.initialize();
        peer_metrics
    }

    fn initialize(&mut self) {
        self.system.refresh_all();
        thread::sleep(time::Duration::from_millis(500));
        self.system.refresh_all();
    }

    /// Get the local stress metric to advertise to peers
    pub fn get_quality_metric(&mut self) -> f64 {
        let mut qm = get_cpu_stress(&mut self.system) * CPU_STRESS_WEIGHT;
        qm += get_network_stress(&mut self.system) * NETWORK_STRESS_WEIGHT;
        qm + get_disk_stress(&mut self.system) * DISK_STRESS_WEIGHT
    }
}

// This function gets the current CPU load on the system.
fn get_cpu_stress(system: &mut System) -> f64 {
    system.refresh_all();

    let load_avg = system.load_average();
    load_avg.one //using the average over the last 1 minute
}

// This function gets the current network load on the system
fn get_network_stress(system: &mut System) -> f64 {
    system.refresh_all();

    let networks = system.networks();

    let mut packets_in = 0;
    let mut packets_out = 0;
    for (_interface_name, network) in networks {
        packets_in += network.received();
        packets_out += network.transmitted();
    }
    (packets_in as f64) + (packets_out as f64)
    //TODO: add network card capabilities to the metric. cards with > network capacity should get a lower stress number.
}

fn get_disk_stress(system: &mut System) -> f64 {
    system.refresh_all();

    // Sum up the disk usage measured as total read and writes per process:
    let mut total_usage = 0_u64;
    for process in system.processes().values() {
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
        let mut peer_metrics = PeerMetrics::new();

        get_cpu_stress(&mut peer_metrics.system);
    }

    #[test]
    fn network_load_test() {
        let mut peer_metrics = PeerMetrics::new();

        get_network_stress(&mut peer_metrics.system);
    }

    #[test]
    fn disk_load_test() {
        let mut peer_metrics = PeerMetrics::new();

        get_disk_stress(&mut peer_metrics.system);
    }

    #[test]
    fn quality_metric_test() {
        let mut peer_metrics = PeerMetrics::new();

        peer_metrics.get_quality_metric();
    }
}
