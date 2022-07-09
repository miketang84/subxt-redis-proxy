use futures::StreamExt;
use sp_keyring::AccountKeyring;
use std::time::Duration;
use subxt::{
    ClientBuilder,
    DefaultConfig,
    PairSigner,
    SubstrateExtrinsicParams,
};

//use futures_util::StreamExt as _;
//use futures::StreamExt as _;
use redis::AsyncCommands;

#[subxt::subxt(runtime_metadata_path = "../metadata.scale")]
pub mod substrate {}

/// Subscribe to all events, and then manually look through them and
/// pluck out the events that we care about.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let redis_client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut redis_conn = redis_client.get_async_connection().await?;
    let mut pubsub_conn = redis_client.get_async_connection().await?.into_pubsub();

/*
    // While this subscription is active, balance transfers are made somewhere:
    tokio::task::spawn(async {
        let signer = PairSigner::new(AccountKeyring::Alice.pair());
        let api =
            ClientBuilder::new()
                .build()
                .await
                .unwrap()
                .to_runtime_api::<substrate::RuntimeApi<
                    DefaultConfig,
                    SubstrateExtrinsicParams<DefaultConfig>,
                >>();

        let mut transfer_amount = 1_000_000_000;

        // Make small balance transfers from Alice to Bob in a loop:
        loop {
            api.tx()
                .balances()
                .transfer(AccountKeyring::Bob.to_account_id().into(), transfer_amount)
                .expect("compatible transfer call on runtime node")
                .sign_and_submit_default(&signer)
                .await
                .unwrap();

            tokio::time::sleep(Duration::from_secs(10)).await;
            transfer_amount += 100_000_000;
        }
    });
*/

    let task_subxt = tokio::task::spawn(async move {

        let api = ClientBuilder::new()
            .build()
            .await?
            .to_runtime_api::<substrate::RuntimeApi<DefaultConfig, SubstrateExtrinsicParams<DefaultConfig>>>();

        // Subscribe to any events that occur:
        let mut event_sub = api.events().subscribe().await?;

        // Our subscription will see the events emitted as a result of this:
        while let Some(events) = event_sub.next().await {
            let events = events?;
            let block_hash = events.block_hash();

            // We can iterate, statically decoding all events if we want:
            println!("All events in block {block_hash:?}:");
            println!("  Static event details:");
            for event in events.iter() {
                let event = event?;
                println!("    {event:?}");
            }

            // Or we can dynamically decode events:
            println!("  Dynamic event details: {block_hash:?}:");
            for event in events.iter_raw() {
                let event = event?;
                let is_balance_transfer = event
                    .as_event::<substrate::balances::events::Transfer>()?
                    .is_some();
                let pallet = event.pallet;
                let variant = event.variant;
                println!(
                    "    {pallet}::{variant} (is balance transfer? {is_balance_transfer})"
                    );
            }

            // Or we can dynamically find the first transfer event, ignoring any others:
            let transfer_event =
                events.find_first::<substrate::balances::events::Transfer>()?;

            if let Some(ev) = transfer_event {
                println!("  - Balance transfer success: value: {:?}", ev.amount);
            } else {
                println!("  - No balance transfer event found in this block");
            }

            //TODO: Do stuff on redis conn to send info
            //redis_conn.publish("wavephone", "banana").await?;

        }

        Ok::<(), subxt::BasicError>(())
    });
    

    // ==============================
    // redis listener part


    let task_redis = tokio::task::spawn(async move {
        pubsub_conn.subscribe("wavephone").await?;
        let mut pubsub_stream = pubsub_conn.on_message();

        //redis_conn.publish("wavephone", "banana").await?;
        //println!("publish to channel wavephone: {}", "banana");

        // Get a instance of subxt to send transactions to substrate
        let signer: PairSigner<DefaultConfig, _> = PairSigner::new(AccountKeyring::Alice.pair());
        let subxt_api =
            ClientBuilder::new()
                .build()
                .await
                .unwrap()
                .to_runtime_api::<substrate::RuntimeApi<
                    DefaultConfig,
                    SubstrateExtrinsicParams<DefaultConfig>,
                >>();
/*
        api.tx()
            .balances()
            .transfer(AccountKeyring::Bob.to_account_id().into(), transfer_amount)
            .expect("compatible transfer call on runtime node")
            .sign_and_submit_default(&signer)
            .await
            .unwrap();
*/

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

    let (_, _) = tokio::join!(task_subxt, task_redis);
    //let _ = task_subxt.await;
    //let _ = task_redis.await;
    
    Ok(())
}
