use influx_db_client as influxdb;
use std::env;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Barrier, Mutex, Once, ONCE_INIT};
use std::thread;
use std::time::{Duration, Instant};
use sys_info::hostname;
use buffett_timing::timing;

#[derive(Debug)]
/// define enueration of MetricsComman with the variants of Submit and Flush
enum MetricsCommand {
    Submit(influxdb::Point),
    Flush(Arc<Barrier>),
}

/// define struct of MetricsAgent with the fields of sender
struct MetricsAgent {
    sender: Sender<MetricsCommand>,
}

/// MetricsWriter trait that consists of the behavior provided by a "write" method
trait MetricsWriter {
    
    fn write(&self, points: Vec<influxdb::Point>);
}

/// define struct of DbWriter with the fields of client
struct DbWriter {
    client: Option<influxdb::Client>,
}

/// implementing new  methods on DbWriter structure
impl DbWriter {
    fn new() -> Self {
        DbWriter {
            client: Self::build_client(),
        }
    }

    /// define the function of "build_client",
    /// its return value type is Option <T> where T is influxdb:: Client
    fn build_client() -> Option<influxdb::Client> {
        /// fetches the environment variable key of "INFLUX_HOST" from the current process 
        /// if failed then return "https://dashboard.bitconch.org"
        let host = env::var("INFLUX_HOST")
            .unwrap_or_else(|_| "https://dashboard.bitconch.org".to_string());
        /// fetches the environment variable key of "IINFLUX_DATABASE" from the current process 
        /// if failed then return "scratch"
        let db = env::var("INFLUX_DATABASE").unwrap_or_else(|_| "scratch".to_string());
        /// fetches the environment variable key of "INFLUX_USERNAME" from the current process 
        /// if failed then return "scratch_writer"
        let username = env::var("INFLUX_USERNAME").unwrap_or_else(|_| "scratch_writer".to_string());
        /// fetches the environment variable key of "NFLUX_PASSWORD" from the current process 
        /// if failed then return "topsecret"
        let password = env::var("INFLUX_PASSWORD").unwrap_or_else(|_| "topsecret".to_string());

        debug!("InfluxDB host={} db={} username={}", host, db, username);
        /// create a new influxdb client with https of "host, db, None"
        /// and change the client's user
        let mut client = influxdb::Client::new_with_option(host, db, None)
            .set_authentication(username, password);

        /// set the read timeout value, unit "s" 
        client.set_read_timeout(1 /*second*/);
        /// set the write timeout value, unit "s" 
        client.set_write_timeout(1 /*second*/);

        debug!("InfluxDB version: {:?}", client.get_version());
        ///  "client" is wrapped in "Some" and returned
        Some(client)
    }
}

/// define write function to implementing MetricsWriter trait on DbWriter structure
impl MetricsWriter for DbWriter {
    fn write(&self, points: Vec<influxdb::Point>) {
        /// destructure the fields "client" of DbWriter in "Some" value
        /// format print the length of "points"
        if let Some(ref client) = self.client {
            debug!("submitting {} points", points.len());
            /// write multiple points to the database
            /// if failed then format print the error message
            if let Err(err) = client.write_points(
                influxdb::Points { point: points },
                Some(influxdb::Precision::Milliseconds),
                None,
            ) {
                debug!("DbWriter write error: {:?}", err);
            }
        }
    }
}

/// define default function to implementing Default trait on MetricsAgent structure
impl Default for MetricsAgent {
    fn default() -> Self {
        Self::new(
            Arc::new(DbWriter::new()),
            Duration::from_secs(10),
        )
    }
}

impl MetricsAgent {
    fn new(metrics_writer: Arc<MetricsWriter + Send + Sync>, write_frequency: Duration) -> Self {
        let (sender, receiver) = channel::<MetricsCommand>();
        thread::spawn(move || Self::run(&receiver, &metrics_writer, write_frequency));
        MetricsAgent { sender }
    }

