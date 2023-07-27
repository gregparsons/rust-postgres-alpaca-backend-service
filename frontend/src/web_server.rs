//! web_server.rs

use std::fs;
use std::io::{BufReader};
use std::str::FromStr;
use crate::configuration::get_yaml_configuration;
use actix_session::storage::CookieSessionStore;
use actix_session::SessionMiddleware;
use actix_web::{web, App, HttpServer};
use handlebars::Handlebars;
use sqlx::PgPool;
use common_lib::common_structs::ConfigLocation;
use common_lib::db::DbMsg;
use crate::account::get_account;
use crate::activities::{get_activities, get_activity_for_symbol};
use crate::dashboard::{get_dashboard, get_dashboard_with_symbol};
use crate::edit_settings::{get_settings, get_settings_button};
use crate::login::{get_login, get_logout, post_login};
use crate::order::get_order;
use crate::positions::get_positions;
use crate::profit::{get_profit, get_profit_summary};
use crate::symbols::{get_symbols, post_symbols};
use crate::utils::*;

// this corresponds to the Dockerfile "COPY static /app/frontend/static"
// static STATIC_FILE_DIR:&'static str = "./frontend/static/templates";



pub struct WebServer {}
impl WebServer {
    pub async fn run(tx_db: crossbeam_channel::Sender<DbMsg>) {

        let settings = get_yaml_configuration().expect("no configuration.yaml");
        let address = format!("{}:{}", settings.database.host, settings.database.port);
        tracing::debug!("[run] address from config: {}", &address);
        let web_port = settings.application_port;
        tracing::info!("[run] web server starting on port: {}", &web_port);

        let _ = WebServer::web_server(web_port, tx_db).await;
    }

    fn get_secret_key() -> actix_web::cookie::Key {
        actix_web::cookie::Key::generate()
    }

