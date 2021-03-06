use kube::Client;
use prometheus::{core::Collector, opts, Gauge, GaugeVec};
use std::{collections::HashMap, thread};

pub struct ExportMetrics {
    build_info: prometheus::Gauge,
    kubernetes_version: prometheus::GaugeVec,
}

impl Default for ExportMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl ExportMetrics {
    pub fn namespace() -> String {
        env!("CARGO_PKG_NAME").replace("-", "_")
    }

    pub fn new() -> Self {
        let build_info = Gauge::with_opts(
            opts!(
                "build_info",
                "Metric with constant value of '1', labeled with build info, such as version"
            )
            .namespace(ExportMetrics::namespace())
            .const_label("version", env!("CARGO_PKG_VERSION")),
        )
        .unwrap();
        build_info.set(1.0);

        let kubernetes_version = GaugeVec::new(
            opts!(
                "kubernetes_version",
                "Metric with constant value of '1', labeled with kubernetes version"
            )
            .namespace(ExportMetrics::namespace()),
            &["version", "major", "minor", "platform", "go_version"],
        )
        .unwrap();

        Self {
            build_info,
            kubernetes_version,
        }
    }
}

impl Collector for ExportMetrics {
    fn desc(&self) -> Vec<&prometheus::core::Desc> {
        [self.build_info.desc(), self.kubernetes_version.desc()].concat()
    }

    fn collect(&self) -> Vec<prometheus::proto::MetricFamily> {
        let info = thread::spawn(|| {
            actix_rt::System::new().block_on(async {
                let client = Client::try_default().await.unwrap();
                client.apiserver_version().await.unwrap()
            })
        })
        .join()
        .unwrap();

        let mut labels: HashMap<&str, &str> = HashMap::new();
        labels.insert("version", &info.git_version);
        labels.insert("major", &info.major);
        labels.insert("minor", &info.minor);
        labels.insert("platform", &info.platform);
        labels.insert("go_version", &info.go_version);

        let kubernetes_version = self.kubernetes_version.with(&labels);
        kubernetes_version.set(1.0);

        [self.build_info.collect(), kubernetes_version.collect()].concat()
    }
}
