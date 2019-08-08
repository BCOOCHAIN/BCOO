extern crate bincode;
#[macro_use]
extern crate clap;
extern crate influx_db_client;
extern crate rayon;
extern crate serde_json;
#[macro_use]
extern crate buffett_core;
extern crate buffett_crypto;
extern crate buffett_metrics;
extern crate buffett_timing;

use clap::{App, Arg};
use influx_db_client as influxdb;
use rayon::prelude::*;
use buffett_core::client::new_client;
use buffett_core::crdt::{Crdt, NodeInfo};
use buffett_core::token_service::DRONE_PORT;
use buffett_crypto::hash::Hash;
use buffett_core::logger;
use buffett_metrics::metrics;
use buffett_core::ncp::Ncp;
use buffett_core::service::Service;
use buffett_crypto::signature::{read_keypair, GenKeys, Keypair,KeypairUtil};
use buffett_core::system_transaction::SystemTransaction;
use buffett_core::thin_client::{sample_leader_by_gossip, ThinClient};
use buffett_timing::timing::{duration_in_milliseconds, duration_in_seconds};
use buffett_core::transaction::Transaction;
use buffett_core::wallet::request_airdrop;
use buffett_core::window::default_window;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::process::exit;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::sleep;
use std::thread::Builder;
use std::time::Duration;
use std::time::Instant;

//mvp001
use buffett_core::asciiart;
use std::io::Write; 

//mvp001
/// define the function of dividing_line and output "----------------------------" through the macro
fn dividing_line() {
    println!("------------------------------------------------------------------------------------------------------------------------");
}
//*

/// define a public structure named NodeStates with parameters tps and tx, 
/// and the parameter types both are u64 and public
pub struct NodeStats {
    pub tps: f64, 
    pub tx: u64,  
}

/// define a function named metrics_submit_token_balance whose parameter is token_balance
fn metrics_submit_token_balance(token_balance: i64) {

    /// use the submit method of the metrics crate and new a Point named "bench-tps" of influxdb,
    /// add a tag named "op" with the value of string “token_balance”,
    /// and a field named "balance" whose value is token_balance of type i64
    metrics::submit(
        influxdb::Point::new("bench-tps")
            .add_tag("op", influxdb::Value::String("token_balance".to_string()))
            .add_field("balance", influxdb::Value::Integer(token_balance as i64))
            .to_owned(),
    );
}

/// define a function named sample_txx_count with parameters exit_signal, maxes, first_tx_count, v, sample_period
fn sample_tx_count(
    exit_signal: &Arc<AtomicBool>,
    maxes: &Arc<RwLock<Vec<(SocketAddr, NodeStats)>>>,
    first_tx_count: u64,
    v: &NodeInfo,
    sample_period: u64,
) {
    /// reference to NodeInfo node information to create a new "client" of ThinClient
    let mut client = new_client(&v);
    /// get the current time 
    let mut now = Instant::now();
    /// get the initial count of transactions on the client
    let mut initial_tx_count = client.transaction_count();
    /// create the mutable variable "max_tps" and initialize it to 0.0
    let mut max_tps = 0.0;
    /// create the mutable variable named "total" 
    let mut total;

    ///  write formatted text of "tpu" to String
    let log_prefix = format!("{:21}:", v.contact_info.tpu.to_string());
    /// infinite loop
    loop {
        /// bound clinet's transactions count to the variable "tx_count"
        let tx_count = client.transaction_count();
        /// assert client's initial count of transactions >= clinet's transactions count is ture
        assert!(
            tx_count >= initial_tx_count,
            "expected tx_count({}) >= initial_tx_count({})",
            tx_count,
            initial_tx_count
        );
        /// get the amount of time elapsed since “now” was created.
        let duration = now.elapsed();
        /// get the current time 
        now = Instant::now();
        /// calculate the value of transactions count - initial count of transactions
        let sample = tx_count - initial_tx_count;
        /// copy "tx_count" into "initial_tx_count"
        initial_tx_count = tx_count;

        /// calculated the sum of the number of whole seconds contained by duration * 1_000_000_000
        /// and the fractional part of duration in nanoseconds
        let ns = duration.as_secs() * 1_000_000_000 + u64::from(duration.subsec_nanos());
        /// calculated tps vlaue by sample * 1_000_000_000 / ns 
        let tps = (sample * 1_000_000_000) as f64 / ns as f64;
        /// if tps > max_tps, then copy "tps" into "max_tps"
        if tps > max_tps {
            max_tps = tps;
        }
        /// if tx_count > first_tx_count, 
        /// then calculate the value of tx_count - first_tx_conut and bound it to toal
        if tx_count > first_tx_count {
            total = tx_count - first_tx_count;
        /// otherwise total = 0
        } else {
            total = 0;
        }
        
        
        /// starting variable named "node_role" with an underscore to avoid getting unused variable warnings
        /// and bound "Node's Roles" to "node_role"
        let _node_role="Node's Roles";
        
        if v.id == v.leader_id {
            let _node_role = "Leader   ";
        } else {
            let _node_role = "Validator";
        }
        let mut node_location = "Node Location";
        let node_ip: Vec<&str> = log_prefix.split(|c| c == '.' || c == ':').collect();
        if node_ip[0] == "192" && node_ip[1] == "168" {
            node_location = "LOCAL";
        } else if node_ip[0] == "148"
            && node_ip[1] == "153"
            && node_ip[2] == "36"
            && node_ip[3] == "220"
        {
            node_location = "US_NEW_YORK";
        } else if node_ip[0] == "148"
            && node_ip[1] == "153"
            && node_ip[2] == "50"
            && node_ip[3] == "162"
        {
            node_location = "DE_FRANKFURT";
        } else if node_ip[0] == "148"
            && node_ip[1] == "153"
            && node_ip[2] == "25"
            && node_ip[3] == "50"
        {
            node_location = "NE_ARMSTERDAM";

        } else if node_ip[0] == "164"
            && node_ip[1] == "52"
            && node_ip[2] == "39"
            && node_ip[3] == "162"
        {
            node_location = "SG_SINGAOPORE";
        } else if node_ip[0] == "118"
            && node_ip[1] == "186"
            && node_ip[2] == "39"
            && node_ip[3] == "238"
        {
            node_location = "CN_PEKING";
        }
        
        
        println!(
            "| {0:13} {1:<8} {2:3}{3:20}|{4:>15}{5:>10.2} |{6:>15}{7:>13} |{8:19}{9:9}",
            node_location,
            _node_role,
            "IP:",
            log_prefix,
            "Real-Time TPS:",
            tps,
            " Txs Proccessed:",
            sample,
            " Total Transactions:",
            total
        );

        /// sleep 0
        sleep(Duration::new(sample_period, 0));

        /// loads the value of "exit_signal" is ture (no ordering constraints, only atomic operations)
        /// print "log_prefix" through macros
        if exit_signal.load(Ordering::Relaxed) {
            println!("\n| Exit Signal detected, kill threas for this Node:{}", log_prefix);
            /// call the function of print_animation_arrows() 
            print_animation_arrows();
            /// instantiate NodeStates structure
            let stats = NodeStats {
                tps: max_tps,
                tx: total,
            };
            /// push the value of tpu and stats onto the end of "maxes"
            maxes.write().unwrap().push((v.contact_info.tpu, stats));
            /// exit the loop
            break;
        }
    }
}

