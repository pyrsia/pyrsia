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
mod tests {
    use super::*;

    const CPU_THREADS: usize = 200;
    const NETWORK_THREADS: usize = 10;

    #[test]
    fn cpu_load_test() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::thread;
        use std::time::Duration;

        let loading = Arc::new(AtomicBool::new(true));

        //first measure of CPU for benchmark
        let qm = get_cpu_stress() * CPU_STRESS_WEIGHT;
        assert_ne!(0_f64, qm); //zero should never be returned here

        //set CPU on fire to measure stress
        let mut threads = vec![];
        for _i in 0..CPU_THREADS {
            threads.push(thread::spawn({
                let mut cpu_fire = 0;
                let loading_test = loading.clone();
                move || {
                    while loading_test.load(Ordering::Relaxed) {
                        cpu_fire = cpu_fire + 1;
                    }
                }
            }));
        }

        thread::sleep(Duration::from_millis(200)); //let cpu spin up

        //second measure of CPU
        let qm2 = get_cpu_stress() * CPU_STRESS_WEIGHT;
        assert!(qm2 >= qm);
        loading.store(false, Ordering::Relaxed); //kill threads

        //wait for threads
        for thread in threads {
            thread.join().unwrap();
        }
        //we could add another measure of CPU did no think it was that important
    }

    #[test]
    fn network_load_test() {
        use std::net::UdpSocket;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::thread;

        let loading = Arc::new(AtomicBool::new(true));

        //fist measure of network for benchmark
        let qm = get_network_stress() * NETWORK_STRESS_WEIGHT;

        //shotgun the network with packets
        let mut threads = vec![];
        for i in 0..NETWORK_THREADS {
            threads.push(thread::spawn({
                let address: String = format_args!("127.0.0.1:3425{i}").to_string();
                let socket = UdpSocket::bind(address).expect("couldn't bind to address");
                let loading_test = loading.clone();
                move || {
                    while loading_test.load(Ordering::Relaxed) {
                        socket
                            .send_to(&[0; 10], "127.0.0.1:4242")
                            .expect("couldn't send data");
                    }
                }
            }));
        }

        let qm2 = get_network_stress() * NETWORK_STRESS_WEIGHT;
        assert!(qm2 > qm);
        loading.store(false, Ordering::Relaxed); //kill threads

        //wait for threads
        for thread in threads {
            thread.join().unwrap();
        }
        //we could add another measure of network did no think it was that important
    }

    #[test]
    fn disk_load_test() {
        use std::fs;
        use std::fs::OpenOptions;
        use std::io::Write;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::thread;
        use std::time::Duration;

        let loading = Arc::new(AtomicBool::new(true));
        let test_file = "pyrsia_test.txt";

        // fist measure of network for benchmark
        let qm = get_disk_stress() * DISK_STRESS_WEIGHT;

        // write some data
        let write_thread = thread::spawn({
            let file_data = "Some test data for the file!\n";
            let except_str = format!("Unable to open file {}", test_file);
            let mut f = OpenOptions::new()
                .append(true)
                .create(true)
                .open(test_file)
                .expect(&except_str);
            let loading_test = loading.clone();
            move || {
                while loading_test.load(Ordering::Relaxed) {
                    f.write_all(file_data.as_bytes())
                        .expect("Unable to write data");
                }
                drop(f);
            }
        });

        thread::sleep(Duration::from_millis(400)); //let writes happen

        // second measure of network
        let qm2 = get_disk_stress() * DISK_STRESS_WEIGHT;
        loading.store(false, Ordering::Relaxed); //kill thread
        write_thread.join().unwrap();
        fs::remove_file(test_file).unwrap();
        assert!(qm2 > qm);

        //we could add another measure of disks did no think it was that important
    }

    #[test]
    fn quality_metric_test() {
        let quality_metric = get_quality_metric();
        assert!(quality_metric != 0_f64);
    }
}
