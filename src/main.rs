use std::{error, io::{self, Error}, net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs}, os::windows::io::IntoRawSocket, rc::Rc, sync::{Arc, Mutex}, thread};
use log::{info};
use clap::{self, Parser};
use isocountry::CountryCode;

mod pool;
// make a load balancer with the following requirements:
// 1. Multi-strategy (e.g. Round Robin, Least Connections, Weighted Round Robin, Geo-based, etc.)
// 2. Secure. No taking arbitrary strings as input. Protection against Ddos with optional rate-limiting, IP whitelisting.
// 3. two varieties: stateful, and stateless
// 4. Configurable via toml file or cli args
// 5. FFFFFFFAST ZOOOM
// 6. Built-in monitoring & analytics

enum Region {
    Asia(CountryCode),
    Africa(CountryCode),
    Europe(CountryCode),
    Oceania(CountryCode),
    NorthAmerica(CountryCode),
    SouthAmerica(CountryCode),
    Unknown
}


struct Source {
    ip_addr: IpAddr,
    region: Option<Region>
}

impl Source {

    pub fn new(ip_addr: IpAddr) -> Source {
        Self {
            ip_addr,
            region: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Backend {
    url: SocketAddr,
    request_limit_per_second: u64,
    whitelist: Option<Vec<IpAddr>>,
    blacklist: Option<Vec<IpAddr>>,
    served_requests: u64,
}

impl Backend {
    pub fn new(url: impl ToSocketAddrs) -> Result<Backend> {    

        if let Ok(socket_addr) = url.to_socket_addrs() {
            Ok(Self {
                url: socket_addr,
                request_limit_per_second: 10,
                whitelist: None,
                blacklist: None,
                served_requests: 0,
            })
        }
 
    }
}


#[derive(Debug, Clone, Copy)]
enum Strategy {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    Geospatial
}


pub trait Selector {
    fn select_service(&mut self, backends: &Vec<Backend>) -> usize;
}

#[derive(Debug, Clone, Copy)]
pub struct RoundRobinSelector {
    last_idx: usize
}

impl RoundRobinSelector {
    pub fn new() -> RoundRobinSelector {
        Self {
            last_idx: 0
        }
    }
}


impl Selector for RoundRobinSelector {
    fn select_service(&mut self, backends: &Vec<Backend>) -> usize {
        if self.last_idx >= backends.len() - 1 {
            self.last_idx = 0;
            return 0;
        }

        self.last_idx += 1;

        return self.last_idx
    }
}


#[derive(Debug, Clone)]
struct LoadBalancer<T>
where 
    T: Clone + std::fmt::Debug + Send,
    T: Selector
{
    services: Vec<Backend>,
    selector: T,
    session_requests: u64,
}

impl<T> LoadBalancer<T> 
where 
    T: Clone + std::fmt::Debug + Send,
    T: Selector
{

    fn increment_session_requests(&mut self) {
        self.session_requests += 1
    }

    pub fn get_session_requests(&self) -> u64 {
        self.session_requests
    }

    fn select_backend(&mut self) -> usize {
       self.selector.select_service(&self.services)
    }

    pub fn serve_request(&mut self, stream: &TcpStream) {
        if let Ok(addr) = stream.peer_addr() {
            let source = Source::new(addr.ip());
        }

        let backend_idx = self.select_backend();
        if let Some(backend) = self.services.get(backend_idx) {
            let server_conn = TcpStream::connect(backend.)
        }


    }
}

#[derive(Default)]
struct LoadBalancerBuilder {
    balancer_strategy: Option<Strategy>,
    backends: Vec<Backend>,
}

impl LoadBalancerBuilder {
    pub fn new() -> LoadBalancerBuilder {
        Self {
            balancer_strategy: None,
            backends: vec![],
        } 
    }

    pub fn with_service(&mut self, service: Backend) -> &Self {
        self.backends.push(service);

        self
    }

    pub fn with_strategy(&mut self, strategy: Strategy) -> &Self {
        self.balancer_strategy = Some(strategy);

        self 
    }

    pub fn build(&self) -> LoadBalancer {   
        LoadBalancer {
            services: self.backends.clone(),
            strategy: self.balancer_strategy.unwrap_or(Strategy::RoundRobin),
            session_requests: 0,
        }
    }
}


#[derive(Debug, Clone)]
enum LogLevel {
    Error,
    Warning,
    Debug
}

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Args {
    listener_addr: SocketAddr,
    port: u16,
    worker_threads: usize
    // log_level: LogLevel
}


fn main() -> Result<(), io::Error> {


    let listener = TcpListener::bind("127.0.0.1:7878")?;

    let args = Args::parse();

    let pool = pool::ThreadPool::new(args.worker_threads);

    let balancer = LoadBalancerBuilder::default()
        .with_strategy(Strategy::RoundRobin)
        // .with_service()
        .build();

    let balancer = Arc::new(Mutex::new(balancer));

    for stream in listener.incoming().take(2) {
        let balancer = Arc::clone(&balancer);

        if let Ok(stream) = stream {
            pool.execute(move || {
                balancer.lock().unwrap().serve_request(&stream);
            });
        }
    }

    Ok(())
}