/// define function named send_barrier_transaction
fn send_barrier_transaction(barrier_client: &mut ThinClient, last_id: &mut Hash, id: &Keypair) {
    /// get the current time
    let transfer_start = Instant::now();
    /// declare a mutable variable "sampel_cnt" and initialization the value of 0
    let mut sampel_cnt = 0;
/// start loop
    loop {
/// if sampel_cnt > 0 and sampel_cnt % 8 == 0
        if sampel_cnt > 0 && sampel_cnt % 8 == 0 {
        }

/// then get ThinClient's last id 
        *last_id = barrier_client.get_last_id();
/// get barrier_client's t transfer result, 
/// if error, then output "Unable to send barrier transaction"
        let signature = barrier_client
            .transfer(0, &id, id.pubkey(), last_id)
            .expect("Unable to send barrier transaction");

/// check the existence of barrier_client‘signature
        let confirmatiom = barrier_client.sample_by_signature(&signature);
/// calculate the interval between transfer_start time and current time in milliseconds
        let duration_ms = duration_in_milliseconds(&transfer_start.elapsed());
/// if barrier client'signature exists
        if confirmatiom.is_ok() {

/// use the submit method of the metrics crate and add a new data to "bench-tps" data table of influxdb,
/// add a tag named "op" with the value of String “token_balance”,
/// add a field named "balance" whose value is mutable variable sampel_cnt with an initial value of 0
/// add a field named "duration"with the interval between transfer_start time and current time in milliseconds
            metrics::submit(
                influxdb::Point::new("bench-tps")
                    .add_tag(
                        "op",
                        influxdb::Value::String("send_barrier_transaction".to_string()),
                    ).add_field("sampel_cnt", influxdb::Value::Integer(sampel_cnt))
                    .add_field("duration", influxdb::Value::Integer(duration_ms as i64))
                    .to_owned(),
            );

/// get balance through the ID public key
/// and if there is fail to get balance, then willoutput "Failed to get balance"           
            let balance = barrier_client
                .sample_balance_by_key_plus(
                    &id.pubkey(),
                    &Duration::from_millis(100),
                    &Duration::from_secs(10),
                ).expect("Failed to get balance");
/// if balance !=1, then will panic
            if balance != 1 {
                panic!("Expected an account balance of 1 (balance: {}", balance);
            }
/// break loop
            break;
        }


/// if the interval between transfer_start time and current time in milliseconds > 1000 * 60 * 3
/// then print the error message and exit the process 
        if duration_ms > 1000 * 60 * 3 {
            println!("Error: Couldn't confirm barrier transaction!");
            exit(1);
        }
/// get ThinClient's last id 
        let new_last_id = barrier_client.get_last_id();
        if new_last_id == *last_id {
            if sampel_cnt > 0 && sampel_cnt % 8 == 0 {
                println!("last_id is not advancing, still at {:?}", *last_id);
            }
        } else {
            *last_id = new_last_id;
        }

/// return the value of sampel_cnt += 1
        sampel_cnt += 1;
    }
}

