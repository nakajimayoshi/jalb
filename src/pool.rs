use std::{
    thread, 
    vec,
    sync::{mpsc, Arc, Mutex},
};

use log::{error};

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers= vec::Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)))
        }

        Self {
          workers,
          sender: Some(sender)
        }
    }

    pub fn execute<F>(&self, f: F)
    where 
        F: FnOnce() + Send +'static,
        {

            let job = Box::new(f);

            if let Some(sender) = self.sender.as_ref() {
                sender.send(job).expect("failed to send job")
            }

        }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for worker in self.workers.drain(..) {
            if let Err(_) = worker.thread.join() {
                error!("failed to join worker id:{} during drop", worker.id)
            }
        }
    }
}

pub struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = std::thread::spawn(move || loop {

            let message = receiver.lock().unwrap().recv();
            
            match message {
                Ok(job) => {
                    job();
                },
                Err(_) => {
                    break
                }
            }
        });

        Worker { id, thread }
    }
}