/*
slim_r_server

# Slim Rust Server

The project is to make an extremely minimal Rust server, 

## A very basic server to: 
- get a post request
- that uses a minimal load-management system
- has https support
- is crash-resistant
- that calls an another program e.g. python or llama cpp, though there is a rust_llamacpp if that works.
-- python: that runs a run script as an external operation e.g. run $python script.py -{"hell world"}

Not requirements:
- the this is not a normal server
- this server does not need many async 'worker' or other threads

requirements:
- a can-fail-routinely loop to make a new disposable_handoff_queue
- a can-fail-routinely loop to make a new single request-handler
- an extremely strict way to regulate that the server is not taxed with any work at all when excess requests come in: passively ignore them and do nothing at all: no prints, no logs, no responses, DO NOTHING and move on. (the optimal design for this may not be in place, if not, request better solutions for this, that is important. is checking the length of the array 'easy' enough as server work, or should there be a better method?, possibly even a time-lag on even checking for the size of the queue if it has been found to be full, and dropping, ignoring, any requests that come in in the mean time.)
- the server is designed to handle one maybe two requests at a time and be indestructible, ignoring failures and ignoring request floods and slowly marching on.

## principles:
- 'fail and try again'
- minimal, as much 'vanilla Rust' with no other packages as possible.

# minimal load management system:
1. queue:
- all incoming requests are first put into a queue
- the queue has a max length, perhaps 500 or 1000, after which requests are just dropped. This should be coded in a such a way as to protect the server from getting overloaded with requests, however that is done. perhaps: if len(queue)>500 {drop}; or a struct that only holds that many and attempts to add more are ignored, whatever is least-work for the server.

2. the queue is handled one item at a time with some pace-wait time to be set as a constant.

3. this is not a multi-worker-thread server, this is a FIFO queue server that may be slow but should be impossible to over-load. e.g. in k8s this could be scaled in various ways.

4. crash resistance: the disposable_handoff_queue should be free to fail and be disposed of and a new fresh (however empty) queue being created without bother. e.g. two loops: a queue loops that makes a clean queue if the old one fails for any reason. and a FIFO queue handler thread that takes the next item in the queue (and when if fails, and new thread handler is made)

5. if the server is overwhelmed by requests it is imperative that the server do as little as possible, ideally absolutely nothing, to ignore the flood of requests. no prints, no logs, no response, nothing; not even an action to check the size of the queue if possible (e.g. if the queue is a fixed size and adding to it fails, whatever is the least effort). 

## use-case example:
- a process that takes most of the cpu-gpu of the server cannot be multiplied in threads, but must be run more serially. speed is not the goal, parallel is not the goal, not-crashing is the goal, resource-balancing is the goal, not crashing is the goal


please start with initial MVP code that runs gradually adding features,
not all features need to exist in the first version

even though we will try (again) with tokio and hyper,
remember this is not a generic async multi-thread react server
this is a minimal server, review the specs and requirements.

see:
- https://docs.rs/llama_cpp/latest/llama_cpp/
- https://docs.rs/bounded-vec-deque/latest/bounded_vec_deque/ 


# # Variations
1. A first mvp version of this should be tried with standard library non-async as the scope, e.g. for very resource intensive operations (data science endpoints).
2. But a more fully async version for smaller operations, basic micro endpoints but still load/crash resistant.


# Queue Handoff
The length-count is not the only thing requiring access to the queue. The main listening loop also needs to add items to the queue when new requests arrive.


Having the handler loop take ownership of the whole queue and then having the main listener create a new queue is sometimes referred to as a "queue handoff" or "queue exchange."

Benefits of Queue Handoff:

Simplified Ownership: The ownership of the queue is clearly defined at each point in time. The handler loop owns the queue while it's processing it, and the main listener owns the queue while it's receiving requests.

Reduced Contention: There's no need for locks or mutexes to protect the queue because only one thread accesses it at any given time.

Potential for Parallelism: If you want to introduce a limited degree of concurrency (e.g., with a small thread pool), you could have multiple handler loops, each taking ownership of a separate queue.


# Overall Design:
headline: The request streamloop and the request_handler will NOT be concurrently accessing the same disposable_handoff_queue.

loop 1: 
application runs in a loop that restarts when it fails

stream-loop 2:
Requests ~are added to a max-sized queue 
(tracked with a counter).
when queue is full (if counter > MAX): 
server ignores additional requests (zero action taken).

if request_handler state is idle: pass along request and Queue Handoff


process loop 3: in request_handler
process request and queue in thread with 3 states:
1. busy
2. idle
3. failed

Note:  handler will NOT be concurrently accessing the same queue
Pay attention to the specific design of this system.
This is not a thread-sharing updated-queue where one queue is both updated and processed, this is Queue Handoff.

e.g.
If the handler is never busy when the next request comes in (if the traffic is slow), then the server simply passes each new request (and an empty queue) into the handler.

If the handler is busy (and the queue is not full) the stream-loop keep adding (up to MAX quantity) new requests to a disposable Handoff_Queue

(however the presence or absence of a queue is handled,
e.g. maybe here is an array of queues that is either empty of full (of 1), you can check the size of that to see if there is a handoff_queue

let mut vec handoffqueue_container;

after the disposable_handoff_queue

The request streamloop and the request_handler will NOT be concurrently accessing the same disposable_handoff_queue.
Again: The request streamloop and the request_handler will NOT be concurrently accessing the same disposable_handoff_queue.


approach:
HandlerState Enum: We define the HandlerState enum to represent the possible states of the request handler.
HANDLER_STATE Atomic Variable: We declare a static atomic variable HANDLER_STATE to store the current state of the handler. It's initialized to HandlerState::Idle.
process_request Function (Updated): We update the process_request function to set the HANDLER_STATE to either Idle or Failed depending on whether the request processing was successful or not.
Main Loop (Updated): In the main loop, before passing a request to the handler, we check the HANDLER_STATE using load(Ordering::Relaxed). If it's Idle, we proceed with passing the request. Otherwise, we handle the busy or failed state (e.g., by adding the request to the queue).

AtomicUsize
Represents an unsigned integer (usize) that can be safely accessed and modified by multiple threads concurrently.
It provides atomic operations (e.g., load, store, compare-and-swap) that guarantee that these operations are completed as a single, indivisible unit, preventing race conditions.
It is not designed to store strings directly.
Why 
In the previous code, we used AtomicUsize to represent the HANDLER_STATE because:
Enum Representation: We defined the HandlerState enum with different states (Busy, Idle, Failed).
Integer Mapping: We implicitly mapped these enum variants to integer values (e.g., Idle might be 0, Busy might be 1, and Failed might be 2). This mapping is done automatically by the compiler when you cast an enum to an integer (e.g., HandlerState::Idle as usize).
Atomic Storage: We needed an atomic variable to store this integer representation of the state so that multiple threads could safely access and update it. AtomicUsize is suitable because it can store unsigned integers.
In essence, we are using 

*/
use std::io::prelude::*;
use std::net::{TcpListener};
use std::thread;
use std::collections::VecDeque;
use std::time::Duration;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::process::Command;

