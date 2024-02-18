// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the aoraki-labs library.

// The aoraki-labs library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The aoraki-labs library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the aoraki-labs library. If not, see <https://www.gnu.org/licenses/>.

use std::{thread};
use tokio::time::{self, Duration};

use chain::dummy_task;
use log::*;
use clap::{load_yaml, App};
use server::loop_task_data;
use web3::signing::keccak256;

mod server;
mod chain;
use crate::{server::start_rpc_server, chain::{PRIV_KEY, RELAYER_URL, CONTRACT}};

#[macro_use]
mod app_marco;

use std::time::{Instant};

pub async fn main_process_task_data() {
    std::panic::set_hook(Box::new(|panic_info| {
        error!("Panic occurred: {:?}", panic_info);
    }));

    loop{
        time::sleep(Duration::from_secs(1)).await;
        match loop_task_data().await{
            Ok(()) => (),
            Err(_) => {
                error!("***process one proof task error***");
            }
        }
    }
}

pub async fn dummy_task_loop(interval:u64) { //dummy onchain task in interval seconds period
    loop{
        time::sleep(Duration::from_secs(interval)).await;
        let mut retry:usize = 0;
        loop{
            retry += 1;
            match dummy_task().await{
                Ok(_) => break,
                Err(_) => {
                    if retry <= 1{
                        continue;
                    }else {
                        break;
                    }
                },
            };
        } 
    }
}

#[tokio::main]
async fn main() {

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
    .filter(Some("chain"), log::LevelFilter::Error)
    .init();

    let cli_param_yml = load_yaml!("app.yml");
    let cli_param = App::from_yaml(cli_param_yml).get_matches();
    let key: String = cli_param.value_of("key").unwrap_or("").into();
    let listen: String = cli_param.value_of("listen").unwrap_or("").into();
    let relayer: String = cli_param.value_of("relayer").unwrap_or("").into();
    let interval: String = cli_param.value_of("interval").unwrap_or("").into();
    let contract_addr: String = cli_param.value_of("contract").unwrap_or("").into();
    
    {
        let mut priv_key = PRIV_KEY.lock().await;
        *priv_key=key.clone();
    
        let mut relayer_url = RELAYER_URL.lock().await;
        *relayer_url=relayer.clone();

        let mut contract = CONTRACT.lock().await;
        *contract=contract_addr.clone();

    }

    let my_server = start_rpc_server(listen);
    let srv_handle = tokio::spawn(async move {
        my_server.await.wait();
    });
    
    let process_task_handle = tokio::spawn(async move {
        main_process_task_data().await
    });

    let dummy_task_handle = tokio::spawn(async move {
        dummy_task_loop(interval.parse::<u64>().unwrap()).await
    });


 
    tokio::select! {
      _ = async { srv_handle.await } => {
        info!("server terminal")
        },
      _ = async { process_task_handle.await } => {
        info!("process task handle terminal")
        },
      _ = async { dummy_task_handle.await } => {
        info!("dummy task handle terminal")
       },
    }
}
