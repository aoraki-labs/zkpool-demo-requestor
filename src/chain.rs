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


use std::{str::FromStr, sync::Arc, collections::VecDeque};
use log::*;
use core::str;
use rand::seq::SliceRandom;
use serde_derive::{Deserialize,Serialize};
use ethereum_private_key_to_address::PrivateKey;
use chrono::{Utc};

use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::Client;

use web3::{
    ethabi::{ethereum_types::U256,Function, ParamType, Param, StateMutability, Token},
    types::{Address,Bytes, TransactionParameters, TransactionRequest}, signing::keccak256,
};

use tokio::time::{self,Duration};

use web3::types::BlockNumber::{Latest,Pending};
use lazy_static::lazy_static;

const MAX_RETRIES: u32 = 5;
const GAS_PRICE_INCREMENT_PERCENTAGE: u32 = 20; // Increase gas price by 20% on each retry
const GAS_INCREMENT_PERCENTAGE: u32 = 20; // Increase gas by 20% on each retry

lazy_static! {
    pub static ref PRIV_KEY: tokio::sync::Mutex<String> = {      //priv_key
        tokio::sync::Mutex::new(String::from(""))
    };
    pub static ref RELAYER_URL: tokio::sync::Mutex<String> = {   //relayer rpc url
        tokio::sync::Mutex::new(String::from(""))
    };
    pub static ref TASK_MSG_QUEUE: Arc<tokio::sync::Mutex<VecDeque<String>>> = {
        Arc::new(tokio::sync::Mutex::new(VecDeque::new()))
    };
    pub static ref CONTRACT: tokio::sync::Mutex<String> = {      //contract
        tokio::sync::Mutex::new(String::from(""))
      };
}


///config chain urls
pub const SEPOLIA_CHAIN_URLS: [&str; 1] = [
    "https://eth-sepolia.g.alchemy.com/v2/kMO8lL7g44IJOGR-Om-kc7DAlmHaXFb7",
];


//Onchain paramter
pub const  GAS_UPPER : &str = "1000000";
// pub const  CONTRACT_ADDR :&str = "0xc20F6905A21c26B106c7A30E77e4711390cffBA8";


//Dummy task info
// Dev env
//const REWARD_TOKEN:&str="0xfDfd239c9dD30445d0e080Ecf055A5cc53456A72";
// Prod env
const REWARD_TOKEN:&str="0x0622118429C54577eF34229526661c41020048bF";
const REWARD:u64 = 100;
// Dev env
//const LIABILITY_TOKEN:&str="0xfDfd239c9dD30445d0e080Ecf055A5cc53456A72";
// Prod env
const LIABILITY_TOKEN:&str="0x0622118429C54577eF34229526661c41020048bF";
const LIABILITY:u64 = 100;
const LIABILITY_WINDOW:u64=36000;

#[derive(Debug, Serialize, Deserialize)]
struct RpcRequest {
    jsonrpc: String,
    method: String,
    params: Vec<String>,
    id: u64,
}

#[derive(Debug, Serialize, Deserialize,Default,Clone)]
struct TaskResponse {
    pub prover: String,
    pub instance: String,
    pub reward_token: String,
    pub reward: u64,
    pub liability_window: u64,
    pub liability_token: String,
    pub liability: u64,
    pub expiry: u64,
    pub signature: String,
}


#[derive(Debug, Serialize, Deserialize)]
struct RpcResponse {
    jsonrpc: String,
    result: String,
    id: u64,
}

/// get the account nonce value
pub async fn get_nonce(addr:Address) -> U256{
    loop {
        for url in SEPOLIA_CHAIN_URLS.iter() {
            let transport = match web3::transports::Http::new(&url){
                Ok(r)=>{r},
                Err(_e) => {
                    continue;
                },
            };
            let web3 = web3::Web3::new(transport);
            info!("addr is {:?}",addr);
            let nonce= match web3.eth().transaction_count(addr,Some(Pending)).await{
                Ok(r)=>{r},
                Err(_e) => {
                    continue;
                },
            };
            info!("nonce value is {:?}",nonce.clone());
            return nonce
    }
 }
}

/// 1.1 multiple of the network gas
pub async fn gas_price() -> U256{
    loop {
        for url in SEPOLIA_CHAIN_URLS.iter() {
            let transport = match web3::transports::Http::new(&url){
                Ok(r)=>{r},
                Err(_e) => {
                    continue;
                },
            };
            let web3 = web3::Web3::new(transport);
            let gas_price= match web3.eth().gas_price().await{
                Ok(r) => r,
                Err(_) => continue,
            };

            let upper_gas = (gas_price.as_u64())*(100)/(50);
            info!("gas price value is {:?}",upper_gas.clone());
            return U256::from_dec_str(&upper_gas.to_string()).unwrap()      
    }
 }
}