const MAX_QUEUE_SIZE: usize = 500;
const PROCESSING_DELAY_MS: u64 = 100; // Adjust as needed
const REQUEST_HANDLER_PAUSE: u64 = 10; // millis

// For states of request_hanlder
enum HandlerState {
    // Busy,
    Idle,
    Failed,
}

/// Represents the state of the request handler with Integer Mapping
/// 
/// This atomic variable is used to track whether the handler is currently busy processing a request,
/// idle and available to handle a new request, or in a failed state.
/// 
/// The state is represented as a `usize` to be compatible with the `AtomicUsize` type.
/// The possible states are defined by the `HandlerState` enum:
/// - `Busy`: The handler is currently processing a request.
/// - `Idle`: The handler is available to process a new request.
/// - `Failed`: The handler has encountered an error and is not operational.
/// 
/// The initial state is set to `Idle`.
/// AtomicUsize
///
/// Represents an unsigned integer (usize) that can be safely accessed and modified by multiple threads concurrently.
///
/// It provides atomic operations (e.g., load, store, compare-and-swap) 
/// that guarantee that these operations are completed as a single, indivisible unit, preventing race conditions.
///
/// It is not designed to store strings directly.
///
/// Why usize for HANDLER_STATE?
/// In the previous code, we used AtomicUsize to represent the HANDLER_STATE because:
/// 1. Enum Representation: We defined the HandlerState enum with different states (Busy, Idle, Failed).
/// 2. Integer Mapping: We implicitly mapped these enum variants to integer values 
/// (e.g., Idle might be 0, Busy might be  1, and Failed might be 2). This mapping is done automatically by the compiler 
/// when you cast an enum to an integer (e.g., HandlerState::Idle as usize).
/// 3. Atomic Storage: We needed an atomic variable to store this integer representation of the state 
/// so that multiple threads could safely access and update it. 
/// AtomicUsize is suitable because it can store unsigned integers.
static HANDLER_STATE: AtomicUsize = AtomicUsize::new(HandlerState::Idle as usize); 

