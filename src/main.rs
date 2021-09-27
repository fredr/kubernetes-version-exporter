use std::{collections::HashMap, thread};

use actix_web::{web, App, HttpResponse, HttpServer};
use actix_web_prom::PrometheusMetricsBuilder;

use prometheus::{core::Collector, opts, Gauge, GaugeVec};

use kube::Client;

fn healthz() -> HttpResponse {
    HttpResponse::Ok().finish()
}

// TODO(fredr): Move all the Metrics implementations to a lib
struct Metrics {
    build_info: prometheus::Gauge,
    kubernetes_version: prometheus::GaugeVec,
}

impl Metrics {
    fn namespace() -> String {
        env!("CARGO_PKG_NAME").replace("-", "_")
    }

    fn new() -> Self {
        let build_info = Gauge::with_opts(
            opts!(
                "build_info",
                "Metric with constant value of '1', labeled with build info, such as version"
            )
            .namespace(Metrics::namespace())
            .const_label("version", env!("CARGO_PKG_VERSION")),
        )
        .unwrap();
        build_info.set(1.0);

        let kubernetes_version = GaugeVec::new(
            opts!(
                "kubernetes_version",
                "Metric with constant value of '1', labeled with kubernetes version"
            )
            .namespace(Metrics::namespace()),
            &["version", "major", "minor", "platform", "go_version"],
        )
        .unwrap();

        Self {
            build_info,
            kubernetes_version,
        }
    }
}

impl Collector for Metrics {
    fn desc(&self) -> Vec<&prometheus::core::Desc> {
        [self.build_info.desc(), self.kubernetes_version.desc()].concat()
    }

    fn collect(&self) -> Vec<prometheus::proto::MetricFamily> {
        let info = thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let prometheus = PrometheusMetricsBuilder::new(&Metrics::namespace())
        .registry(prometheus::default_registry().clone())
        .endpoint("/metrics")
        .build()
        .unwrap();

    prometheus
        .registry
        .register(Box::new(Metrics::new()))
        .unwrap();

    HttpServer::new(move || {
        App::new()
            .wrap(prometheus.clone())
            .service(web::resource("/healthz").to(healthz))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await?;

    Ok(())
}
