pub mod config;
pub mod core;
pub mod sphinx;
pub mod storage;

pub mod client;
pub mod mixnode;
pub mod provider;

pub mod messages;

pub mod broker;

use prometheus::{register_counter, register_gauge, register_histogram, Counter, Gauge, Histogram};
use log;

lazy_static::lazy_static! {
    pub static ref END_TO_END_LATENCY: Histogram = match register_histogram!(
        "loopix_end_to_end_latency_seconds",
        "End-to-end latency web proxy request.",
        vec![1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
    ) {
        Ok(histogram) => {
            log::info!("END_TO_END_LATENCY histogram registered successfully.");
            histogram
        },
        Err(e) => {
            log::error!("Failed to register END_TO_END_LATENCY histogram: {:?}", e);
            Histogram::with_opts(prometheus::HistogramOpts::new(
                "loopix_end_to_end_latency_seconds",
                "End-to-end latency web proxy request."
            )).unwrap()
        }
    };

    pub static ref MIXNODE_DELAY: Histogram = match register_histogram!(
        "loopix_mixnode_delay_milliseconds",
        "Delay introduced by a mixnode.",
        vec![0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.25, 2.5, 2.75, 3.0, 3.25, 3.5, 3.75, 4.0, 6.0, 10.0, 50.0, 200.0]
        // vec![0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 6.0, 10.0, 50.0, 200.0]
    ) {
        Ok(histogram) => {
            log::info!("MIXNODE_DELAY histogram registered successfully.");
            histogram
        },
        Err(e) => {
            log::error!("Failed to register MIXNODE_DELAY histogram: {:?}", e);
            Histogram::with_opts(prometheus::HistogramOpts::new(
                "loopix_mixnode_delay_milliseconds",
                "Delay introduced by a mixnode."
            )).unwrap()
        }
    };

    pub static ref ENCRYPTION_LATENCY: Histogram = match register_histogram!(
        "loopix_encryption_latency_milliseconds",
        "Time taken for encryption.", // TODO maybe let's take all encryption latencies
        vec![0.1, 0.25, 0.5, 1.0, 1.25, 1.5, 1.75, 2.0, 2.25, 2.5, 2.75, 3.0, 3.25, 3.5, 3.75, 4.0, 10.0, 50.0, 200.0] 
    ) {
        Ok(histogram) => {
            log::info!("ENCRYPTION_LATENCY histogram registered successfully.");
            histogram
        },
        Err(e) => {
            log::error!("Failed to register ENCRYPTION_LATENCY histogram: {:?}", e);
            Histogram::with_opts(prometheus::HistogramOpts::new(
                "loopix_encryption_latency_milliseconds",
                "Time taken for encryption."
            )).unwrap()
        }
    };

    pub static ref DECRYPTION_LATENCY: Histogram = match register_histogram!(
        "loopix_decryption_latency_milliseconds",
        "Time taken for decryption.",
        vec![0.0001, 0.001, 0.0025, 0.005, 0.01, 0.5] // TODO maybe it makes more sense to do processing latency?
    ) {
        Ok(histogram) => {
            log::info!("DECRYPTION_LATENCY histogram registered successfully.");
            histogram
        },
        Err(e) => {
            log::error!("Failed to register DECRYPTION_LATENCY histogram: {:?}", e);
            Histogram::with_opts(prometheus::HistogramOpts::new(
                "loopix_decryption_latency_milliseconds",
                "Time taken for decryption."
            )).unwrap()
        }
    };

    pub static ref BANDWIDTH: Counter = match register_counter!(
        "loopix_bandwidth_bytes",
        "Bandwidth usage in bytes"
    ) {
        Ok(counter) => {
            log::info!("BANDWIDTH counter registered successfully.");
            counter
        },
        Err(e) => {
            log::error!("Failed to register BANDWIDTH counter: {:?}", e);
            Counter::new("loopix_bandwidth_bytes", "Bandwidth usage in bytes").unwrap()
        }
    };

    pub static ref NUMBER_OF_PROXY_REQUESTS: Counter = match register_counter!(
        "loopix_number_of_proxy_requests",
        "Number of proxy requests"
    ) {
        Ok(counter) => {
            log::info!("NUMBER_OF_PROXY_REQUESTS counter registered successfully.");
            counter
        },
        Err(e) => {
            log::error!("Failed to register NUMBER_OF_PROXY_REQUESTS counter: {:?}", e);
            Counter::new("loopix_number_of_proxy_requests", "Number of proxy requests").unwrap()
        }
    };

    pub static ref CLIENT_DELAY: Histogram = match register_histogram!(
        "loopix_client_delay_milliseconds",
        "Delay introduced by teh client queue.",
        vec![1.0, 3.0, 5.0, 10.0, 20.0, 50.0, 200.0, 500.0, 1000.0, 5000.0, 10000.0, 20000.0, 50000.0, 100000.0]
    ) {
        Ok(histogram) => {
            log::info!("CLIENT_DELAY histogram registered successfully.");
            histogram
        },
        Err(e) => {
            log::error!("Failed to register CLIENT_DELAY histogram: {:?}", e);
            Histogram::with_opts(prometheus::HistogramOpts::new(
                "loopix_client_delay_milliseconds",
                "Delay introduced by a client."
            )).unwrap()
        }
    };

    pub static ref PROVIDER_DELAY: Histogram = match register_histogram!(
        "loopix_provider_delay_milliseconds",
        "Delay introduced by a provider.",
        vec![1.0, 3.0, 5.0, 10.0, 20.0, 50.0, 200.0, 500.0, 1000.0, 5000.0, 10000.0, 20000.0, 50000.0, 100000.0]
    ) {
        Ok(histogram) => {
            log::info!("PROVIDER_DELAY histogram registered successfully.");
            histogram
        },
        Err(e) => {
            log::error!("Failed to register PROVIDER_DELAY histogram: {:?}", e);
            Histogram::with_opts(prometheus::HistogramOpts::new(
                "loopix_provider_delay_milliseconds",
            "Delay introduced by a provider."
            )).unwrap()
        }
    };

    pub static ref LOOPIX_START_TIME: Gauge = match register_gauge!(
        "loopix_start_time_seconds",
        "Start time of the loopix service."
    ) {
        Ok(gauge) => {
            log::info!("LOOPIX_START_TIME gauge registered successfully.");
            gauge
        },
        Err(e) => {
            log::error!("Failed to register LOOPIX_START_TIME gauge: {:?}", e);
            Gauge::new("loopix_start_time_seconds", "Start time of the loopix service.").unwrap()
        }
    };
}

#[cfg(feature = "testing")]
pub mod testing;
