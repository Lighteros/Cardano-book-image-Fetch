mod models;
mod services;
mod utils;

use services::{
    blockfrost::BlockFrostService,
    bookio::{BookioService, URL},
};
use std::{path::PathBuf, process};
use structopt::StructOpt;

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
        let result = service.fetch_assets_metadata(&opt.policy_id, &opt.output_dir).await;
        if let Err(e) = result {
            println!("cannot fetch metadata: {}", e);
            process::exit(1);
        }

        if let Ok(assets) = result {
            for asset in assets {
                println!("asset: {:?}", asset);
            }
        }
    }
}