/// define function of generate_txs
fn generate_txs(
    shared_txs: &Arc<RwLock<VecDeque<Vec<Transaction>>>>,
    id: &Keypair,
    keypairs: &[Keypair],
    last_id: &Hash,
    threads: usize,
    reclaim: bool,
) {
/// get the length of keypairs
    let tx_count = keypairs.len();
    
/// call the function of dividing_line()
    dividing_line();
    println!(
        "{0: <2}{1: <40}: {2: <10}",
        "|", "Transactions to be signed", tx_count
    );
    println!(
        "{0: <2}{1: <40}: {2: <10}",
        "|", "Reclaimed Tokens", reclaim
    );
    dividing_line();
    
    println!(
        "{0: <2}{1: <40}: {2: <60}",
        "|", "Status", "Signing Started"
    );
/// call the function of dividing_line()
    dividing_line();
    
/// get the current time
    let signing_start = Instant::now();
/// traverse keypairs, generating transaction for each keypair in keypairs，and transforms it to dynamic Vec collection,
/// if reclaim is false, the sender of the transaction is id and the recipient is keypair，
/// if reclaim is true, the sender of the transaction is keypair and the receiver is id
    let transactions: Vec<_> = keypairs
        .par_iter()
        .map(|keypair| {
            if !reclaim {
                Transaction::system_new(&id, keypair.pubkey(), 1, *last_id)
            } else {
                Transaction::system_new(keypair, id.pubkey(), 1, *last_id)
            }
        }).collect();
/// get the amount of time elapsed since “now” was created
    let duration = signing_start.elapsed();
/// calculated the vlaue of duration * 1_000_000_000 converted to seconds + duration  converted to nanoseconds.
    let ns = duration.as_secs() * 1_000_000_000 + u64::from(duration.subsec_nanos());
    let bsps = (tx_count) as f64 / ns as f64;

/// call the function of dividing_line()    
    dividing_line();
    println!(
        "{0: <2}{1: <40}: {2: <60}",
        "|", "Status", "Signing Finished"
    );
    println!(
        "{0: <2}Transaction Generated :{1:?} ,Time Consumed:{2:.2}, Speed:{3:?} in the last {4:.2 } milliseconds",
        "|",
        tx_count,
        ns/1_000_000_000_u64,
        bsps * 1_000_000_f64 * 1000_f64,
        duration_in_milliseconds(&duration)
        
    );
    dividing_line();

/// use the submit method of the metrics crate and add a new data to "bench-tps" data table of influxdb,
/// add a tag named "op" with the value of String “generate_txs”,
/// add a field named "duration"with the interval between duration time and current time in milliseconds
    metrics::submit(
        influxdb::Point::new("bench-tps")
            .add_tag("op", influxdb::Value::String("generate_txs".to_string()))
            .add_field(
                "duration",
                influxdb::Value::Integer(duration_in_milliseconds(&duration) as i64),
            ).to_owned(),
    );

/// calculate the value of the length of Vec's transactions / threads
    let sz = transactions.len() / threads;
/// in sz steps, slice the transaction Vec and transforms it into Vec collection
    let chunks: Vec<_> = transactions.chunks(sz).collect();
    {
/// unwraps shared_txs‘s Transaction result. yielding the content of an Ok, panics if the value is an Err
        let mut shared_txs_wl = shared_txs.write().unwrap();
/// traverse the chunks Vec, adding the elements of the chunks Vec to the shared_txs_wl Vec
        for chunk in chunks {
            shared_txs_wl.push_back(chunk.to_vec());
        }
    }
}

