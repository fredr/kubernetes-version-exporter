use actix_web::{web, App, HttpResponse, HttpServer};
use actix_web_prom::PrometheusMetricsBuilder;

use kubernetes_version_exporter::ExportMetrics;

fn healthz() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let prometheus = PrometheusMetricsBuilder::new(&ExportMetrics::namespace())
        .registry(prometheus::default_registry().clone())
        .endpoint("/metrics")
        .build()
        .unwrap();

    prometheus
        .registry
        .register(Box::new(ExportMetrics::new()))
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
