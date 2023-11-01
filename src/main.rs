mod models;
mod services;
mod utils;

use futures::future::AbortHandle;
use services::{
    blockfrost::BlockFrostService,
    bookio::{BookioService, URL},
};
use std::{path::PathBuf, process};
use structopt::StructOpt;
use tokio::signal;

#[derive(StructOpt, Debug)]
#[structopt(name = "cardano_book_image_fetcher")]
struct Opt {
    #[structopt(short, long)]
    policy_id: String,

    #[structopt(short, long, parse(from_os_str))]
    output_dir: PathBuf,
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();

    println!("policy_id: {}, output_dir: {:?}", opt.policy_id, opt.output_dir);

    let service = BookioService::new().unwrap();
    let result = service.verify_policy_id(&opt.policy_id, URL).await;

    if let Err(e) = result {
        println!("policy_id {} is not valid: {}", opt.policy_id, e);
        process::exit(1);
    }

    if let Ok(false) = result {
        println!("policy_id {} is not found", opt.policy_id);
        process::exit(1);
    }

    if let Ok(true) = result {
        let service = BlockFrostService::new().unwrap();

        // Create an AbortHandle using futures::future::Abortable
        // This allows us to cancel a future from a different context
        let (abort_handle, abort_registration) = AbortHandle::new_pair();

        let fetch_handle = tokio::spawn(async move {
            let result = service.fetch_assets_metadata(&opt.policy_id, &opt.output_dir).await;
            if let Err(e) = result {
                println!("cannot fetch metadata: {}", e);
                process::exit(1);
            }

            if let Ok(assets) = result {
                assets
            } else {
                vec![]
            }
        });

        // The Abortable future will complete with an Error::Aborted when abort_handle.abort() is called
        let fetch_future = futures::future::Abortable::new(fetch_handle, abort_registration);

        tokio::select! {
            fetch_result = fetch_future => {
                match fetch_result {
                Ok(Ok(assets)) => {
                    for asset in assets {
                        println!("asset: {:?}", asset);
                    }
                },
                Ok(Err(_)) => println!("fetching was aborted!"),
                Err(_) => println!("fetching was not completed successfully"),
            }
            }
            _ = signal::ctrl_c() => {
                println!("Received CTRL+C! Cancelling tasks ...");
                abort_handle.abort();
            }
        }
    }
}