/// define function of send_transaction
fn send_transaction(
    exit_signal: &Arc<AtomicBool>,
    shared_txs: &Arc<RwLock<VecDeque<Vec<Transaction>>>>,
    leader: &NodeInfo,
    shared_tx_thread_count: &Arc<AtomicIsize>,
    total_tx_sent_count: &Arc<AtomicUsize>,
) {
/// refer to NodeInfo node information to create a new client
    let client = new_client(&leader);
    println!("| Begin to sendout transactions in parrallel");
/// start loop
    loop {
        let txs;
        {
/// unwraps shared_txs‘s Transaction content
            let mut shared_txs_wl = shared_txs.write().unwrap();
/// pop shared_txs‘s Transaction content's first element
            txs = shared_txs_wl.pop_front();
        }
/// if txs is not null, then add 1 to shared_tx_thread_count (only in atomic operations)
        if let Some(txs0) = txs {
            shared_tx_thread_count.fetch_add(1, Ordering::Relaxed);

/// get the length of txs0         
            let tx_len = txs0.len();
/// get current time
            let transfer_start = Instant::now();
/// traverse txs0, send tx transaction and return signature
            for tx in txs0 {
                client.transfer_signed(&tx).unwrap();
            }
/// add -1 to shared_tx_thread_count (only in atomic operations)
            shared_tx_thread_count.fetch_add(-1, Ordering::Relaxed);
/// add txs0's length to total_tx_sent_count (only in atomic operations)
            total_tx_sent_count.fetch_add(tx_len, Ordering::Relaxed);
            println!(
                "| > 1 MU sent, to {} in {} ms, TPS: {} ",
                leader.contact_info.tpu,
                duration_in_milliseconds(&transfer_start.elapsed()),
                tx_len as f32 / duration_in_seconds(&transfer_start.elapsed()),
            );
/// use the submit method of the metrics crate and add a new data to "bench-tps" data table of influxdb,
/// add a tag named "op" with the value of String “send_transaction”,
/// add a field named "duration"with the interval between duration time and current time in milliseconds
/// add a field named "count"with the value of txs0's length
            metrics::submit(
                influxdb::Point::new("bench-tps")
                    .add_tag("op", influxdb::Value::String("send_transaction".to_string()))
                    .add_field(
                        "duration",
                        influxdb::Value::Integer(duration_in_milliseconds(&transfer_start.elapsed()) as i64),
                    ).add_field("count", influxdb::Value::Integer(tx_len as i64))
                    .to_owned(),
            );
        }
/// determine whether to exit, if there is exit signal, then break the loop
        if exit_signal.load(Ordering::Relaxed) {
            break;
        }
    }
}

/// define a function of airdrop_tokens
fn airdrop_tokens(client: &mut ThinClient, leader: &NodeInfo, id: &Keypair, tx_count: i64) {
/// get leader's airdrop address
    let mut drone_addr = leader.contact_info.tpu;
/// set the port of the airdrop address to DRONE_PORT
    drone_addr.set_port(DRONE_PORT);
/// obtain the balance through id pubkey,
/// return the balance if it is obtained, and return 0 if it is not obtainable
    let starting_balance = client.sample_balance_by_key(&id.pubkey()).unwrap_or(0);
/// call the function of metrics_submit_token_balance
    metrics_submit_token_balance(starting_balance);
/// output the value of the balance through macros
    println!("starting balance {}", starting_balance);
/// if balance < tx_count, then output，then output "| Begin to prepare data and send some Transactions:" through macros
    if starting_balance < tx_count {
        
        println!("| Begin to prepare data and send some Transactions:",);
/// call the function dividing_line()
        dividing_line();
/// call the function print_animation_arrows()
        print_animation_arrows();
        

/// calculate the value of tx_count - starting_balance        
        let airdrop_amount = tx_count - starting_balance;
        println!(
            "Airdropping {:?} tokens from {} for {}",
            airdrop_amount,
            drone_addr,
            id.pubkey(),
        );
/// send airdrop request to drone_addr with the number of requests airdrop_amount, 
/// if there is error， then will be panic
        if let Err(e) = request_airdrop(&drone_addr, &id.pubkey(), airdrop_amount as u64) {
            panic!(
                "Error requesting airdrop: {:?} to addr: {:?} amount: {}",
                e, drone_addr, airdrop_amount
            );
        }

    
/// get the value of the balance
        let mut current_balance = starting_balance;
/// 20 cycles
        for _ in 0..20 {
/// sleep 500 millisenconds
            sleep(Duration::from_millis(500));
/// obtained the balance by the public key, and if it is obtained, the balance is returned,
/// and if the balance is not obtained, the output the error message through the macro.
            current_balance = client.sample_balance_by_key(&id.pubkey()).unwrap_or_else(|e| {
                println!("airdrop error {}", e);
                starting_balance
            });
/// if the value of starting_balance not equal current_balance，then break the loop
            if starting_balance != current_balance {
                break;
            }
            
            println!(
                "Current balance of {} is {}...",
                id.pubkey(),
                current_balance
            );
            
        }
/// call the function of metrics_submit_token_balance(current_balance)
        metrics_submit_token_balance(current_balance);
        if current_balance - starting_balance != airdrop_amount {
            println!(
                "Airdrop failed! {} {} {}",
                id.pubkey(),
                current_balance,
                starting_balance
            );
/// exit the process
            exit(1);
        }
    }
}