/// submit proof data to sepolia chain
pub async fn submit_task(  
    instance:Bytes,
    prover:Address,
    reward_token:Address,
    reward_amount:U256,
    liability_window:u64,
    liability_token:Address,
    liability_amount:U256,
    expiry:u64,
    signature:Bytes
) -> Result<String, String> { 

    let url_str = SEPOLIA_CHAIN_URLS.choose(&mut rand::thread_rng()).unwrap();
    let transport = web3::transports::Http::new(url_str).unwrap();
    let web3 = web3::Web3::new(transport);

    let ctr = CONTRACT.lock().await;
    let ctr_addr = (*ctr).clone();
    let contract_address = Address::from_str(ctr_addr.as_str()).unwrap();

    let func = Function {
        name: "submitTask".to_owned(),
        inputs: vec![
            Param { name: "instance".to_owned(), kind: ParamType::Bytes, internal_type: None },
            Param { name: "prover".to_owned(), kind: ParamType::Address, internal_type: None },
            Param { name: "rewardToken".to_owned(), kind: ParamType::Address, internal_type: None }, 
            Param { name: "rewardAmount".to_owned(), kind: ParamType::Uint(256), internal_type: None }, 
            Param { name: "liabilityWindow".to_owned(), kind: ParamType::Uint(64), internal_type: None }, 
            Param { name: "liabilityToken".to_owned(), kind: ParamType::Address, internal_type: None },
            Param { name: "liabilityAmount".to_owned(), kind: ParamType::Uint(256), internal_type: None },
            Param { name: "expiry".to_owned(), kind: ParamType::Uint(64), internal_type: None },
            Param { name: "signature".to_owned(), kind: ParamType::Bytes, internal_type: None },
             
        ],
        outputs: vec![],
        constant: Some(false),
        state_mutability: StateMutability::Payable,
    };

      //enocde send tx input parameters
    let mut data_vec_input:Vec<Token>=Vec::new();
    data_vec_input.push(Token::Bytes(instance.0));
    data_vec_input.push(Token::Address(prover));
    data_vec_input.push(Token::Address(reward_token));
    data_vec_input.push(Token::Uint(reward_amount));
    data_vec_input.push(Token::Uint(liability_window.into()));
    data_vec_input.push(Token::Address(liability_token));
    data_vec_input.push(Token::Uint(liability_amount));
    data_vec_input.push(Token::Uint(expiry.into()));
    data_vec_input.push(Token::Bytes(signature.0));

    let tx_data = func.encode_input(&data_vec_input).unwrap();

    let priv_key = PRIV_KEY.lock().await;
    let key = (*priv_key).clone();
    let prvk = web3::signing::SecretKey::from_str(key.as_str()).unwrap();
    let private_key = PrivateKey::from_str(key.as_str()).unwrap();
    let addr = private_key.address();

    let mut attempts = 0;
    let mut gas_price = gas_price().await;
    let mut gas_limit = U256::from_dec_str(GAS_UPPER).unwrap();

    //send tx to network
    loop {
        let nonce = get_nonce(Address::from_str(addr.as_str()).unwrap()).await;
        let tx_object = TransactionParameters {
            to: Some(contract_address),
            gas_price:Some(gas_price),
            gas:gas_limit,
            nonce:Some(nonce),
            data:Bytes(tx_data.clone()),
            ..Default::default()
        };

        let signed = match web3.accounts().sign_transaction(tx_object.clone(), &prvk).await {
            Ok(signed_tx) => signed_tx,
            Err(e) => {
                attempts += 1;
                if attempts >= MAX_RETRIES {
                    return Err(format!("Failed to sign transaction: {}", e));
                }
                time::sleep(Duration::from_secs(2u64.pow(attempts))).await;
                continue;
            }
        };

        match web3.eth().send_raw_transaction(signed.raw_transaction).await {
            Ok(tx_hash) => {
                info!("invoke a tx hash is : {:?}",tx_hash);
                return Ok(hex::encode(tx_hash.as_bytes()));
            },
            Err(e) => {
                if e.to_string().contains("replacement transaction underpriced") {
                    gas_price = gas_price * (100 + GAS_PRICE_INCREMENT_PERCENTAGE) / 100;
                } else {
                    // Handle other errors or add a general error increment
                    gas_limit = gas_limit * (100 + GAS_INCREMENT_PERCENTAGE) / 100;
                }
            }
        }

        attempts += 1;
        if attempts >= MAX_RETRIES {
            return Err("Transaction failed after maximum number of retries".to_string());
        }
        time::sleep(Duration::from_secs(2u64.pow(attempts))).await;
    }
}

