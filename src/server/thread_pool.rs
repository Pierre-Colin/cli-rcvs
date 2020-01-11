use std::{
    sync::{atomic, mpsc, Arc, Mutex},
    thread,
};

trait FnBox {
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<F>) {
        (*self)()
    }
}

type Job = Box<dyn FnBox + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}

struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    pub fn new(receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        Worker {
            thread: Some(thread::spawn(move || loop {
                let message = receiver.lock().unwrap().recv().unwrap();

                match message {
                    Message::NewJob(job) => job.call_box(),
                    Message::Terminate => break,
                }
            })),
        }
    }
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
    canary: Arc<atomic::AtomicBool>,
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("Sending shutdown signals");
        for _ in &mut self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }

        self.canary.store(true, atomic::Ordering::Relaxed);
    }
}

impl ThreadPool {
    pub fn new(size: usize, canary: Arc<atomic::AtomicBool>) -> Option<ThreadPool> {
        if size > 0 {
            let (sender, receiver) = mpsc::channel();
            let receiver = Arc::new(Mutex::new(receiver));
            let mut workers = Vec::with_capacity(size);
            for _ in 0..size {
                workers.push(Worker::new(Arc::clone(&receiver)));
            }
            Some(ThreadPool {
                workers: workers,
                sender: sender,
                canary: canary,
            })
        } else {
            None
        }
    }

    pub fn run(&self, f: impl FnOnce() + Send + 'static) {
        self.sender.send(Message::NewJob(Box::new(f))).unwrap();
    }
}

#[cfg(tests)]
mod tests {
    use super::*;

    #[test]
    fn new_pool() {
        assert!(ThreadPool::new(0).is_none(), "Size-0 pool is instanced");
        for size in 1..1000000 {
            if let Some(pool) = ThreadPool::new(size) {
                assert_eq!(
                    pool.threads.capacity(),
                    size,
                    "Pool doesn't have capacity {}",
                    size
                );
            } else {
                panic!("Size-{} pool isn't instanced", size);
            }
        }
    }
}