/// define a function of print_status_and_report
fn print_status_and_report(
    maxes: &Arc<RwLock<Vec<(SocketAddr, NodeStats)>>>,
    _sample_period: u64,
    tx_send_elapsed: &Duration,
    _total_tx_send_count: usize,
) {
    
    let mut max_of_maxes = 0.0;
    let mut max_tx_count = 0;
    let mut nodes_with_zero_tps = 0;
    let mut total_maxes = 0.0;
    println!(" Node address        |       Max TPS | Total Transactions");
    println!("---------------------+---------------+--------------------");

/// traversal maxes
    for (sock, stats) in maxes.read().unwrap().iter() {
/// Match with NodeStats structure’ member of tx
        let maybe_flag = match stats.tx {
/// if tx is 0 , then return "!!!!!" to maybe_flag
            0 => "!!!!!",
/// otherwise return ""
            _ => "",
        };

        println!(
            "{:20} | {:13.2} | {} {}",
            (*sock).to_string(),
            stats.tps,
            stats.tx,
            maybe_flag
        );

        if stats.tps == 0.0 {
            nodes_with_zero_tps += 1;
        }
        total_maxes += stats.tps;

        if stats.tps > max_of_maxes {
            max_of_maxes = stats.tps;
        }
        if stats.tx > max_tx_count {
            max_tx_count = stats.tx;
        }
    }

    if total_maxes > 0.0 {
        let num_nodes_with_tps = maxes.read().unwrap().len() - nodes_with_zero_tps;
        let average_max = total_maxes / num_nodes_with_tps as f64;
        println!("====================================================================================");
        println!("| Normal TPS:{:.2}",average_max);
        println!("====================================================================================");
        
       
    }

    println!("====================================================================================");
    println!("| Peak TPS:{:.2}",max_of_maxes);
    println!("====================================================================================");
    

    println!(
        "\tAverage TPS: {}",
        max_tx_count as f32 / duration_in_seconds(tx_send_elapsed)
    );
}


/// define a function of should_switch_directions, the type of return value is bool
fn should_switch_directions(num_tokens_per_account: i64, i: i64) -> bool {
    i % (num_tokens_per_account / 4) == 0 && (i >= (3 * num_tokens_per_account) / 4)
}

/// define a function of print_animation_arrows()
fn print_animation_arrows(){
    print!("|\n|");
/// cycle 5 times
    for _ in 0..5 {
        print!(".");
        sleep(Duration::from_millis(300));
/// refresh standard I/O, if error, then output "some error message"
        std::io::stdout().flush().expect("some error message");
    }
    print!("\n|\n");
    
}

fn leader_node_selection(){
    dividing_line();
    println!("| {:?}","Selecting Transaction Validator Nodes from the Predefined High-Reputation Nodes List.");
    sleep(Duration::from_millis(100));
    std::io::stdout().flush().expect("some error message");
    println!("| {:?}","HRNL is populated with hundreds, even thousands of candidate nodes.");
    sleep(Duration::from_millis(100));
    std::io::stdout().flush().expect("some error message");
    println!("| {:?}","An random process is evoked to select up to 21 nodes from this list.");
    sleep(Duration::from_millis(100));
    std::io::stdout().flush().expect("some error message");
    println!("| {:?}","These 21 nodes are responsible for validating transactions on the DLT network.");
    sleep(Duration::from_millis(100));
    std::io::stdout().flush().expect("some error message");
    println!("| {:?}","They are further grouped into one leader node and 20 voting nodes.");
    sleep(Duration::from_millis(100));
    std::io::stdout().flush().expect("some error message");
    println!("| {:?}","For MVP demo, we only use 5 nodes from 5 different countries.");
    sleep(Duration::from_millis(100));
    std::io::stdout().flush().expect("some error message");
    dividing_line();
    sleep(Duration::from_millis(100));
    std::io::stdout().flush().expect("some error message");
    print_animation_arrows();
    dividing_line();
    println!("| {:?}","Transaction Validator Nodes Selection Process Complete!!");
    dividing_line();
}