pub async fn dummy_task() -> Result<String, String> {   //TBD
  info!("start to send dummy_task");

  let random_number = SystemTime::now()
  .duration_since(UNIX_EPOCH)
  .unwrap().as_millis(); 

  let input = format!("{}{}{}","\"5.7,2.5,5,2\"".to_string(),"#".to_string(),random_number.to_string());

  let task_key_result = hex::encode(keccak256(&input.to_string().into_bytes()));
  info!("this task task key is:{}",task_key_result);

  match assign_task(input.to_string()).await{ //replace one task parameter String
    Ok(_) => {
        Ok("dummy task send success".to_string())
    },
    Err(_) => {
        error!("dummy task generate failed");
        Err("dummy task send failed".to_string())
    },
}
}

pub async fn assign_task(instance:String)  -> Result<(), String> {  //TBD
    let client = Client::new();
    let request = RpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "ReceiveTask".to_string(),
        params: vec![instance.to_string(), LIABILITY_WINDOW.to_string(),LIABILITY_TOKEN.to_string(),LIABILITY.to_string(),REWARD_TOKEN.to_string(),REWARD.to_string()],
        id: 1,
    };

    let relayer_url = RELAYER_URL.lock().await;
    let relayer_endpoint = (*relayer_url).clone();

    // let response: RpcResponse = client
    //     .post(relayer_endpoint.clone()) //relayer rpc address
    //     .json(&request)
    //     .send()
    //     .await.unwrap()
    //     .json()
    //     .await.unwrap();

    let response_res= match client
        .post(relayer_endpoint) //relayer rpc address
        .json(&request)
        .send()
        .await{
            Ok(r) => r,
            Err(_) => return Err("invode relayer failed".to_string()),
        };
    let response:RpcResponse=match response_res.json().await{
        Ok(r) => r,
        Err(_) => return Err("invode relayer failed".to_string()),
    };

    let task_response:TaskResponse=match serde_json::from_str(response.result.as_str()){
        Ok(r) => r,
        Err(_) => {
            info!("can not parse the relayer response:{:?}",response.result);
            return  Err("assign_task parse response error".to_string())
        },
    };
    info!("receice relayer response result is : {:?}", task_response); 

    let instance=Bytes::from(task_response.instance);
    let addr=Address::from_str(task_response.prover.as_str()).unwrap();
    let reward_token=Address::from_str(task_response.reward_token.as_str()).unwrap();
    let reward = U256::from(task_response.reward);
    let liability_window = u64::from(task_response.liability_window);
    let liability_token=Address::from_str(task_response.liability_token.as_str()).unwrap();
    let liability_amount = U256::from(task_response.liability);
    let expiry = u64::from(task_response.expiry);

    let sig_bytes = hex::decode(task_response.signature).unwrap();
    let signature=Bytes::from(sig_bytes.clone());

    info!("receive relayer response signature:{:?}",hex::encode(sig_bytes.clone()));

    // debug!("invoke the submitTask function parasmeter is:instance:{:?},prover:{:?},reward_token:{:?},reward:{:?},liability_window:{:?},liability_token:{:?},
    // liability_amount:{:?},
    // expiry:{:?},
    // signature:{:?}",
    // instance,addr,reward_token,reward,liability_window,liability_token,liability_amount,expiry,signature);

    //send onchain transcations
    let _ = match submit_task(instance,addr,reward_token,reward,liability_window,liability_token,liability_amount,expiry,signature).await{
        Ok(r) => {
            info!("send submit_task success, tx hash is {:?}",r)
        }
        Err(e) => {
            error!("send submit_task error, reason:{:?}",e)
        },
    };
    Ok(())
}

pub async fn process_task_data(task:String){        //submit the task
    match assign_task(task.clone()).await{
        Ok(()) => (),
        Err(r) => {
            error!("assign the task:{} failed {}",task, r)
        }
    }
}


pub fn test() {
    println!("{}",Utc::now().timestamp());
    let dt = (Utc::now() + chrono::Duration::from_std(Duration::from_secs(100)).unwrap()).timestamp();
    println!("today date + 137 days {}", dt);
}
