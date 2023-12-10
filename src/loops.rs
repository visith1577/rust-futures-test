use std::{error::Error, sync::Arc, collections::{HashMap,  hash_map::Entry}};

use async_std::{
    prelude::*,
    net::{TcpListener, ToSocketAddrs, TcpStream}, io::BufReader, task
};
use futures::{channel::mpsc, SinkExt};

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
pub type Receiver<T> = mpsc::UnboundedReceiver<T>;
pub type Sender<T> = mpsc::UnboundedSender<T>;


#[derive(Debug)]
pub enum Event {
    NewPeer {
        name: String,
        stream: Arc<TcpStream>,
    },
    Message {
        from: String,
        to: Vec<String>,
        msg: String,
    }
}


fn spawn_and_log_error<F>(fut: F) -> task::JoinHandle<()>
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    task::spawn(async move {
        if let Err(e) = fut.await {
            eprintln!("{}", e)
        }
    })
}


pub async fn accept_loop(addr: impl ToSocketAddrs) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    let mut incoming = listener.incoming();

    let (broker_sender, broker_receiver) = mpsc::unbounded();
    let broker_handle = task::spawn(broker_loop(broker_receiver));

    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        println!("Accepting from {:?}", stream.peer_addr());
        spawn_and_log_error(connection_loop(broker_sender.clone(), stream));
    }
    drop(broker_sender);
    broker_handle.await?;
    Ok(())
}

pub async fn connection_loop(mut broker: Sender<Event>, stream: TcpStream) -> Result<()> {
    let stream = Arc::new(stream);
    let reader = BufReader::new(&*stream);
    let mut lines = reader.lines();

    let name = match lines.next().await {
        None => Err("peer disconnected immediately")?,
        Some(line) => line?, 
    };

    broker.send(Event::NewPeer { name: name.clone(), stream: Arc::clone(&stream) }).await // 3
        .unwrap();

    while let Some(line) = lines.next().await {
        let line = line?;
        let (dest, msg) = match line.find(":") {
            None => continue,
            Some(idx) => (&line[..idx], line[idx + 1 ..].trim()),            
        };
        let dest: Vec<String> = dest.split(',').map(|name| name.trim().to_string()).collect();
        let msg = msg.to_string();

        broker.send(Event::Message { 
            from: name.clone(), 
            to: dest, 
            msg
        }).await.unwrap();
    }

    Ok(())
}

async fn connection_writer_loop(
    mut message: Receiver<String>,
    stream: Arc<TcpStream>,
) -> Result<()> {
    let mut stream = &*stream;
    while let Some(msg) = message.next().await {
        stream.write_all(msg.as_bytes()).await?;
    }
    Ok(())
}

pub async fn broker_loop(
    mut events: Receiver<Event>,
) -> Result<()> {
    let mut writers = Vec::new();
    let mut peers: HashMap<String, Sender<String>> = HashMap::new();

    while let Some(event) = events.next().await {
        match event {
            Event::Message { from, to, msg } => {
                for addr in to {
                    if let Some(peer) = peers.get_mut(&addr) {
                        let msg = format!("From {}: {}\n", from, msg);
                        peer.send(msg).await?
                    }
                }
            }
            Event::NewPeer { name, stream } => {
                match peers.entry(name) {
                    Entry::Occupied(..) => (),
                    Entry::Vacant(entry) => {
                        let (client_sender, client_receiver) = mpsc::unbounded();
                        entry.insert(client_sender);
                        let handle = spawn_and_log_error(connection_writer_loop(client_receiver, stream));
                        writers.push(handle);
                    }
                }
            }
        }
    }
    drop(peers);
    for writer in writers {
        writer.await;
    }
    Ok(())
}
