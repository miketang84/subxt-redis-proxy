//use futures_util::StreamExt as _;
use futures::StreamExt as _;
use redis::AsyncCommands;

#[tokio::main]
async fn main() -> redis::RedisResult<()> {
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut redis_conn = client.get_async_connection().await?;
    let mut pubsub_conn = client.get_async_connection().await?.into_pubsub();


    let task_redis = tokio::task::spawn(async move {
        pubsub_conn.subscribe("wavephone").await?;
        let mut pubsub_stream = pubsub_conn.on_message();

        redis_conn.publish("wavephone", "banana").await?;
        println!("publish to channel wavephone: {}", "banana");

        loop {
            let msg = pubsub_stream.next().await;
            println!("received msg from channel wavephone: {:?}", msg);
            let msg_payload: String = msg.unwrap().get_payload()?;
            println!("received from channel wavephone: {:?}", msg_payload);
        }

        Ok::<(), redis::RedisError>(())
    });

    //let pubsub_msg: String = pubsub_stream.next().await.unwrap().get_payload()?;
    //assert_eq!(&pubsub_msg, "banana");
    //println!("received from channel wavephone: {}", pubsub_msg);

    let _ = task_redis.await;
    
    Ok(())
}