    fn run(
        receiver: &Receiver<MetricsCommand>,
        writer: &Arc<MetricsWriter + Send + Sync>,
        write_frequency: Duration,
    ) {
        trace!("run: enter");
        let mut last_write_time = Instant::now();
        let mut points = Vec::new();

        loop {
            match receiver.recv_timeout(write_frequency / 2) {
                Ok(cmd) => match cmd {
                    MetricsCommand::Flush(barrier) => {
                        debug!("metrics_thread: flush");
                        if !points.is_empty() {
                            writer.write(points);
                            points = Vec::new();
                            last_write_time = Instant::now();
                        }
                        barrier.wait();
                    }
                    MetricsCommand::Submit(point) => {
                        debug!("run: submit {:?}", point);
                        points.push(point);
                    }
                },
                Err(RecvTimeoutError::Timeout) => {
                    trace!("run: receive timeout");
                }
                Err(RecvTimeoutError::Disconnected) => {
                    debug!("run: sender disconnected");
                    break;
                }
            }

            let now = Instant::now();
            if now.duration_since(last_write_time) >= write_frequency && !points.is_empty() {
                debug!("run: writing {} points", points.len());
                writer.write(points);
                points = Vec::new();
                last_write_time = now;
            }
        }
        trace!("run: exit");
    }

    pub fn submit(&self, mut point: influxdb::Point) {
        if point.timestamp.is_none() {
            point.timestamp = Some(timing::timestamp() as i64);
        }
        debug!("Submitting point: {:?}", point);
        self.sender.send(MetricsCommand::Submit(point)).unwrap();
    }

    pub fn flush(&self) {
        debug!("Flush");
        let barrier = Arc::new(Barrier::new(2));
        self.sender
            .send(MetricsCommand::Flush(Arc::clone(&barrier)))
            .unwrap();

        barrier.wait();
    }
}

impl Drop for MetricsAgent {
    fn drop(&mut self) {
        self.flush();
    }
}

fn get_mutex_agent() -> Arc<Mutex<MetricsAgent>> {
    static INIT: Once = ONCE_INIT;
    static mut AGENT: Option<Arc<Mutex<MetricsAgent>>> = None;
    unsafe {
        INIT.call_once(|| AGENT = Some(Arc::new(Mutex::new(MetricsAgent::default()))));
        match AGENT {
            Some(ref agent) => agent.clone(),
            None => panic!("Failed to initialize metrics agent"),
        }
    }
}


pub fn submit(point: influxdb::Point) {
    let agent_mutex = get_mutex_agent();
    let agent = agent_mutex.lock().unwrap();
    agent.submit(point);
}


pub fn flush() {
    let agent_mutex = get_mutex_agent();
    let agent = agent_mutex.lock().unwrap();
    agent.flush();
}


pub fn set_panic_hook(program: &'static str) {
    use std::panic;
    static SET_HOOK: Once = ONCE_INIT;
    SET_HOOK.call_once(|| {
        let default_hook = panic::take_hook();
        panic::set_hook(Box::new(move |ono| {
            default_hook(ono);
            submit(
                influxdb::Point::new("panic")
                    .add_tag("program", influxdb::Value::String(program.to_string()))
                    .add_tag(
                        "thread",
                        influxdb::Value::String(
                            thread::current().name().unwrap_or("?").to_string(),
                        ),
                    )
                    
                    .add_field("one", influxdb::Value::Integer(1))
                    .add_field(
                        "message",
                        influxdb::Value::String(
                            
                            ono.to_string(),
                        ),
                    )
                    .add_field(
                        "location",
                        influxdb::Value::String(match ono.location() {
                            Some(location) => location.to_string(),
                            None => "?".to_string(),
                        }),
                    )
                    .add_field(
                        "host",
                        influxdb::Value::String(
                            hostname().unwrap_or_else(|_| "?".to_string())
                        ),
                    )
                    .to_owned(),
            );
            
            flush();
        }));
    });
}

