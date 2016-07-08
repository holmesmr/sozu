#![feature(libc)]
#[macro_use] extern crate log;
extern crate env_logger;
extern crate yxorp;
extern crate openssl;
extern crate time;
extern crate libc;

use std::net::{UdpSocket,ToSocketAddrs};
use std::sync::mpsc::{channel};
use std::fs::File;
use std::io::Read;
use std::env;
use yxorp::network;
use yxorp::messages;
use yxorp::network::metrics::{METRICS,ProxyMetrics};
use openssl::ssl;
use log::{LogRecord,LogLevelFilter,LogLevel};
use env_logger::LogBuilder;

fn main() {
  //env_logger::init().unwrap();
  let pid = unsafe { libc::getpid() };
  let format = move |record: &LogRecord| {
    match record.level() {
    LogLevel::Debug | LogLevel::Trace => format!("{}\t{}\t{}\t{}\t{}\t|\t{}",
      time::now_utc().rfc3339(), time::precise_time_ns(), pid,
      record.level(), record.args(), record.location().module_path()),
    _ => format!("{}\t{}\t{}\t{}\t{}",
      time::now_utc().rfc3339(), time::precise_time_ns(), pid,
      record.level(), record.args())

    }
  };

  let mut builder = LogBuilder::new();
  builder.format(format).filter(None, LogLevelFilter::Info);

  if env::var("RUST_LOG").is_ok() {
   builder.parse(&env::var("RUST_LOG").unwrap());
  }

  builder.init().unwrap();

  info!("MAIN\tstarting up");
  let metrics_socket = UdpSocket::bind("0.0.0.0:0").unwrap();
  let metrics_host   = ("192.168.59.103", 8125).to_socket_addrs().unwrap().next().unwrap();
  METRICS.lock().unwrap().set_up_remote(metrics_socket, metrics_host);
  let metrics_guard = ProxyMetrics::run();
  METRICS.lock().unwrap().gauge("TEST", 42);

  let (sender, rec) = channel::<network::ServerMessage>();
  let (tx, jg) = network::http::start_listener("127.0.0.1:8080".parse().unwrap(), 500, 12000, sender);

  let http_front = messages::HttpFront { app_id: String::from("app_1"), hostname: String::from("lolcatho.st:8080"), path_begin: String::from("/") };
  let http_instance = messages::Instance { app_id: String::from("app_1"), ip_address: String::from("127.0.0.1"), port: 1026 };
  tx.send(network::ProxyOrder::Command(String::from("ID_ABCD"), messages::Command::AddHttpFront(http_front)));
  tx.send(network::ProxyOrder::Command(String::from("ID_EFGH"), messages::Command::AddInstance(http_instance)));
  info!("MAIN\tHTTP -> {:?}", rec.recv().unwrap());
  info!("MAIN\tHTTP -> {:?}", rec.recv().unwrap());

  let (sender2, rec2) = channel::<network::ServerMessage>();

  let options = ssl::SSL_OP_CIPHER_SERVER_PREFERENCE | ssl::SSL_OP_NO_COMPRESSION |
               ssl::SSL_OP_NO_TICKET | ssl::SSL_OP_NO_SSLV2 |
               ssl::SSL_OP_NO_SSLV3 | ssl::SSL_OP_NO_TLSV1;
  let cipher_list = String::from("ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305:\
                              ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:\
                              ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:\
                              DHE-RSA-AES128-GCM-SHA256:DHE-RSA-AES256-GCM-SHA384:\
                              ECDHE-ECDSA-AES128-SHA256:ECDHE-RSA-AES128-SHA256:\
                              ECDHE-ECDSA-AES128-SHA:ECDHE-RSA-AES256-SHA384:\
                              ECDHE-RSA-AES128-SHA:ECDHE-ECDSA-AES256-SHA384:\
                              ECDHE-ECDSA-AES256-SHA:ECDHE-RSA-AES256-SHA:DHE-RSA-AES128-SHA256:\
                              DHE-RSA-AES128-SHA:DHE-RSA-AES256-SHA256:DHE-RSA-AES256-SHA:\
                              ECDHE-ECDSA-DES-CBC3-SHA:ECDHE-RSA-DES-CBC3-SHA:\
                              EDH-RSA-DES-CBC3-SHA:AES128-GCM-SHA256:AES256-GCM-SHA384:\
                              AES128-SHA256:AES256-SHA256:AES128-SHA:AES256-SHA:DES-CBC3-SHA:\
                              !DSS");

  let (tx2, jg2) = network::tls::start_listener("127.0.0.1:8443".parse().unwrap(), 500, 12000, Some((options, cipher_list)), sender2);

  let cert1 = include_str!("../assets/certificate.pem");
  let key1  = include_str!("../assets/key.pem");

  let tls_front = messages::TlsFront { app_id: String::from("app_1"), hostname: String::from("lolcatho.st"), path_begin: String::from("/"), certificate: String::from(cert1), key: String::from(key1), certificate_chain: vec!() };
  tx2.send(network::ProxyOrder::Command(String::from("ID_IJKL"), messages::Command::AddTlsFront(tls_front)));
  let tls_instance = messages::Instance { app_id: String::from("app_1"), ip_address: String::from("127.0.0.1"), port: 1026 };
  tx2.send(network::ProxyOrder::Command(String::from("ID_MNOP"), messages::Command::AddInstance(tls_instance)));

  let cert2 = include_str!("../assets/cert_test.pem");
  let key2  = include_str!("../assets/key_test.pem");

  let tls_front2 = messages::TlsFront { app_id: String::from("app_2"), hostname: String::from("test.local"), path_begin: String::from("/"), certificate: String::from(cert2), key: String::from(key2), certificate_chain: vec!() };
  tx2.send(network::ProxyOrder::Command(String::from("ID_QRST"), messages::Command::AddTlsFront(tls_front2)));
  let tls_instance2 = messages::Instance { app_id: String::from("app_2"), ip_address: String::from("127.0.0.1"), port: 1026 };
  tx2.send(network::ProxyOrder::Command(String::from("ID_UVWX"), messages::Command::AddInstance(tls_instance2)));

  info!("MAIN\tTLS -> {:?}", rec2.recv().unwrap());
  info!("MAIN\tTLS -> {:?}", rec2.recv().unwrap());
  info!("MAIN\tTLS -> {:?}", rec2.recv().unwrap());
  info!("MAIN\tTLS -> {:?}", rec2.recv().unwrap());

  let _ = jg.join();
  info!("MAIN\tgood bye");
}