fn main() {
/// Initialization log
    logger::setup();
/// insert data into the "panic" data table 
    metrics::set_panic_hook("bench-tps");
/// create a command line  program named "bitconch-bench-tps" 
/// adds argument to the list of valid possibilities
    let matches = App::new("bitconch-bench-tps")
        .version(crate_version!())
        .arg(
/// creates a new instance of Arg named "network" 
            Arg::with_name("network")
/// sets the short version of the argument "network"
                .short("n")
/// sets the long version of the argument "network"
                .long("network")
/// specifies the name for value of option or positional arguments inside of help documentation
                .value_name("HOST:PORT")
/// when running the specifies argument is "network"
                .takes_value(true)
/// Sets the short help text of the argument， when input -h  
/// then will output the help information  "Rendezvous with the network at this gossip entry point; defaults to 127.0.0.1:8001"
                .help("Rendezvous with the network at this gossip entry point; defaults to 127.0.0.1:8001"),
        )
        .arg(
            Arg::with_name("identity")
                .short("i")
                .long("identity")
                .value_name("PATH")
                .takes_value(true)
                .required(true)
                .help("File containing a client identity (keypair)"),
        )
        .arg(
            Arg::with_name("num-nodes")
                .short("N")
                .long("num-nodes")
                .value_name("NUM")
                .takes_value(true)
                .help("Wait for NUM nodes to converge"),
        )
        .arg(
            Arg::with_name("reject-extra-nodes")
                .long("reject-extra-nodes")
                .help("Require exactly `num-nodes` on convergence. Appropriate only for internal networks"),
        )
        .arg(
            Arg::with_name("threads")
                .short("t")
                .long("threads")
                .value_name("NUM")
                .takes_value(true)
                .help("Number of threads"),
        )
        .arg(
            Arg::with_name("duration")
                .long("duration")
                .value_name("SECS")
                .takes_value(true)
                .help("Seconds to run benchmark, then exit; default is forever"),
        )
        .arg(
            Arg::with_name("converge-only")
                .long("converge-only")
                .help("Exit immediately after converging"),
        )
        .arg(
            Arg::with_name("sustained")
                .long("sustained")
                .help("Use sustained performance mode vs. peak mode. This overlaps the tx generation with transfers."),
        )
        .arg(
            Arg::with_name("tx_count")
                .long("tx_count")
                .value_name("NUM")
                .takes_value(true)
                .help("Number of transactions to send per batch")
        )
/// starts the parsing process, upon a failed parse an error will be displayed 
/// and the process will exit with the appropriate error code. 
        .get_matches();

/// get the value of "network", if fails then will output the error message, then exit the program
    let network = if let Some(addr) = matches.value_of("network") {
        addr.parse().unwrap_or_else(|e| {
            eprintln!("failed to parse network: {}", e);
            exit(1)
        })
/// if command line program's argument is not specified, then will return "127.0.0.1:8001"
    } else {
        socketaddr!("127.0.0.1:8001")
    };

/// get keypair according to the parameter "identity" of the command line program,
/// if fails，then will display the error message "can't read client identity"
    let id =
        read_keypair(matches.value_of("identity").unwrap()).expect("can't read client identity");

/// get the value of "threads" 
/// if fails，then will display the error message "can't parse threads"
    let threads = if let Some(t) = matches.value_of("threads") {
        t.to_string().parse().expect("can't parse threads")
/// if command line program's argument is not specified, then will return “4usize” to threads
    } else {
        4usize
    };

/// get the value of "num-nodes" 
/// if fails，then will display the error message "can't parse num-nodes"
    let num_nodes = if let Some(n) = matches.value_of("num-nodes") {
        n.to_string().parse().expect("can't parse num-nodes")
/// if command line program's argument is not specified, then will return “1usize” to num_nodes
    } else {
        1usize
    };

/// get the value of "duration" ，creates a new Duration
/// if fails，then will display the error message "can't parse duration"
    let duration = if let Some(s) = matches.value_of("duration") {
        Duration::new(s.to_string().parse().expect("can't parse duration"), 0)
/// if command line program's argument is not specified, then will return  the largest value to duration
    } else {
        Duration::new(std::u64::MAX, 0)
    };

/// get the value of "tx_count" 
/// if fails，then will display the error message "can't parse tx_count"
    let tx_count = if let Some(s) = matches.value_of("tx_count") {
        s.to_string().parse().expect("can't parse tx_count")
/// else return 500_000 to tx_count
    } else {
        500_000
    };

/// check if the argument "sustained" was present
    let sustained = matches.is_present("sustained");

/// the `ascii_art` module implement fancy ascii arts
    asciiart::welcome();
    dividing_line();
    leader_node_selection();

    
    println!(
        "{0: <2}{1: <40}: {2: <60}",
        "|", "Search for Leader Node On Network", network
    );
    dividing_line();
    print_animation_arrows();


/// get leader node information on the network,
/// if fails，then will display the error message "unable to find leader on network"
    let leader = sample_leader_by_gossip(network, None).expect("unable to find leader on network");

/// define exit signal, default initial value is false
    let exit_signal = Arc::new(AtomicBool::new(false));
    
    dividing_line();
    println!(
        "| Leader Node is found!, ID: {:?}",
        &leader.id
    );
    dividing_line();
    sleep(Duration::from_millis(100));
    
/// call the function of converge, search the effective nodes on the network
    let (nodes, leader, ncp) = converge(&leader, &exit_signal, num_nodes);

/// if the length of the node < the number of nodes, then print the error message and exit the program
    if nodes.len() < num_nodes {
        println!(
            "Error: Insufficient nodes discovered.  Expecting {} or more",
            num_nodes
        );
        exit(1);
    }
/// if command program's argument is "reject-extra-nodes", and the length of the node > the number of nodes
/// then print the error message and exit the program
    if matches.is_present("reject-extra-nodes") && nodes.len() > num_nodes {
        println!(
            "Error: Extra nodes discovered.  Expecting exactly {}",
            num_nodes
        );
        exit(1);
    }

/// if leader is a None value, then print "no leader", and exit program
    if leader.is_none() {
        println!("no leader");
        exit(1);
    }

/// if command line program's argument "converge-only", then return it
    if matches.is_present("converge-only") {
        return;
    }

    let leader = leader.unwrap();

    //mvp001
    dividing_line();
    println!(
        "{0: <2}{1: <40}: {2: <60}",
        "|", "Leader Node Contact Information", leader.contact_info.rpu
    );
    println!(
        "{0: <2}{1: <40}: {2: <60}",
        "|", "Leader Node ID", leader.id
    );
    dividing_line();
    //*
    //println!("leader is at {} {}", leader.contact_info.rpu, leader.id);
    
/// refer the leader node information to create different new client
    let mut client = new_client(&leader);
    let mut barrier_client = new_client(&leader);

/// declare a mutable variable array seed of type U8 with 32 elements
///  and initial values is 0
    let mut seed = [0u8; 32];
/// copy the little-endian-encoded public key bytes of the id into  seed
    seed.copy_from_slice(&id.public_key_bytes()[..32]);
/// new a GenKeys with the parameter seed
    let mut rnd = GenKeys::new(seed);

    //mvp
    println!("| Begin to prepare data and send some Transactions:");
    dividing_line();
    print_animation_arrows();
    //println!("Creating {} keypairs...", tx_count / 2);
    println!(
        "{0: <2}{1: <40}: {2: <60}",
        "|",
        "Create Key Pairs",
        tx_count / 2
    );
    //*

/// generate keypairs of type Vec based on the value of tx_count / 2
    let keypairs = rnd.gen_n_keypairs(tx_count / 2);
/// generate the keypair array with 1 as the parameter and pop the element to barrier_id
    let barrier_id = rnd.gen_n_keypairs(1).pop().unwrap();

    //mvp001
    print_animation_arrows();
    println!(
        "{0: <2}{1: <40}: {2: <60}",
        "|", "Issue Tokens", "Yes, issue some tokens to each account."
    );
    //*
    //println!("Get tokens...");
    let num_tokens_per_account = 20;

    // Sample the first keypair, see if it has tokens, if so then resume
    // to avoid token loss
/// get the balance through the first pubkey () of keypairs, if not then return 0
    let keypair0_balance = client.sample_balance_by_key(&keypairs[0].pubkey()).unwrap_or(0);

/// if num_tokens_per_account > keypair0_balance, then call the function of airdrop_tokens()
    if num_tokens_per_account > keypair0_balance {
        airdrop_tokens(
            &mut client,
            &leader,
            &id,
            (num_tokens_per_account - keypair0_balance) * tx_count,
        );
    }
/// call the function of airdrop_tokens()
    airdrop_tokens(&mut barrier_client, &leader, &barrier_id, 1);

    
/// get leader's last id 
    let mut last_id = client.get_last_id();
    

/// get leader's transactions count
    let first_tx_count = client.transaction_count();
    println!("Initial transaction count {}", first_tx_count);

    
/// creat a new array
    let maxes = Arc::new(RwLock::new(Vec::new()));
    let sample_period = 1; 
    println!("Sampling TPS every {} second...", sample_period);
/// use multithread to execute the sample_tx_count function
    let v_threads: Vec<_> = nodes
        .into_iter()
        .map(|v| {
/// exit_signal get a copy, leaving the original value in place
            let exit_signal = exit_signal.clone();
            let maxes = maxes.clone();
/// create a thread named "bitconch-client-sample", 
/// call the sample_tx_count function, transfer the results to Vec 
            Builder::new()
                .name("bitconch-client-sample".to_string())
                .spawn(move || {
                    sample_tx_count(&exit_signal, &maxes, first_tx_count, &v, sample_period);
                }).unwrap()
        }).collect();

/// creates an empty VecDeque.
    let shared_txs: Arc<RwLock<VecDeque<Vec<Transaction>>>> =
        Arc::new(RwLock::new(VecDeque::new()));

    let shared_tx_active_thread_count = Arc::new(AtomicIsize::new(0));
    let total_tx_sent_count = Arc::new(AtomicUsize::new(0));

/// use multithread to execute the function of sample_tx_count
    let s_threads: Vec<_> = (0..threads)
        .map(|_| {
/// get a copy, leaving the original value in place
            let exit_signal = exit_signal.clone();
            let shared_txs = shared_txs.clone();
            let leader = leader.clone();
            let shared_tx_active_thread_count = shared_tx_active_thread_count.clone();
            let total_tx_sent_count = total_tx_sent_count.clone();
/// create a thread named "bitconch-client-sender", 
/// call the sample_tx_count function, and transfer the results to Vec 
            Builder::new()
                .name("bitconch-client-sender".to_string())
                .spawn(move || {
                    send_transaction(
                        &exit_signal,
                        &shared_txs,
                        &leader,
                        &shared_tx_active_thread_count,
                        &total_tx_sent_count,
                    );
                }).unwrap()
        }).collect();

    
/// get the current time
    let start = Instant::now();
/// define mutable variable reclaim_tokens_back_to_source_account with an initial value of false
    let mut reclaim_tokens_back_to_source_account = false;
/// get balance through the first pubkey () of keypairs
    let mut i = keypair0_balance;
/// while the amount of time elapsed since “now” was created < command line  program's value duration
    while start.elapsed() < duration {
/// then obtained the balance by the public key, if it's faile then ruturn -1
        let balance = client.sample_balance_by_key(&id.pubkey()).unwrap_or(-1);
/// call the function of metrics_submit_token_balance()
        metrics_submit_token_balance(balance);
/// call the function of generate_txs()
        generate_txs(
            &shared_txs,
            &id,
            &keypairs,
            &last_id,
            threads,
            reclaim_tokens_back_to_source_account,
        );
///  if the argument "sustained" not-exist
        if !sustained {
/// while shared_tx_active_thread_count > 0 is ture
            while shared_tx_active_thread_count.load(Ordering::Relaxed) > 0 {
/// sleep 100 milliseconds
                sleep(Duration::from_millis(100));
            }
        }
/// call the function of send_barrier_transaction()
        send_barrier_transaction(&mut barrier_client, &mut last_id, &barrier_id);

        i += 1;
        if should_switch_directions(num_tokens_per_account, i) {
            reclaim_tokens_back_to_source_account = !reclaim_tokens_back_to_source_account;
        }
    }

/// uses true to replace the value of false
    exit_signal.store(true, Ordering::Relaxed);

    dividing_line(); //mvp001
    println!("| Kill all the remaining threads.");
    print_animation_arrows();
/// loop v_threads array
    for t in v_threads {
/// Waits for the v_threads associated thread to finish running, 
/// if the v_threads thread goes wrong, then output the error information through macros
        if let Err(err) = t.join() {
            println!("  join() failed with: {:?}", err);
        }
    }

    // join the tx send threads
    //println!("Waiting for transmit threads...");
/// loop s_threads array
    for t in s_threads {
/// Waits for the s_threads associated thread to finish running, 
/// if the s_threads thread goes wrong, then output the error information through macros
        if let Err(err) = t.join() {
            println!("  join() failed with: {:?}", err);
        }
    }

/// obtained the balance by the public key, if it's faile then ruturn -1
    let balance = client.sample_balance_by_key(&id.pubkey()).unwrap_or(-1);
    metrics_submit_token_balance(balance);

/// call the function
    print_status_and_report(
        &maxes,
        sample_period,
        &start.elapsed(),
        total_tx_sent_count.load(Ordering::Relaxed),
    );

    // join the crdt client threads
/// running threads in ncp, waits for the associated thread to finish.
    ncp.join().unwrap();
}

