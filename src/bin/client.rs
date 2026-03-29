use bytes::Bytes;
use mini_redis::client;
use tokio::sync::{mpsc, oneshot};

type Responder<T> = oneshot::Sender<mini_redis::Result<T>>;

#[derive(Debug)]
enum Command {
    Get {
        key: String,
        response: Responder<Option<Bytes>>,
    },
    Set {
        key: String,
        val: Bytes,
        response: Responder<()>,
    },
}

#[tokio::main]
async fn main() {
    // Create a new channel with a capacity of at most 32.
    let (sender, mut receiver) = mpsc::channel(32);

    // The `move` keyword is used to **move** ownership of `rx` into the task.
    let manager = tokio::spawn(async move {
        // Establish a connection to the server
        let mut client = client::connect("127.0.0.1:6379").await.unwrap();

        // Start receiving messages
        while let Some(cmd) = receiver.recv().await {
            use Command::*;

            match cmd {
                Get { key, response } => {
                    let res = client.get(&key).await;

                    // Ignore errors
                    let _ = response.send(res);
                }
                Set { key, val, response } => {
                    let res = client.set(&key, val).await;

                    // Ignore errors
                    let _ = response.send(res);
                }
            }
        }
    });

    let sender_2 = sender.clone();

    // Spawn two tasks, one gets a key, the other sets a key
    let thread_1 = tokio::spawn(async move {
        let (response_sender, response_receiver) = oneshot::channel();
        let cmd = Command::Get {
            key: "foo".to_string(),
            response: response_sender,
        };

        // Send the GET request
        sender.send(cmd).await.unwrap();

        // Await the response
        let res = response_receiver.await;
        println!("GOT = {res:?}");
    });

    let thread_2 = tokio::spawn(async move {
        let (response_sender, response_receiver) = oneshot::channel();
        let cmd = Command::Set {
            key: "foo".to_string(),
            val: "bar".into(),
            response: response_sender,
        };

        // Send the SET request
        sender_2.send(cmd).await.unwrap();

        // Await the response
        let res = response_receiver.await;
        println!("GOT = {res:?}");
    });

    thread_1.await.unwrap();
    thread_2.await.unwrap();

    manager.await.unwrap();
}