// Placeholder for external program execution
fn process_request(request_data: String) {
    // TODO: Implement calling external program (e.g., Python, llama.cpp)
    println!("Processing request: {}", request_data); 
    println!("Request data: {:?}", request_data); // Print the string's debug representation
    
    // Remove null bytes from request_data
    let request_data_without_nulls = request_data.replace('\0', ""); 
    
    println!("request_data_without_nulls data: {:?}", &request_data_without_nulls); // Print the string's debug representation 
    
    let output = Command::new("/home/oops/code/llama_cpp/llama.cpp/llama-cli")
        .stderr(std::process::Stdio::null())
        .arg("-m")
        .arg("/home/oops/jan/models/gemma-2-2b-it/gemma-2-2b-it-Q4_K_M.gguf")
        .arg("-p")
        .arg(request_data_without_nulls)
        .output()
        .expect("Failed to execute llama-cli");

    if output.status.success() {
        println!("Output: {}", String::from_utf8_lossy(&output.stdout));
    } else {
        eprintln!("Error during llama-cli execution: {:?}", output.stderr);
    }
    
    
    // Simulate processing delay
    thread::sleep(Duration::from_millis(PROCESSING_DELAY_MS));
}

// todo: modify to accept -> handler_of_request_and_queue(request_string, disposable_handoff_queue);
// fn handler_of_request_and_queue(
//     request_body: String, 
//     mut disposable_handoff_queue: VecDeque<String>
// ) {
//     // 1. Add request to queue
//     disposable_handoff_queue.push_back(request_body);

//     // 2. Process queue
//     loop {
//         if let Some(request_data) = disposable_handoff_queue.pop_front() {
//             process_request(request_data);
//         } else {
//             // Queue is empty, wait a bit before checking again
//             thread::sleep(Duration::from_millis(REQUEST_HANDLER_PAUSE)); // Adjust as needed
//         }
//     } // TODO: add error handling so if they fails for any reason
//     // it sends a signal for the other loop to restart
// }
fn handler_of_request_and_queue(
    request_body: String, 
    disposable_handoff_queue: VecDeque<String> 
) {
    std::panic::catch_unwind(|| {
        let mut queue_clone = disposable_handoff_queue.clone(); // Clone the queue

        // 1. Add request to the cloned queue
        queue_clone.push_back(request_body);

        // 2. Process the cloned queue
        loop {
            if let Some(request_data) = queue_clone.pop_front() {
                process_request(request_data);
            } else {
            // Queue is empty, wait a bit before checking again
            thread::sleep(Duration::from_millis(REQUEST_HANDLER_PAUSE)); // Adjust as needed
            }
        }
    }).unwrap_or_else(|_| {
        // Set HANDLER_STATE to Failed
        HANDLER_STATE.store(HandlerState::Failed as usize, Ordering::Relaxed);
        println!("Handler thread panicked!"); 
    }); 
}