/// define the function of converge(), whose return values type are Vec<NodeInfo>, Option<NodeInfo>, Ncp
fn converge(
    leader: &NodeInfo,
    exit_signal: &Arc<AtomicBool>,
    num_nodes: usize,
) -> (Vec<NodeInfo>, Option<NodeInfo>, Ncp) {
    //lets spy on the network
/// creat node, gossip_socket
    let (node, gossip_socket) = Crdt::spy_node();
/// create Crdt with a parameter of node, if goes wrong then will output "Crdt:: new" 
    let mut spy_crdt = Crdt::new(node).expect("Crdt::new");
/// insert leader's NodeInfo into spy_crdt
    spy_crdt.insert(&leader);
/// Set NodeInfo's id to leader's id
    spy_crdt.set_leader(leader.id);
/// create Arc with the argument of spy_crdt
/// locks this rwlock with shared read access, blocking the current thread until it can be acquired.
    let spy_ref = Arc::new(RwLock::new(spy_crdt));
/// constructs a new Arc with the argument of default_window()
    let window = Arc::new(RwLock::new(default_window()));
/// create Ncp with the argument of &spy_ref, window, None, gossip_socket, exit_signal.clone()
    let ncp = Ncp::new(&spy_ref, window, None, gossip_socket, exit_signal.clone());
/// initialize array
    let mut v: Vec<NodeInfo> = vec![];
    // wait for the network to converge, 30 seconds should be plenty
/// loop 30 times
    for _ in 0..30 {
        {
/// read the data of spy_ref's spy_crdt node 
            let spy_ref = spy_ref.read().unwrap();

/// output the node's information through macros
            println!("{}", spy_ref.node_info_trace());

/// if the node has values
            if spy_ref.leader_data().is_some() {
/// get valid communication address and converting it into a collection
                v = spy_ref
                    .table
                    .values()
                    .filter(|x| Crdt::is_valid_address(&x.contact_info.rpu))
                    .cloned()
                    .collect();

                if v.len() >= num_nodes {
                    println!("CONVERGED!");
                    break;
                } else {
                    println!(
                        "{} node(s) discovered (looking for {} or more)",
                        v.len(),
                        num_nodes
                    );
                }
            }
        }
        sleep(Duration::new(1, 0));
    }
/// clone the data of the leader node
    let leader = spy_ref.read().unwrap().leader_data().cloned();
    (v, leader, ncp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_switch_directions() {
        assert_eq!(should_switch_directions(20, 0), false);
        assert_eq!(should_switch_directions(20, 1), false);
        assert_eq!(should_switch_directions(20, 14), false);
        assert_eq!(should_switch_directions(20, 15), true);
        assert_eq!(should_switch_directions(20, 16), false);
        assert_eq!(should_switch_directions(20, 19), false);
        assert_eq!(should_switch_directions(20, 20), true);
        assert_eq!(should_switch_directions(20, 21), false);
        assert_eq!(should_switch_directions(20, 99), false);
        assert_eq!(should_switch_directions(20, 100), true);
        assert_eq!(should_switch_directions(20, 101), false);
    }
}
