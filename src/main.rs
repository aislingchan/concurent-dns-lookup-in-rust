use std::env;
use std::sync::mpsc::channel;
use std::thread;
use std::net::{TcpStream, ToSocketAddrs, SocketAddr, Shutdown};
use std::io::{Write, Read};

fn main() {
    //Get command line arguments
    let args: Vec<String> = env::args().collect();
    
    if args.len() == 2{
        //formatting hostname for dns lookup
        let hostname = format!("{}:80", &args[1]);

        //preform dns lookup
        let wrapped_addrs_iter = hostname.to_socket_addrs();

        match wrapped_addrs_iter {
            Ok(addrs_iter) => {
                let mut threads = Vec::new();
                let addrs: Vec<SocketAddr> = addrs_iter.collect();
                let num_addrs = addrs.len();

                //Create single Connected Client Thread (CCT) and channel
                let (cat_tx, cct_rx) = channel::<TcpStream>();
                let cct_thread = thread::spawn(move || {
                    match cct_rx.recv() {
                        Ok(mut con_stream) => {
                            //Print peer address of first one received
                            match con_stream.peer_addr() {
                                Ok(peer_addr) => println!("{peer_addr}"),
                                Err(e) => println!("Error occurred retrieving peer address\nError: {e}"),
                            }
                            //Send GET request, print response, close connection
                            let msg = format!("GET / HTTP/1.1\r\nHost: {}\r\n\r\n", &args[1]);
                            match con_stream.write_all(msg.as_bytes()){
                                Ok(_) => (),
                                Err(e) => println!("Error when writing to server\nError: {e}"),
                            }

                            //read response from server
                            let mut response_buf = String::new();
                            //con_stream.set_read_timeout(Some(Duration::new(30,0)));
                            match con_stream.read_to_string(&mut response_buf) {
                                Ok(_) => println!("{response_buf}"),
                                Err(e) => println!("Error when reading from server\nError: {e}"),
                            }

                            match con_stream.shutdown(Shutdown::Both){
                                Ok(_) => (),
                                Err(e) => println!("Error when shutting down TcpStream\nError: {e}"),
                            }
                        },
                        Err(e) => println!("Error: {e}"),
                    }
                    //If not first TcpStream received, then close connection
                    for _ in 1..num_addrs {
                        match cct_rx.recv(){
                            Ok(other_stream) => {
                                match other_stream.shutdown(Shutdown::Both){
                                    Ok(_) => (),
                                    Err(e) => println!("Error when shutting down TcpStream\nError: {e}"),
                                }
                            },
                            Err(e) => println!("Error: {e}"),
                        }
                    }
                });
                threads.push(cct_thread);
                
                //Create connection attempt threads (CATs) for each address returned from DNS lookup
                let mut tranmitters = Vec::new();
                for i in 0..addrs.len(){
                    //Create new Min-CAT channel
                    let (main_tx, cat_rx) = channel::<SocketAddr>();
                    tranmitters.push(main_tx);
                    //Clone transmitter for CAT-CCT channel
                    let new_cat_tx = cat_tx.clone();

                    //Create Connection Attempt Thread
                    let thr = thread::spawn(move ||{
                        match cat_rx.recv() {
                            Ok(val) => {
                                match TcpStream::connect(val) {
                                    Ok(stream) => {
                                        match new_cat_tx.send(stream){
                                            Ok(_) => (),
                                            Err(e) => println!("Error occurred when CAT {i} tried to send a message\nError: {e}"),
                                        }
                                        
                                    },
                                    Err(e) => println!("Error occurred when CAT {i} tried to establish connection\nError: {e}"),
                                }
                            },
                            Err(e) => println!("Error while creating connection attempt thread\nError: {e}"),
                        }             
                    });
                    threads.push(thr);

                }
                //Send a message with IP address to each Connection Attempt Thread
                for (i, trsmtr) in tranmitters.iter().enumerate() {
                    match trsmtr.send(addrs[i]){
                        Ok(_) => (),
                        Err(e) => println!("Error: {e}"),
                    }
                }

                //Wait for all threads to finish
                for th in threads {
                    match th.join(){
                        Ok(_) => (),
                        Err(_) => println!("Error occurred when calling join on a thread"),
                    }
                }
            },
            Err(e) => println!("Error occurred during DNS lookup\nError: {e}"),
        }
    }
    else{
        println!("This program requires 1 hostname as argument");
    }
}