// TODO Add explanation here, in detail
/*
text
*/
static QUEUE_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn main() {
    
    // Main loop for crash resistance, 'Let it fail, and try again.'
    // Main Loop:
    // Purpose: The main loop is responsible for the overall lifecycle of the server. 
    // It initializes components, starts the stream-loop, and handles potential 
    // restarts if the stream-loop encounters errors.
    //
    // Execution: The main loop typically runs only once when the server starts and continues 
    // running indefinitely until the server is intentionally shut down.
    //
    // Responsibility for Queues: The main loop is responsible for creating the initial disposable handoff 
    // queue when the server starts. It might also handle the creation of a new queue if the handler thread 
    // encounters an error, but this logic might also be delegated to the stream-loop.
    loop {
        
        // in stream-loop?
        // // Bootstrap: Make a first empty queue
        // let mut disposable_handoff_queue: Vec<String> = Vec::with_capacity(MAX_QUEUE_SIZE); 

        // // Bootstrap: set restart_signal flag
        // let mut signal_to_restart = false;

        // // Bootstrap: set restart_signal flag
        // let mut counter = 0;        


        let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
        // TODO error handling, when fails: signal_to_restart = True, restart loop

        
        // Purpose: The stream-loop is responsible for listening for incoming requests, 
        // handling the request queue, and passing requests to the handler.
        // Execution: The stream-loop runs continuously within the main loop, 
        // accepting and processing incoming requests.
        // Responsibility for Queues: The stream-loop is primarily responsible 
        // for creating new disposable handoff queues immediately after handing 
        // off the previous queue to the handler. It also manages adding requests 
        // to the current queue and checking if the queue is full.
        // Additionally, the stream-loop can signal a restart of the main loop in case of bad failures.
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    
                    // make sure no old queue exists
                    // let disposable_handoff_queue: Option<VecDeque<String>> = None;


                    // Initial creation (in the main loop)
                    let mut disposable_handoff_queue: Option<VecDeque<String>> = Some(VecDeque::with_capacity(MAX_QUEUE_SIZE));
                                        
                
                    // // check restart single to exit stream, restart main(loop)
                    // if restart_signal{
                    //     exit stream; // TODO this is pseuodcode
                    // }
                        
                    
                    let mut buffer = [0; 1024];
                    stream.read(&mut buffer).unwrap();
                    // TODO when fails, error restart_signal = True / or drop and continue stream

                    let request_string = String::from_utf8_lossy(&buffer[..]);
                    // TODO when fails, error restart_signal = True / or drop and continue stream

                    // Very basic parsing of the request (assuming POST)
                    if request_string.starts_with("POST") {
                        
                        let body_start = request_string.find("\r\n\r\n").unwrap_or(0) + 4;
                        // TODO when fails, error restart_signal = True / or drop and continue stream
                        
                        // let request_body = request_string[body_start..].to_string();
                        // let request_string = String::from_utf8_lossy(&buffer[..]);
                        let request_body = request_string[body_start..].to_string();
                        // TODO when fails, error restart_signal = True / or drop and continue stream

                        // 
                        /*                        
                        Design question:
                        What kind of structure/thread can the handler be or be in
                        that has states:
                        1. busy
                        2. idle
                        3. failed
                        
                        scoped threads?
                        thread parking?
                        
                        */
                        
                        // if Idle
                        // Checks if the request handler is currently in the `Idle` state.
                        //
                        // This function loads the current state of the handler from the `HANDLER_STATE` atomic variable
                        // and compares it with the integer representation of the `Idle` state. 
                        // It returns `true` if the handler is `Idle` and `false` otherwise (if it's `Busy` or `Failed`).
                        //
                        // Note: This check only explicitly distinguishes between `Idle` and non-`Idle` states.
                        // It does not differentiate between `Busy` and `Failed` within this specific check. 
                        // However, the `else` block that follows this check handles both `Busy` and `Failed` states 
                        // by attempting to add the incoming request to the queue.
                        if HANDLER_STATE.load(Ordering::Relaxed) == HandlerState::Idle as usize {
                            /*
                            handler can be: 1 busy, 2. not_busy 3. failed
                            
                            A. look for quit-signal_to_restart (optional, if needed later)
                            B. if handler is not busy, give request+queue to handler & reset counter to 0
                            C. if handler is busy, check counter
                            E. if counter > MAX: drop request
                            F. if counter < MAX: check if there is an existing queue
                            G. if there is an existing queue: add request to quque
                            H: if there is no queue: make a queue and add request to queue
                            loop back 
                            */
                            // Request processing oc
                            // when this fails (everything will fail at some point)
                            // this should output a signal to set a 'restart' flag 
                            thread::spawn(move || {
                                handler_of_request_and_queue(
                                    request_body.to_string(), 
                                    disposable_handoff_queue.expect("REASON")
                                );
                            }); // TODO error handling, when fails: signal_to_restart = True

                            // 1. handler_of_request_and_queue(request, quque)

                            
                            // Double Tap: make sure queue is removed
                            // When the handler finishes or fails (in the handler thread or stream-loop):
                            // let disposable_handoff_queue: Option<VecDeque<String>> = None; // Indicate that a new queue needs to be created 
                            
                            // 2. counter = zero
                            // Reset the queue counter
                            QUEUE_COUNTER.store(0, Ordering::Relaxed);

                            // 3. make a new empty disposable_handoff_queue
                            // let mut disposable_handoff_queue: Option<VecDeque<String>> = Some(VecDeque::with_capacity(MAX_QUEUE_SIZE));
                            
                            // if faile:
                            // exit/continue/break stream-loop/quit/reboot
                            
                            
                        } else {  // if NOT Idle: elif busy, elif failed
                            
                            // Handle busy/failed state (e.g., add to queue)
                            // Check if a queue exists and add requests to it
                            if let Some(queue) = &mut disposable_handoff_queue {
                                // ... (add requests to the queue) ...
                                // if queue is not full: add request to queue
                                if QUEUE_COUNTER.load(Ordering::Relaxed) < MAX_QUEUE_SIZE {

                                    // add request to queue!
                                    queue.push_back(request_body.to_string()); // Call push_back on the VecDeque inside the Option
                     
                                    QUEUE_COUNTER.fetch_add(1, Ordering::Relaxed);
                                    
                                    
                                    
                                
                            } else {
                                // No queue available, create a new one 
                                // let mut disposable_handoff_queue: Option<VecDeque<String>> = Some(VecDeque::with_capacity(MAX_QUEUE_SIZE));
                                // ... (potentially add the current request to the new queue) ...
                            }
                            

                            } else {
                                // Ignore the request (queue is full)
                            }
                            
                            // increment counter
                            QUEUE_COUNTER.fetch_add(1, Ordering::Relaxed);
                        }
                        
                        
                        if HANDLER_STATE.load(Ordering::Relaxed) == HandlerState::Failed as usize {
                            println!("Handler thread failed. Restarting..."); // Log the failure
                            break; // Exit the stream-loop to signal a restart 
                        }

                    }

                    let response = "HTTP/1.1 200 OK\r\n\r\n";
                    stream.write(response.as_bytes()).unwrap();
                    stream.flush().unwrap();
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
                // look for restart-flag from failure and signal larger restart exit
            }
        }

        // If the code reaches here, it means the listener loop has exited (e.g., due to an error)
        // The outer loop will restart, creating a fresh disposable_handoff_queue and listener
    } 
}