    async fn web_server(web_port: u16, tx_db: crossbeam_channel::Sender<DbMsg>) -> std::io::Result<()> {
        tracing::info!("starting HTTP server at http://localhost:8080");

        let configuration = get_yaml_configuration().expect("[web_server] no configuration.yaml?");
        let pool = PgPool::connect(&configuration.database.connection_string()).await.expect("[frontend][web server] failed to connect to postgres");


        // handlebars
        // refs:
        // https://github.com/actix/examples/blob/master/templating/handlebars/src/main.rs
        // https://github.com/sunng87/handlebars-rust/tree/master/examples
        let mut handlebars = Handlebars::new();
        let config_location:ConfigLocation = ConfigLocation::from_str(&std::env::var("CONFIG_LOCATION").unwrap_or_else(|_| "not_docker".to_owned())).expect("CONFIG_LOCATION");

        // or CARGO_PKG_NAME
        let package_name = env!("CARGO_MANIFEST_DIR");
        let handlebar_static_path = match config_location {
            ConfigLocation::Docker => "./static/templates".to_string(),
            ConfigLocation::NotDocker => {
                // frontend/static/templates
                // /Users/xyz/.../trade/frontend/static/templates
                format!("{}/static/templates", &package_name)
            }
        };
        tracing::debug!("[web_server] registering handlebars static files to: {}",&handlebar_static_path);
        handlebars.register_templates_directory(".html", handlebar_static_path).unwrap();
        let handlebars_ref = web::Data::new(handlebars);

        // srv is server controller type, `dev::Server`
        let secret_key = WebServer::get_secret_key();

        // state
        let db_pool_data = web::Data::new(pool);
        let tx_db_data = web::Data::new(tx_db.clone());

        // TLS
        // https://github.com/rustls/rustls/blob/main/examples/src/bin/tlsserver-mio.rs
        // let certs = load_certs("/Users/gp/trade/frontend/certificate/cert.pem");
        // let privkey = load_private_key("/Users/gp/trade/frontend/certificate/key.pem");
        let (certs, privkey) = match config_location{
            ConfigLocation::Docker => {
                let certs = load_certs("./static/certificate/cert.pem");
                let privkey = load_private_key("./static/certificate/key.pem");
                // let ocsp = load_ocsp(&args.flag_ocsp);
                (certs, privkey)
            },
            ConfigLocation::NotDocker => {
                let certs = load_certs("frontend/static/certificate/cert.pem");
                let privkey = load_private_key("frontend/static/certificate/key.pem");
                // let ocsp = load_ocsp(&args.flag_ocsp);
                (certs, privkey)
            }
        };

        let config = rustls::ServerConfig::builder()
            .with_cipher_suites(&rustls::ALL_CIPHER_SUITES.to_vec())
            .with_safe_default_kx_groups()
            .with_protocol_versions(&rustls::ALL_VERSIONS.to_vec())
            .expect("inconsistent cipher-suites/versions specified")
            // .with_client_cert_verifier(NoClientAuth::)
            // .with_single_cert_with_ocsp(certs, privkey, ocsp)
            .with_no_client_auth()
            .with_single_cert(certs, privkey)
            .expect("bad certificates/private key");


        let server = HttpServer::new(move || {
            App::new()
                // https://actix.rs/docs/middleware
                // setting secure = false for local testing; otherwise TLS required
                .wrap(SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone()).cookie_secure(false).build())
                .app_data(db_pool_data.clone())
                .app_data(tx_db_data.clone())
                .app_data(handlebars_ref.clone())
                .route("/", web::get().to(get_home))
                .route("/login", web::get().to(get_login))
                .route("/login", web::post().to(post_login))
                // disable signup for now
                // .route("/signup", web::get().to(get_signup))
                // .route("/signup", web::post().to(post_signup))
                .route("/ping", web::get().to(get_ping))
                // .route("/avg", web::get().to(get_avg))
                // .route("/chart", web::get().to(get_chart))
                .route("/profit", web::get().to(get_profit))
                .route("/profit_summary", web::get().to(get_profit_summary))
                .route("/account", web::get().to(get_account))
                .route("/logout", web::get().to(get_logout))
                .route("/symbols", web::get().to(get_symbols))
                .route("/symbols", web::post().to(post_symbols))
                .route("/activity", web::get().to(get_activities))
                .route("/activity/{symbol}", web::get().to(get_activity_for_symbol))
                .route("/settings", web::get().to(get_settings))
                .route("/positions", web::get().to(get_positions))
                .route(
                    "/settings/button/{name}",
                    web::get().to(get_settings_button),
                )
                .route("/dashboard", web::get().to(get_dashboard))
                .route(
                    "/dashboard/{symbol}",
                    web::get().to(get_dashboard_with_symbol),
                )
                .route("/order", web::get().to(get_order))
            // .route("/order/{symbol}", web::get().to(get_orders))
        })
        .bind_rustls(("0.0.0.0", web_port), config)?
        .workers(2)
        .run();

        server.await
    }
}


fn load_private_key(filename: &str) -> rustls::PrivateKey {
    let keyfile = fs::File::open(filename).expect("cannot open private key file");
    let mut reader = BufReader::new(keyfile);



    loop {
        match rustls_pemfile::read_one(&mut reader).expect("cannot parse private key .pem file") {
            Some(rustls_pemfile::Item::RSAKey(key)) => return rustls::PrivateKey(key),
            Some(rustls_pemfile::Item::PKCS8Key(key)) => return rustls::PrivateKey(key),
            Some(rustls_pemfile::Item::ECKey(key)) => return rustls::PrivateKey(key),
            None => break,
            _ => {}
        }
    }

    panic!(
        "no keys found in {:?} (encrypted keys not supported)",
        filename
    );
}


fn load_certs(filename: &str) -> Vec<rustls::Certificate> {
    let certfile = fs::File::open(filename).expect("cannot open certificate file");
    let mut reader = BufReader::new(certfile);
    rustls_pemfile::certs(&mut reader)
        .unwrap()
        .iter()
        .map(|v| rustls::Certificate(v.clone()))
        .collect()
}

// fn load_ocsp(filename: &Option<String>) -> Vec<u8> {
//     let mut ret = Vec::new();
//
//     if let Some(name) = filename {
//         fs::File::open(name)
//             .expect("cannot open ocsp file")
//             .read_to_end(&mut ret)
//             .unwrap();
//     }
//
//     ret
// }