use bitcoincore_rpc::RpcApi;
use bitcoincore_rpc::bitcoincore_rpc_json::{AddressType, ImportDescriptors};
use bitcoincore_rpc::bitcoincore_rpc_json::{WalletProcessPsbtResult, ListTransactionResult, Bip125Replaceable, GetTransactionResultDetailCategory, WalletCreateFundedPsbtResult};
use bitcoincore_rpc::bitcoin::Address;
use bitcoincore_rpc::bitcoin::Network;
use bitcoincore_rpc::bitcoin::Script;
use bitcoin;
use bitcoin::consensus::serialize;
use bitcoin::consensus::deserialize;
use bitcoin::psbt::PartiallySignedTransaction;
use std::process::Command;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::{time::Duration};
use std::process::Stdio;
use std::collections::{HashMap, HashSet};
use serde_json::{json};
use serde::{Serialize, Deserialize};
extern crate hex;

use crate::Error;

//import helper.rs module
use crate::helper::{
	bash, get_user, get_home, is_dir_empty, get_uuid, store_psbt, get_descriptor_checksum, retrieve_decay_time, 
	retrieve_decay_time_integer, unix_to_block_height, store_unsigned_psbt
};

//custom structs
#[derive(Clone, Serialize)]
struct CustomTransaction {
	id: i32,
    info: CustomWalletTxInfo,
    detail: CustomGetTransactionResultDetail,
    trusted: Option<bool>,
    comment: Option<String>,
}

#[derive(Clone, Serialize)]
struct CustomWalletTxInfo {
    confirmations: i32,
    blockhash: Option<String>,
    blockindex: Option<usize>,
    blocktime: Option<u64>,
    blockheight: Option<u32>,
    txid: String,
    time: u64,
    timereceived: u64,
    bip125_replaceable: String,
    wallet_conflicts: Vec<String>,
}

#[derive(Clone, Serialize)]
struct CustomGetTransactionResultDetail {
    address: Option<String>,
    category: String,
    amount: i64,
    label: Option<String>,
    vout: u32,
    fee: Option<i64>,
    abandoned: Option<bool>,
}

impl PartialEq for CustomTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.info.txid == other.info.txid &&
        self.detail.address == other.detail.address &&
        self.detail.amount == other.detail.amount
    }
}


//creates a blank wallet equivalent to: ./bitcoin-cli createwallet "wallet name" ___, true, ___, ____
pub fn create_wallet(wallet: String, hwnumber: &String) -> Result<String, String> {
	let auth = bitcoincore_rpc::Auth::UserPass("rpcuser".to_string(), "477028".to_string());
    let client = match bitcoincore_rpc::Client::new(&"127.0.0.1:8332".to_string(), auth){
		Ok(client)=> client,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	//create blank wallet
	let output = match client.create_wallet(&(wallet.to_string()+"_wallet"+&hwnumber.to_string()), None, Some(true), None, None) {
		Ok(file) => file,
		Err(err) => return Err(err.to_string()),
	};
	Ok(format!("SUCCESS creating the wallet {:?}", output))
}

//builds the high security descriptor, 7 of 11 thresh with decay. 4 of the 11 keys will go to the BPS
//note that the BPS keys are not apart of the thresh 5 required to reach spend threshold and are exclusively for satisfying the timelock condition
pub fn build_high_descriptor(keys: &Vec<String>, hwnumber: &String, internal: bool) -> Result<String, String> {
	println!("calculating 4 year block time span");
	//decay1 which is the timelock var
	let four_years_int = retrieve_decay_time_integer("delayed_decay1".to_string()); 
	let four_years = four_years_int.to_string();
	println!("delayed wallet decay1 threshold: {}", four_years);
	//decay2 which is the first threshold decay
	let four_years_two_months_int = retrieve_decay_time_integer("delayed_decay2".to_string()); 
	let four_years_two_months = four_years_two_months_int.to_string();
	println!("delayed wallet decay2 threshold: {}", four_years_two_months);
	//decay3 which is the second threshold decay
	let four_years_four_months_int = retrieve_decay_time_integer("delayed_decay3".to_string()); 
	let four_years_four_months = four_years_four_months_int.to_string();
	println!("delayed wallet decay3 threshold: {}", four_years_four_months);
	//decay4 which is the third threshold decay
	let four_years_six_months_int = retrieve_decay_time_integer("delayed_decay4".to_string()); 
	let four_years_six_months = four_years_six_months_int.to_string();
	println!("delayed wallet decay4 threshold: {}", four_years_six_months);
	//decay5 which is the fourth threshold decay
	let four_years_eight_months_int = retrieve_decay_time_integer("delayed_decay5".to_string()); 
	let four_years_eight_months = four_years_eight_months_int.to_string();
	println!("delayed wallet decay5 threshold: {}", four_years_eight_months);
	println!("reading xpriv");
	//read xpriv from file to string
	let mut private_key = "private_key";
	//internal change condition is true
	if internal {
		private_key = "private_change_key";
	}
	let xpriv = match fs::read_to_string(&("/mnt/ramdisk/sensitive/".to_string()+&(private_key.to_string())+&(hwnumber.to_string()))){
		Ok(xpriv)=> xpriv,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	println!("{}", xpriv);
	//format the descriptor based on which HW is currently booted
	if hwnumber == "1"{
		println!("Found HW = 1");
		let descriptor = format!("wsh(and_v(v:thresh(5,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}),snu:after({}),snu:after({})),thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}))))", xpriv, keys[1], keys[2], keys[3], keys[4], keys[5], keys[6], four_years_two_months, four_years_four_months, four_years_six_months, four_years_eight_months, keys[7], keys[8], keys[9], keys[10], four_years, four_years);
		println!("DESC: {}", descriptor);
		let output: String = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
	}else if hwnumber == "2"{
		println!("Found HW = 2");
		let descriptor = format!("wsh(and_v(v:thresh(5,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}),snu:after({}),snu:after({})),thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}))))", keys[0], xpriv, keys[2], keys[3], keys[4], keys[5], keys[6], four_years_two_months, four_years_four_months, four_years_six_months, four_years_eight_months, keys[7], keys[8], keys[9], keys[10], four_years, four_years);
		println!("DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
	}else if hwnumber == "3"{
		println!("Found HW = 3");
		let descriptor = format!("wsh(and_v(v:thresh(5,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}),snu:after({}),snu:after({})),thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}))))", keys[0], keys[1], xpriv, keys[3], keys[4], keys[5], keys[6], four_years_two_months, four_years_four_months, four_years_six_months, four_years_eight_months, keys[7], keys[8], keys[9], keys[10], four_years, four_years);
		println!("DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
	}else if hwnumber == "4"{
		println!("Found HW = 4");
		let descriptor = format!("wsh(and_v(v:thresh(5,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}),snu:after({}),snu:after({})),thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}))))", keys[0], keys[1], keys[2], xpriv, keys[4], keys[5], keys[6], four_years_two_months, four_years_four_months, four_years_six_months, four_years_eight_months, keys[7], keys[8], keys[9], keys[10], four_years, four_years);
		println!("DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
	}else if hwnumber == "5"{
		println!("Found HW = 5");
		let descriptor = format!("wsh(and_v(v:thresh(5,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}),snu:after({}),snu:after({})),thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}))))", keys[0], keys[1], keys[2], keys[3], xpriv, keys[5], keys[6], four_years_two_months, four_years_four_months, four_years_six_months, four_years_eight_months, keys[7], keys[8], keys[9], keys[10], four_years, four_years);
		println!("DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
	}else if hwnumber == "6"{
		println!("Found HW = 6");
		let descriptor = format!("wsh(and_v(v:thresh(5,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}),snu:after({}),snu:after({})),thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}))))", keys[0], keys[1], keys[2], keys[3], keys[4], xpriv, keys[6], four_years_two_months, four_years_four_months, four_years_six_months, four_years_eight_months, keys[7], keys[8], keys[9], keys[10], four_years, four_years);
		println!("DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
	}else if hwnumber == "7"{
		println!("Found HW = 7");
		let descriptor = format!("wsh(and_v(v:thresh(5,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}),snu:after({}),snu:after({})),thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}))))", keys[0], keys[1], keys[2], keys[3], keys[4], keys[5], xpriv, four_years_two_months, four_years_four_months, four_years_six_months, four_years_eight_months, keys[7], keys[8], keys[9], keys[10], four_years, four_years);
		println!("DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
	//handle time machine descriptors
	}else if hwnumber == "timemachine1"{
		println!("Found HW = timemachine1");
		let timemachinexpriv = match fs::read_to_string(&("/mnt/ramdisk/CDROM/timemachinekeys/time_machine_private_key".to_string()+&(hwnumber.to_string()))){
			Ok(xpriv)=> xpriv,
			Err(err)=> return Err(format!("{}", err.to_string()))
		};
		let descriptor = format!("wsh(and_v(v:thresh(5,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}),snu:after({}),snu:after({})),thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}))))", keys[0], keys[1], keys[2], keys[3], keys[4], keys[5], keys[6], four_years_two_months, four_years_four_months, four_years_six_months, four_years_eight_months, timemachinexpriv, keys[8], keys[9], keys[10], four_years, four_years);
		println!("DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))	
	}else if hwnumber == "timemachine2"{
		println!("Found HW = timemachine2");
		let timemachinexpriv = match fs::read_to_string(&("/mnt/ramdisk/CDROM/timemachinekeys/time_machine_".to_string()+&(private_key.to_string())+&(hwnumber.to_string()))){
			Ok(xpriv)=> xpriv,
			Err(err)=> return Err(format!("{}", err.to_string()))
		};
		let descriptor = format!("wsh(and_v(v:thresh(5,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}),snu:after({}),snu:after({})),thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}))))", keys[0], keys[1], keys[2], keys[3], keys[4], keys[5], keys[6], four_years_two_months, four_years_four_months, four_years_six_months, four_years_eight_months, keys[7], timemachinexpriv, keys[9], keys[10], four_years, four_years);
		println!("DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))	
	}else if hwnumber == "timemachine3"{
		println!("Found HW = timemachine3");
		let timemachinexpriv = match fs::read_to_string(&("/mnt/ramdisk/CDROM/timemachinekeys/time_machine_".to_string()+&(private_key.to_string())+&(hwnumber.to_string()))){
			Ok(xpriv)=> xpriv,
			Err(err)=> return Err(format!("{}", err.to_string()))
		};
		let descriptor = format!("wsh(and_v(v:thresh(5,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}),snu:after({}),snu:after({})),thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}))))", keys[0], keys[1], keys[2], keys[3], keys[4], keys[5], keys[6], four_years_two_months, four_years_four_months, four_years_six_months, four_years_eight_months, keys[7], keys[8], timemachinexpriv, keys[10], four_years, four_years);
		println!("DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))	
	}else if hwnumber == "timemachine4"{
		println!("Found HW = timemachine4");
		let timemachinexpriv = match fs::read_to_string(&("/mnt/ramdisk/CDROM/timemachinekeys/time_machine_".to_string()+&(private_key.to_string())+&(hwnumber.to_string()))){
			Ok(xpriv)=> xpriv,
			Err(err)=> return Err(format!("{}", err.to_string()))
		};
		let descriptor = format!("wsh(and_v(v:thresh(5,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}),snu:after({}),snu:after({})),thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}))))", keys[0], keys[1], keys[2], keys[3], keys[4], keys[5], keys[6], four_years_two_months, four_years_four_months, four_years_six_months, four_years_eight_months, keys[7], keys[8], keys[9], timemachinexpriv, four_years, four_years);
		println!("DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))	
	//create a read only descriptor if no valid param is found
	}else{
		println!("no valid hwnumber param found, creating read only desc");
		let descriptor = format!("wsh(and_v(v:thresh(5,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}),snu:after({}),snu:after({})),thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({}),snu:after({}))))", keys[0], keys[1], keys[2], keys[3], keys[4], keys[5], keys[6], four_years_two_months, four_years_four_months, four_years_six_months, four_years_eight_months, keys[7], keys[8], keys[9], keys[10], four_years, four_years);
		println!("Read only DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))	
	}

}

//build the medium security descriptor, 2 of 7 thresh with decay. 
pub fn build_med_descriptor(keys: &Vec<String>, hwnumber: &String, internal: bool) -> Result<String, String> {
	println!("calculating 4 year block time span");
	//four_years_eight_months is a unix timestamp created with create_setup_cd
    let four_years_eight_months_int = retrieve_decay_time_integer("immediate_decay".to_string()); 
	let four_years_eight_months = four_years_eight_months_int.to_string();
	println!("immediate wallet decay threshold: {}", four_years_eight_months);

	println!("reading xpriv");
	let mut private_key = "private_key";
	//internal change condition is true
	if internal {
		private_key = "private_change_key";
	}
	let xpriv = match fs::read_to_string(&("/mnt/ramdisk/sensitive/".to_string()+&(private_key.to_string())+&(hwnumber.to_string()))){
		Ok(xpriv)=> xpriv,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	println!("{}", xpriv);
	//format the descriptor based on which HW is currently booted
	if hwnumber == "1"{
		println!("Found HW = 1");
		let descriptor = format!("wsh(thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({})))", xpriv, keys[1], keys[2], keys[3], keys[4], keys[5], keys[6], four_years_eight_months);
		println!("DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
	}else if hwnumber == "2"{
		println!("Found HW = 2");
		let descriptor = format!("wsh(thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({})))", keys[0], xpriv, keys[2], keys[3], keys[4], keys[5], keys[6], four_years_eight_months);
		println!("DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
	}else if hwnumber == "3"{
		println!("Found HW = 3");
		let descriptor = format!("wsh(thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({})))", keys[0], keys[1], xpriv, keys[3], keys[4], keys[5], keys[6], four_years_eight_months);
		println!("DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
	}else if hwnumber == "4"{
		println!("Found HW = 4");
		let descriptor = format!("wsh(thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({})))", keys[0], keys[1], keys[2], xpriv, keys[4], keys[5], keys[6], four_years_eight_months);
		println!("DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
	}else if hwnumber == "5"{
		println!("Found HW = 5");
		let descriptor = format!("wsh(thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({})))", keys[0], keys[1], keys[2], keys[3], xpriv, keys[5], keys[6], four_years_eight_months);
		println!("DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
	}else if hwnumber == "6"{
		println!("Found HW = 6");
		let descriptor = format!("wsh(thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({})))", keys[0], keys[1], xpriv, keys[3], keys[4], xpriv, keys[6], four_years_eight_months);
		println!("DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
	}else if hwnumber == "7"{
		println!("Found HW = 7");
		let descriptor = format!("wsh(thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({})))", keys[0], keys[1], keys[2], keys[3], keys[4], keys[5], xpriv, four_years_eight_months);
		println!("DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
	//create a read only descriptor if no valid param is found
	}else{
		println!("no valid hwnumber param found, creating read only desc");
		let descriptor = format!("wsh(thresh(2,pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),s:pk({}),snu:after({})))", keys[0], keys[1], keys[2], keys[3], keys[4], keys[5], keys[6], four_years_eight_months);
		println!("DESC: {}", descriptor);
		let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
	}
}

//builds the low security descriptor, 1 of 7 thresh, used for tripwire
//TODO this needs to use its own special keypair or it will be a privacy leak once implemented
//TODO this may not need child key designators /* because it seems to use hardened keys but have not tested this descriptor yet
//TODO might remove all low wallet assets
	pub fn build_low_descriptor(keys: &Vec<String>, hwnumber: &String, internal: bool) -> Result<String, String> {
		println!("reading xpriv");
		let mut private_key = "private_key";
		//internal change condition is true, use private_change_key instead
		if internal  {
			private_key = "private_change_key";
		}
		let xpriv = match fs::read_to_string(&("/mnt/ramdisk/sensitive/".to_string()+&(private_key)+&(hwnumber.to_string()))){
			Ok(xpriv)=> xpriv,
			Err(err)=> return Err(format!("{}", err.to_string()))
		};
		println!("{}", xpriv);
	//format the descriptor based on which HW is currently booted
	if hwnumber == "1"{
			println!("Found HW = 1");
			let descriptor = format!("wsh(c:or_i(pk_k({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),pk_h({}))))))))", xpriv, keys[1], keys[2], keys[3], keys[4], keys[5], keys[6]);
			println!("DESC: {}", descriptor);
			let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
		}else if hwnumber == "2"{
			println!("Found HW = 2");
			let descriptor = format!("wsh(c:or_i(pk_k({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),pk_h({}))))))))", keys[0], xpriv, keys[2], keys[3], keys[4], keys[5], keys[6]);
			println!("DESC: {}", descriptor);
			let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
		}else if hwnumber == "3"{
			println!("Found HW = 3");
			let descriptor = format!("wsh(c:or_i(pk_k({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),pk_h({}))))))))", keys[0], keys[1], xpriv, keys[3], keys[4], keys[5], keys[6]);
			println!("DESC: {}", descriptor);
			let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
		}else if hwnumber == "4"{
			println!("Found HW = 4");
			let descriptor = format!("wsh(c:or_i(pk_k({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),pk_h({}))))))))", keys[0], keys[1], keys[2], xpriv, keys[4], keys[5], keys[6]);
			println!("DESC: {}", descriptor);
			let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
		}else if hwnumber == "5"{
			println!("Found HW = 5");
			let descriptor = format!("wsh(c:or_i(pk_k({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),pk_h({}))))))))", keys[0], keys[1], keys[2], keys[3], xpriv, keys[5], keys[6]);
			println!("DESC: {}", descriptor);
			let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
		}else if hwnumber == "6"{
			println!("Found HW = 6");
			let descriptor = format!("wsh(c:or_i(pk_k({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),pk_h({}))))))))", keys[0], keys[1], keys[2], keys[3], keys[4], xpriv, keys[6]);
			println!("DESC: {}", descriptor);
			let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
		}else if hwnumber == "7"{
			println!("Found HW = 7");
			let descriptor = format!("wsh(c:or_i(pk_k({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),pk_h({}))))))))", keys[0], keys[1], keys[2], keys[3], keys[4], keys[5], xpriv);
			println!("DESC: {}", descriptor);
			let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
		//create a read only descriptor if no valid param is found
		}else{
			println!("No valid sd card param found, creating read only desc");
			let descriptor = format!("wsh(c:or_i(pk_k({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),or_i(pk_h({}),pk_h({}))))))))", keys[0], keys[1], keys[2], keys[3], keys[4], keys[5], keys[6]);
			println!("DESC: {}", descriptor);
			let output = get_descriptor_checksum(descriptor);
		Ok(format!("{}", output))
		}
	}

//equivalent to ./bitcoin-cli -rpcwallet=<filepath>|"wallet_name" importdescriptors "requests"
//requests is a JSON and is formatted as follows
//'[{"desc": "<descriptor goes here>", "active":true, "range":[0,100], "next_index":0, "timestamp": <start_time_timestamp>}]'
//acceptable params here are "low" & "low_change", "immediate" & "immediate_change", "delayed" & "delayed_change"; hwNumber 1-7; internal: true designates change descriptor
pub fn import_descriptor(wallet: String, hwnumber: &String, internal: bool) -> Result<String, String> {
	let auth = bitcoincore_rpc::Auth::UserPass("rpcuser".to_string(), "477028".to_string());
    let client = match bitcoincore_rpc::Client::new(&("127.0.0.1:8332/wallet/".to_string()+&(wallet.to_string())+"_wallet"+ &(hwnumber.to_string())), auth){
		Ok(client)=> client,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	//if the internal bool is true, filepath is given the change_descriptor designator
	let mut descriptor_str = "_descriptor";
	if internal  {
		descriptor_str = "_change_descriptor"
	}
	//read the descriptor to a string from file
		let desc: String = match fs::read_to_string(&("/mnt/ramdisk/sensitive/descriptors/".to_string()+&(wallet.to_string())+&(descriptor_str.to_string()) + &(hwnumber.to_string()))){
			Ok(desc)=> desc,
			Err(err)=> return Err(format!("{}", err.to_string()))
		};

	//obtain the start time from file
	let start_time = retrieve_decay_time("start_time".to_string());
	let mut change = Some(true);
	if internal == false {
		change = Some(false);
	}
	//import the descriptors into the wallet file
	let output = match client.import_descriptors(ImportDescriptors {
		descriptor: desc,
		timestamp: start_time,
		active: Some(true),
		range: Some((0, 100)),
		next_index: Some(0),
		internal: change,
		label: None
	}){
			Ok(file) => file,
			Err(err) => return Err(err.to_string()),
		
	};
	Ok(format!("Success in importing descriptor...{:?}", output))
}

#[tauri::command]
//get a new address
//accepts "low", "immediate", and "delayed" as a param
//equivalent to... Command::new("./bitcoin-25.0/bin/bitcoin-cli").args([&("-rpcwallet=".to_string()+&(wallet.to_string())+"_wallet"), "getnewaddress"])
//must be done with client url param URL=<hostname>/wallet/<wallet_name>
pub async fn get_address(walletname: String, hwnumber:String) -> Result<String, String> {
	// //need to kill eog here if it's running as it will show a stale QR and/or crash otherwise
	// let pidof = Command::new("pidof").arg("eog").output().unwrap();
	// let pid = std::str::from_utf8(&pidof.stdout).unwrap().trim();
	// //kill pid
	// Command::new("kill").args(["-9", &pid]).output().unwrap();
	let auth = bitcoincore_rpc::Auth::UserPass("rpcuser".to_string(), "477028".to_string());
    let client = match bitcoincore_rpc::Client::new(&("127.0.0.1:8332/wallet/".to_string()+&(walletname.to_string())+"_wallet"+&hwnumber.to_string()), auth){
		Ok(client)=> client,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	//address labels can be added here
	let address_type = Some(AddressType::Bech32);
	let address = match client.get_new_address(None, address_type){
		Ok(addr) => addr,
		Err(err) => return Err(format!("{}", err.to_string()))
	};
	// //create a QR code for the address
	let address_str = address.to_string();
	//delete stale QR file
	Command::new("sudo").args(["rm", "/mnt/ramdisk/qrcode.png"]).output().unwrap();
	//file destination for QR code
	let mut file = match File::create("/mnt/ramdisk/qrcode.svg"){
		Ok(file) => file,
		Err(err) => return Err(format!("{}", err.to_string()))
	};
	//create QR code
	let output = Command::new("qrencode").args(["-s", "6", "-l", "H", "-o", "/mnt/ramdisk/qrcode.png", &address_str]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in generating QR code {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	Ok(format!("{}", address))
}

#[tauri::command]
//calculate the current balance of the declared wallet
pub async fn get_balance(walletname:String, hwnumber:String) -> Result<String, String> {
	let auth = bitcoincore_rpc::Auth::UserPass("rpcuser".to_string(), "477028".to_string());
    let client = match bitcoincore_rpc::Client::new(&("127.0.0.1:8332/wallet/".to_string()+&(walletname.to_string())+"_wallet"+&hwnumber.to_string()), auth){
		Ok(client)=> client,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	//get wallet balance
	match client.get_balance(None, Some(true)){
		Ok(bal) => {
			//split string into a vec and extract the number only without the BTC unit
			let bal_output = bal.to_string();
			let split = bal_output.split(' ');
			let bal_vec: Vec<_> = split.collect();
			return Ok(bal_vec[0].to_string())
			
		},
		Err(err) => return Err(format!("{}", err.to_string()))
	};
}

#[tauri::command]
//retrieve the current transaction history for the provided wallet
pub async fn get_transactions(walletname: String, hwnumber:String) -> Result<String, String> {
	let auth = bitcoincore_rpc::Auth::UserPass("rpcuser".to_string(), "477028".to_string());
    let client = match bitcoincore_rpc::Client::new(&("127.0.0.1:8332/wallet/".to_string()+&(walletname.to_string())+"_wallet"+&hwnumber.to_string()), auth){
		Ok(client)=> client,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
   let transactions: Vec<ListTransactionResult> = match client.list_transactions(None, None, None, Some(true)) {
	Ok(tx) => tx,
	Err(err) => return Err(format!("{}", err.to_string()))
   };
   //empty wallet with no transaction history
   if transactions.is_empty() {
	return Ok(format!("empty123321"))
   }
   else{
	let mut custom_transactions: Vec<CustomTransaction> = Vec::new();
	let mut x = 0;
    //append result to a custom tx struct
	for tx in transactions {
		let custom_tx = CustomTransaction {
			id: x,
			info: CustomWalletTxInfo {
				confirmations: tx.info.confirmations,
				blockhash: tx.info.blockhash.map(|hash| hash.to_string()),
				blockindex: tx.info.blockindex,
				blocktime: tx.info.blocktime,
				blockheight: tx.info.blockheight,
				txid: tx.info.txid.to_string(),
				time: tx.info.time,
				timereceived: tx.info.timereceived,
				bip125_replaceable: match tx.info.bip125_replaceable {
					Bip125Replaceable::Yes => "Yes".to_string(),
					Bip125Replaceable::No => "No".to_string(),
					Bip125Replaceable::Unknown => "Unknown".to_string(),
				},
				wallet_conflicts: tx.info.wallet_conflicts.into_iter().map(|c| c.to_string()).collect(),
			},
			detail: CustomGetTransactionResultDetail {
				address: tx.detail.address.as_ref().map(|addr| addr.to_string()),
				category: match tx.detail.category {
				 GetTransactionResultDetailCategory::Send => "Send".to_string(),
				 GetTransactionResultDetailCategory::Receive => "Receive".to_string(),
				 GetTransactionResultDetailCategory::Generate => "Generate".to_string(),
				 GetTransactionResultDetailCategory::Immature => "Immature".to_string(),
				 GetTransactionResultDetailCategory::Orphan => "Orphan".to_string(),
			 }, 
				amount: tx.detail.amount.to_sat(),
				label: tx.detail.label,
				vout: tx.detail.vout,
				fee: tx.detail.fee.map_or_else(|| None, |x| Some(x.to_sat())),
				abandoned: tx.detail.abandoned,
			},
			trusted: tx.trusted,
			comment: tx.comment,
		};
			custom_transactions.push(custom_tx);
			x += 1;
		
	}
	//check for duplicate txids. 
	//if a batch of txids has >2 outputs & atleast two duplicate amounts & addresses...assume change and filter from results
	let json_string = serde_json::to_string(&custom_transactions).unwrap();
	println!("{}", json_string);
	Ok(format!("{}", json_string))
   }
}

#[tauri::command]
//generate a PSBT for the provided wallet
pub async fn generate_psbt(walletname: String, hwnumber:String, recipient: &str, amount: f64, fee: u64) -> Result<String, String> {
	let auth = bitcoincore_rpc::Auth::UserPass("rpcuser".to_string(), "477028".to_string());
    let client = match bitcoincore_rpc::Client::new(&("127.0.0.1:8332/wallet/".to_string()+&(walletname.to_string())+"_wallet"+&hwnumber.to_string()), auth){
		Ok(client)=> client,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	//create the directory where the PSBT will live if it does not exist
   let a = std::path::Path::new("/mnt/ramdisk/psbt").exists();
   if a == false{
       //make psbt dir
       let output = Command::new("mkdir").args(["/mnt/ramdisk/psbt"]).output().unwrap();
       if !output.status.success() {
    	return Err(format!("ERROR in creating /mnt/ramdisk/psbt dir {}", std::str::from_utf8(&output.stderr).unwrap()));
       }
   }
//create the input JSON
let json_input = json!([]);
//creat the output JSON
let json_output = json!([{
	recipient: amount
}]);
//empty options JSON
let mut options = json!({
});
//if the user specifies a custom fee, append it to the options JSON
if fee != 0{
	options["fee_rate"] = json!(fee);
}
//retrieve the current median blocktime
let locktime = match retrieve_median_blocktime(){
	Ok(locktime)=> locktime,
	Err(err)=> return Err(format!("{}", err.to_string()))
};
//1st attempt
let psbt_output1 = Command::new(&(get_home().unwrap()+"/bitcoin-25.0/bin/bitcoin-cli"))
.args([&("-rpcwallet=".to_string()+&(walletname.to_string())+"_wallet"+&hwnumber.to_string()), 
"walletcreatefundedpsbt", 
&json_input.to_string(), //empty array lets core pick the inputs
&json_output.to_string(), //receive address & output amount
&locktime, //current unix time
&options.to_string() //manually providing fee rate if applicable
]) 
.output()
.unwrap();
//if insufficient funds error, attempt to subtract fees from outputs
let fee_check = std::str::from_utf8(&psbt_output1.stderr).unwrap();
if fee_check.contains("Insufficient funds"){
	options["subtractFeeFromOutputs"] = json!([]);
};
//2nd attempt
let psbt_output2 = Command::new(&(get_home().unwrap()+"/bitcoin-25.0/bin/bitcoin-cli"))
.args([&("-rpcwallet=".to_string()+&(walletname.to_string())+"_wallet"+&hwnumber.to_string()), 
"walletcreatefundedpsbt", 
&json_input.to_string(), //unchanged
&json_output.to_string(), //unchanged
&locktime, //unchanged
&options.to_string() //enable subtractFeeFromOutputs option
]) 
.output()
.unwrap();
//handle error if failed
if !psbt_output2.status.success() {
	return Err(format!("ERROR in generating PSBT = {}", std::str::from_utf8(&psbt_output2.stderr).unwrap()));
}
//convert psbt to string from hex
let psbt_str = String::from_utf8(psbt_output2.stdout).unwrap();
//convert psbt string to an rpc crate struct
let psbt: WalletCreateFundedPsbtResult = match serde_json::from_str(&psbt_str) {
	Ok(psbt)=> psbt,
	Err(err)=> return Err(format!("{}", err.to_string()))
};
//declare the destination path for the PSBT file
let file_dest = "/mnt/ramdisk/psbt/psbt".to_string();
//store the transaction as a file
match store_unsigned_psbt(&psbt, file_dest) {
	Ok(_) => {},
	Err(err) => return Err("ERROR could not store PSBT: ".to_string()+&err)
	};
Ok(format!("PSBT: {:?}", psbt))
}

//init bitcoind, for initialzing bitcoin node on the dummy node without any wallet media inserted
#[tauri::command]
pub async fn init_bitcoind() ->Result<String, String> {
	//Download core if needed
	let bitcoin_tar = std::path::Path::new(&(get_home().unwrap()+"/arctica/bitcoin-25.0-x86_64-linux-gnu.tar.gz")).exists();
	if bitcoin_tar == false{
		let output = Command::new("wget").args(["-O", &(get_home().unwrap()+"/arctica/bitcoin-25.0-x86_64-linux-gnu.tar.gz"),"https://bitcoincore.org/bin/bitcoin-core-25.0/bitcoin-25.0-x86_64-linux-gnu.tar.gz"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in init_bitcoind with downloading bitcoin core = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	//extract bitcoin core
	let bitcoin = std::path::Path::new(&(get_home().unwrap()+"/bitcoin-25.0")).exists();
	if bitcoin == false{
		let output = Command::new("tar").args(["-xzf", &(get_home().unwrap()+"/arctica/bitcoin-25.0-x86_64-linux-gnu.tar.gz"), "-C", &get_home().unwrap()]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in init_bitcoind with extracting bitcoin core = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	//create target device .bitcoin dir
	let dot_bitcoin = std::path::Path::new(&(get_home().unwrap()+"/.bitcoin")).exists();
	if dot_bitcoin == false{
		let output = Command::new("mkdir").arg(&(get_home().unwrap()+"/.bitcoin")).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in init_bitcoind with making target .bitcoin dir = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	//open file permissions on .bitcoin
	let output = Command::new("sudo").args(["chmod", "777", &(get_home().unwrap()+"/.bitcoin")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in init_bitcoind with opening permissions on .bitcoin dir = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}	
	//create bitcoin.conf on target device
	let bitcoin_conf = std::path::Path::new(&(get_home().unwrap()+"/.bitcoin/bitcoin.conf")).exists();
	if bitcoin_conf == false{
		let file = File::create(&(get_home().unwrap()+"/.bitcoin/bitcoin.conf")).unwrap();
		let output = Command::new("echo").args(["-e", "rpcuser=rpcuser\nrpcpassword=477028\nspendzeroconfchange=1"]).stdout(file).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in init_bitcoind, with creating bitcoin.conf = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	Ok(format!("SUCCESS initializing bitcoin daemon"))
}

//start bitcoin core daemon//TODO: rename networkactive to disablenetwork, dont forget to swap
//boolean cases 
#[tauri::command]
pub async fn start_bitcoind(reindex:bool, networkactive:bool, wallets:bool) -> Result<String, Error> {
    println!("start_bitcoind");
	//TODO: See if we can use this less often, the only time this block should be required is immediately following initial setup
    //pgrep bitcoind to check if already running
	let output = Command::new("pgrep").arg("bitcoind").output().unwrap();
	if !output.stdout.is_empty(){
		return Ok(format!("SUCCESS Bitcoin Daemon is already running"));
	}

    if !networkactive {
        println!("start_bitcoind: networkactive == false");
        bash("sudo", &vec!["nmcli", "networking", "off"], false)?;
        if (!bash("ping", &vec!["-c", "1", "linux.org"], false).is_err()) { return Err(Error::NetworkActive()); }
        bash(&(get_home()?+"/bitcoin-25.0/bin/bitcoind"), &vec![
            "-debuglogfile=/mnt/ramdisk/debug.log",
            &format!("-conf={}", get_home()?+"/.bitcoin/bitcoin.conf"),//TODO: no need to specify conf if datadir is listed
            "-walletdir=/mnt/ramdisk/sensitive/wallets",
            "-networkactive=0"], true)?;
    } else {
        println!("start_bitcoind: networkactive == true");
        bash("ping", &vec!["-c", "1", "linux.org"], false).or(Err(Error::NetworkNotActive()))?;
        println!("start_bitcoind: pinged");
    
        //TODO: Better name for wallets(wallets_enabled?)
        if !wallets {
            println!("start_bitcoind: wallets == false");
            bash(&(get_home()?+"/bitcoin-25.0/bin/bitcoind"), &vec![
                "-debuglogfile=/mnt/ramdisk/debug.log",
                &format!("-datadir={}", get_home()?+"/.bitcoin")], true)?;

        } else {
            println!("start_bitcoind: wallets == true");
            //Ensure walletdir exists
            if !std::path::Path::new("/mnt/ramdisk/sensitive/wallets").exists() {
                println!("start_bitcoind: walletdir getting created");
                bash("mkdir", &vec!["/mnt/ramdisk/sensitive/wallets"], false)?;
            }

            let uuid = get_uuid()?;
            println!("start_bitcoind: get_uuid");
            let host = bash("ls", &vec![&("/media/".to_owned()+&get_user()?+"/"+&uuid+"/home")], false)?;
            let host_user = host.trim();
            println!("start_bitcoind: get host");
            if reindex {
                println!("start_bitcoind: reindex == true");
                bash(&(get_home()?+"/bitcoin-25.0/bin/bitcoind"), &vec![
                    "-reindex",
                    "-debuglogfile=/mnt/ramdisk/debug.log",
                    &format!("-conf={}", get_home()?+"/.bitcoin/bitcoin.conf"),
                    &format!("-datadir=/media/{}/{}/home/{}/.bitcoin", get_user()?, uuid, host_user),
                    "-walletdir=/mnt/ramdisk/sensitive/wallets"], true)?;
            } else {
                println!("start_bitcoind: reindex == false");
                bash(&(get_home()?+"/bitcoin-25.0/bin/bitcoind"), &vec![
                    "-debuglogfile=/mnt/ramdisk/debug.log",
                    &format!("-conf={}", get_home()?+"/.bitcoin/bitcoin.conf"),
                    &format!("-datadir=/media/{}/{}/home/{}/.bitcoin", get_user()?, uuid, host_user),
                    "-walletdir=/mnt/ramdisk/sensitive/wallets"], true)?;
            }
        }
    }
////   	//sleep for spooling
////std::thread::sleep(Duration::from_secs(5));
//////pgrep bitcoind to check for a successful start
////let output = Command::new("pgrep").arg("bitcoind").output().unwrap();
////if output.stdout.is_empty(){
////	return Err(format!("ERROR Bitcoin Daemon did not properly start"));
////}
	Ok(format!("SUCCESS in starting bitcoin daemon with networking enabled"))
}


//check Bitcoin Sync Status
#[tauri::command]
pub async fn check_bitcoin_sync_status() -> Result<String, String> {
		//loop to monitor sync progress
		loop{
			//declare the client object
			let auth = bitcoincore_rpc::Auth::UserPass("rpcuser".to_string(), "477028".to_string());
			let client = match bitcoincore_rpc::Client::new(&"127.0.0.1:8332".to_string(), auth){
				Ok(client)=> client,
				Err(err)=> return Err(format!("{}", err.to_string()))
			};
			//query getblockchaininfo
			match client.get_blockchain_info(){
				//if a valid response is received...
				Ok(res) => {
					//sleep and continue the loop in the event that the chain is not synced
					let progress =  res.verification_progress; 
					if progress < 0.9999{
						std::thread::sleep(Duration::from_secs(5));
						continue;
					}
					//break the query loop once the sync exceed 0.9999
					else{
						break;
					}
				},
				//when the daemon is still performing initial block db verification
				Err(_) => {
					//pgrep bitcoind to check if running
					let output = Command::new("pgrep").arg("bitcoind").output().unwrap();
					if output.stdout.is_empty(){
						Command::new("sync").output().unwrap();
						return Err(format!("ERROR Bitcoin Daemon has stopped unexpectedly"))
					}
					//sleep and continue the loop
					std::thread::sleep(Duration::from_secs(5));
					continue;
				},
			};
		}
		Ok(format!("SUCCESS in syncing bitcoin timechain"))
		}

#[tauri::command]
pub async fn stop_bitcoind() -> Result<String, String> {
	//stop bitcoind
	let output = Command::new(&(get_home().unwrap()+"/bitcoin-25.0/bin/bitcoin-cli")).args(["stop"]).output().unwrap();
	if !output.status.success() {
		
		return Err(format!("ERROR in stopping bitcoin daemon = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//sleep for 5 seconds before resolving to allow time for db shutdown
	std::thread::sleep(Duration::from_secs(6));
	//pgrep bitcoind to check for a successful stop
	let output = Command::new("pgrep").arg("bitcoind").output().unwrap();
	if output.stdout.is_empty(){
		Command::new("sync").output().unwrap();
		Ok(format!("SUCCESS in stopping the bitcoin daemon"))
	}else{
		std::thread::sleep(Duration::from_secs(10));
		//pgrep bitcoind to check for a successful stop
		let output = Command::new("pgrep").arg("bitcoind").output().unwrap();
		if output.stdout.is_empty(){
			Command::new("sync").output().unwrap();
			Ok(format!("SUCCESS in stopping the bitcoin daemon"))
		}else{
			Err(format!("ERROR in stopping bitcoin daemon"))
		}
	}
}

// ./bitcoin-cli getdescriptorinfo '<descriptor>'
// analyze a descriptor and report a canonicalized version with checksum added, this is currently not used anywhere
//acceptable params here are "low", "immediate", "delayed"
#[tauri::command]
pub async fn get_descriptor_info(walletname: String) -> Result<String, String> {
	let auth = bitcoincore_rpc::Auth::UserPass("rpcuser".to_string(), "477028".to_string());
    let client = match bitcoincore_rpc::Client::new(&"127.0.0.1:8332".to_string(), auth){
		Ok(client)=> client,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	//read descriptor to a string from file
	let desc: String = match fs::read_to_string(&("/mnt/ramdisk/sensitive/descriptors/".to_string()+&(walletname.to_string())+"_descriptor")){
		Ok(desc)=> desc,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	let desc_info = client.get_descriptor_info(&desc).unwrap();
	Ok(format!("SUCCESS in getting descriptor info {:?}", desc_info))
}

//load a wallet file contained in /mnt/ramdisk/sensitive/wallets
#[tauri::command]
pub async fn load_wallet(walletname: String, hwnumber: String) -> Result<String, String> {
	//sleep time to ensure daemon is fully spooled before making an RPC call (TODO should loop and evaluate the output instead here)
	std::thread::sleep(Duration::from_secs(5));
	let auth = bitcoincore_rpc::Auth::UserPass("rpcuser".to_string(), "477028".to_string());
    let client = match bitcoincore_rpc::Client::new(&("127.0.0.1:8332/wallet/".to_string()+&(walletname.to_string())+"_wallet"+&hwnumber.to_string()), auth){
		Ok(client)=> client,
		Err(err)=> return Err(format!("error connecting to client: {}", err.to_string()))
	};
	// load the specified wallet...using a match statement here throws a JSON RPC error that breaks the loop logic
	client.load_wallet(&(walletname.to_string()+"_wallet"+&(hwnumber.to_string())));
	// parse list_wallets in a continuous loop to verify when rescan is completed
	loop{
		let list = match client.list_wallets(){
			Ok(list)=> list,
			Err(err)=> return Err(format!("error listing wallets: {}", err.to_string()))
		};
		let search_string = &(walletname.to_string()+"_wallet"+&(hwnumber.to_string()));
		//listwallets returns the wallet name as expected...wallet is properly loaded and scanned
		if list.contains(&search_string){
			break;
		}
		//listwallets does not return the wallet name...wallet not finished scanning
		else{
			std::thread::sleep(Duration::from_secs(5));
		}
	}
	Ok(format!("Success in loading {} wallet", walletname))
	}

//./bitcoin-cli getblockchaininfo
#[tauri::command]
pub async fn get_blockchain_info() -> Result<String, String> {
	let auth = bitcoincore_rpc::Auth::UserPass("rpcuser".to_string(), "477028".to_string());
    let client = match bitcoincore_rpc::Client::new(&"127.0.0.1:8332".to_string(), auth){
		Ok(client)=> client,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	//get blockchain info
	let info = client.get_blockchain_info();
	Ok(format!("Results: {:?}", info))
}

//used to export the psbt config and masterkey in memory to a transfer CD
#[tauri::command]
pub async fn export_psbt(progress: String) -> Result<String, String>{
	//sleep for 6 seconds for trigger happy users
	Command::new("sleep").args(["6"]).output().unwrap();
	//create conf for transfer CD
	let a = std::path::Path::new("/mnt/ramdisk/psbt/config.txt").exists();
	if a == false{
		let file = File::create(&("/mnt/ramdisk/psbt/config.txt")).unwrap();
		let output = Command::new("echo").args(["-e", &("psbt=".to_string()+&progress.to_string())]).stdout(file).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR with creating config: {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	let b = std::path::Path::new("/mnt/ramdisk/psbt/masterkey").exists();
	//copy over masterkey, this enables us to extend the login session to an offline wallet
	if b == false{
		let output = Command::new("cp").args(["/mnt/ramdisk/CDROM/masterkey", "/mnt/ramdisk/psbt"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR with copying masterkey = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	Ok(format!("SUCCESS in Exporting PSBT"))
}

//used to sign for a psbt that has already been signed with another wallet and expects the 
//WalletProcessPsbtResult struct rather than the WalletCreateFundedPsbtResult struct. PSBT originates from transfer CDROM here. 
//TODO refactor sign_processed_psbt & sign_funded_psbt into a single function that accepts a param
#[tauri::command]
pub async fn sign_processed_psbt(walletname: String, hwnumber: String, progress: String) -> Result<String, String>{
	let auth = bitcoincore_rpc::Auth::UserPass("rpcuser".to_string(), "477028".to_string());
    let client = match bitcoincore_rpc::Client::new(&("127.0.0.1:8332/wallet/".to_string()+&(walletname.to_string())+"_wallet"+&hwnumber.to_string()), auth){
		Ok(client)=> client,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	//import the psbt from CDROM
	let psbt_str: String = match fs::read_to_string("/mnt/ramdisk/CDROM/psbt"){
		Ok(psbt_str)=> psbt_str,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	//convert result to valid base64
	let psbt: WalletProcessPsbtResult = match serde_json::from_str(&psbt_str) {
		Ok(psbt)=> psbt,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	//attempt to sign the tx
	let signed_result = client.wallet_process_psbt(
		&psbt.psbt,
		Some(true),
		None,
		None,
	);
	let signed = match signed_result{
		Ok(psbt)=> psbt,
		Err(err)=> return Err(format!("Could not sign processed PSBT: {}", err.to_string()))
	};
	let a = std::path::Path::new("/mnt/ramdisk/psbt").exists();
	if a == false {
		let output = Command::new("mkdir").args(["/mnt/ramdisk/psbt"]).output().unwrap();
		if !output.status.success() {
		return Err(format!("ERROR in creating /mnt/ramdisk/psbt dir {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	//declare file dest
	let file_dest = "/mnt/ramdisk/psbt/psbt".to_string();
	//remove stale psbt from /mnt/ramdisk/psbt/psbt
	Command::new("sudo").args(["rm", "/mnt/ramdisk/psbt/psbt"]).output().unwrap();
	//store the signed transaction as a file
	match store_psbt(&signed, file_dest) {
	Ok(_) => {},
	Err(err) => return Err("ERROR could not store PSBT: ".to_string()+&err)
	};
	//remove the stale config.txt
	Command::new("sudo").args(["rm", "/mnt/ramdisk/CDROM/config.txt"]).output().unwrap();
	let file = File::create(&("/mnt/ramdisk/CDROM/config.txt")).unwrap();
	let output = Command::new("echo").args(["-e", &("psbt=".to_string()+&progress.to_string())]).stdout(file).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in sign_processed_psbt with creating config = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}

	Ok(format!("Success in signing: {:?}", signed))
}

#[tauri::command]
//used to sign for the first key in the quorum which will be in the WalletCreateFundedPsbtResult format rather
//than the WalletProcessPsbtResult format used in other circumstances. PSBT originates from RAM here.
//TODO refactor sign_processed_psbt & sign_funded_psbt into a single function that accepts a param
pub async fn sign_funded_psbt(walletname: String, hwnumber: String, progress: String) -> Result<String, String>{
	let auth = bitcoincore_rpc::Auth::UserPass("rpcuser".to_string(), "477028".to_string());
    let client = match bitcoincore_rpc::Client::new(&("127.0.0.1:8332/wallet/".to_string()+&(walletname.to_string())+"_wallet"+&hwnumber.to_string()), auth){
		Ok(client)=> client,
		Err(err)=> return Err(format!("Error establishing client connection: {}", err.to_string()))
	};
	//read the psbt from file
	let psbt_str: String = match fs::read_to_string("/mnt/ramdisk/psbt/psbt"){
		Ok(psbt_str)=> psbt_str,
		Err(err)=> return Err(format!("Error reading PSBT from file: {}", err.to_string()))
	};
	//convert result to WalletCreateFundedPsbtResult
	let psbt: WalletCreateFundedPsbtResult = match serde_json::from_str(&psbt_str) {
		Ok(psbt)=> psbt,
		Err(err)=> return Err(format!("Error parsing PSBT: {}", err.to_string()))
	};
	//attempt to sign the tx
	let signed_result = client.wallet_process_psbt(
		&psbt.psbt,
		Some(true),
		None,
		None,
	);
	let signed = match signed_result{
		Ok(psbt)=> psbt,
		Err(err)=> return Err(format!("Error signing PSBT: {}", err.to_string()))
	};
	//remove the stale psbt
	Command::new("sudo").args(["rm", "/mnt/ramdisk/psbt/psbt"]).output().unwrap();
	//declare file dest
	let file_dest = "/mnt/ramdisk/psbt/psbt".to_string();
	//remove stale psbt from /mnt/ramdisk/psbt/psbt
	Command::new("sudo").args(["rm", "/mnt/ramdisk/psbt/psbt"]).output().unwrap();
	//store the signed transaction as a file
	match store_psbt(&signed, file_dest) {
	Ok(_) => {},
	Err(err) => return Err("ERROR could not store PSBT: ".to_string()+&err)
	};
	Ok(format!("Reading PSBT from file: {:?}", signed))
}

//broadcast fully signed psbt 
#[tauri::command]
pub async fn broadcast_tx(walletname: String, hwnumber: String) -> Result<String, String>{
	let auth = bitcoincore_rpc::Auth::UserPass("rpcuser".to_string(), "477028".to_string());
    let client = match bitcoincore_rpc::Client::new(&("127.0.0.1:8332/wallet/".to_string()+&(walletname.to_string())+"_wallet"+&hwnumber.to_string()), auth){
		Ok(client)=> client,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	//copy the psbt from CDROM to /mnt/ramdisk/psbt/ if necessary
	let a = std::path::Path::new("/mnt/ramdisk/CDROM/psbt").exists();
	let b = std::path::Path::new("/mnt/ramdisk/psbt/psbt").exists();
	if a  && b == false{
		Command::new("mkdir").arg("/mnt/ramdisk/psbt").output().unwrap();
		let output = Command::new("cp").args(["/mnt/ramdisk/CDROM/psbt", "/mnt/ramdisk/psbt"]).output().unwrap();
			if !output.status.success() {
			return Err(format!("ERROR in psbt from CDROM dir to psbt dir{}", std::str::from_utf8(&output.stderr).unwrap()));
			}
	}
	//read the psbt from file
	let psbt_str: String = match fs::read_to_string("/mnt/ramdisk/psbt/psbt"){
		Ok(psbt_str)=> psbt_str,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	//convert result to valid base64
	let psbt: WalletProcessPsbtResult = match serde_json::from_str(&psbt_str) {
		Ok(psbt)=> psbt,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	//finalize the psbt
	let finalized_result = client.finalize_psbt(
		&psbt.psbt,
		None,
	);
	let finalized = match finalized_result{
		Ok(tx)=> tx.hex.unwrap(),
		Err(err)=> return Err(format!("{}", err.to_string()))	
	};
	let finalized_str = hex::encode(finalized);

	//broadcast the tx
	let broadcast_output = Command::new(&(get_home().unwrap()+"/bitcoin-25.0/bin/bitcoin-cli"))
		.args([&("-rpcwallet=".to_string()+&(walletname.to_string())+"_wallet"+&hwnumber.to_string()), 
		"sendrawtransaction", 
		&finalized_str
		]) 
		.output()
		.unwrap();
		if !broadcast_output.status.success() {
			return Err(format!("ERROR in broadcasting PSBT = {}", std::str::from_utf8(&broadcast_output.stderr).unwrap()));
		}
		//convert psbt to string from hex
		let broadcast = String::from_utf8(broadcast_output.stdout).unwrap();

	//remove stale psbt from ramdisk
	Command::new("sudo").args(["rm", "-r", "/mnt/ramdisk/psbt"]).output().unwrap();
	Ok(format!("Broadcasting Fully Signed TX: {:?}", broadcast))
}

//used to decode a processed PSBT and display tx details on the front end
//TODO refactor decode_funded_psbt & decode_processed_psbt into a single function that accepts a param
#[tauri::command]
pub async fn decode_processed_psbt(walletname: String, hwnumber: String) -> Result<String, String>{
	let auth = bitcoincore_rpc::Auth::UserPass("rpcuser".to_string(), "477028".to_string());
    let client = match bitcoincore_rpc::Client::new(&("127.0.0.1:8332/wallet/".to_string()+&(walletname.to_string())+"_wallet"+&hwnumber.to_string()), auth){
		Ok(client)=> client,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	//copy the psbt from CDROM to ramdisk if necessary
	let a = std::path::Path::new("/mnt/ramdisk/CDROM/psbt").exists();
	let b = std::path::Path::new("/mnt/ramdisk/psbt/psbt").exists();
	if a  && b == false{
		Command::new("mkdir").arg("/mnt/ramdisk/psbt").output().unwrap();
		let output = Command::new("cp").args(["/mnt/ramdisk/CDROM/psbt", "/mnt/ramdisk/psbt"]).output().unwrap();
			if !output.status.success() {
			return Err(format!("ERROR in psbt from CDROM dir to psbt dir{}", std::str::from_utf8(&output.stderr).unwrap()));
			}
	}
	//read the psbt from file
	let psbt_str: String = match fs::read_to_string("/mnt/ramdisk/psbt/psbt"){
		Ok(psbt_str)=> psbt_str,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	//convert result to valid base64
	let psbt: WalletProcessPsbtResult = match serde_json::from_str(&psbt_str) {
		Ok(psbt)=> psbt,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	//decode the psbt
	let psbt_bytes = base64::decode(&psbt.psbt).unwrap();
	let psbtx: PartiallySignedTransaction = PartiallySignedTransaction::deserialize(&psbt_bytes[..]).unwrap();
	// Calculate the total fees for the transaction
	let fee_amount = psbtx.fee().unwrap();
	let fee = fee_amount.to_btc();
	//establish a baseline index for the output vector
	let mut x = 0;
	let length = psbtx.unsigned_tx.output.len();
	//attempt to filter out change output
	while length > x {
		//obtain scriptpubkey for output at index x
		let script_pubkey = psbtx.unsigned_tx.output[x].script_pubkey.as_script(); 
		//obtain amount of output
		let amount = psbtx.unsigned_tx.output[x].value;
		//derive address from scriptpubkey
		let address = match bitcoin::Address::from_script(script_pubkey, bitcoin::Network::Bitcoin){
			Ok(address)=> address,
			Err(err)=> return Err(format!("{}", err.to_string()))
        };
		//check if address ismine: true
		let address_info_result: Result<bitcoincore_rpc::json::GetAddressInfoResult, bitcoincore_rpc::Error> = client.call("getaddressinfo", &[address.to_string().into()]); 
        let address_info = match address_info_result {
			Ok(info)=>info,
			Err(err)=> return Err(format!("{}", err.to_string()))
		};
		//if the address is not recognized, return the results
		if address_info.is_mine == Some(false) {
			return Ok(format!("address={:?}, amount={:?}, fee={:?}", address, amount, fee))
		//else continue to iterate through the vector
		}else{
			x += 1;
		}
	}
	//fallback if the user is sending to their own wallet
	//obtain scriptpubkey for output at index 0
	let script_pubkey = psbtx.unsigned_tx.output[0].script_pubkey.as_script(); 
	//obtain amount of output
	let amount = psbtx.unsigned_tx.output[0].value;
	//derive address from scriptpubkey
	let address = match bitcoin::Address::from_script(script_pubkey, bitcoin::Network::Bitcoin){
		Ok(address)=> address,
		Err(err)=> return Err(format!("{}", err.to_string()))
    };
	Ok(format!("address={:?}, amount={:?}, fee={:?}", address, amount, fee))
}

//used to decode a walletcreatefundedpsbt result
//TODO refactor decode_funded_psbt & decode_processed_psbt into a single function that accepts a param
#[tauri::command]
pub async fn decode_funded_psbt(walletname: String, hwnumber: String) -> Result<String, String> {
	let auth = bitcoincore_rpc::Auth::UserPass("rpcuser".to_string(), "477028".to_string());
    let client = match bitcoincore_rpc::Client::new(&("127.0.0.1:8332/wallet/".to_string()+&(walletname.to_string())+"_wallet"+&hwnumber.to_string()), auth){
		Ok(client)=> client,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	//check if this file path exists
	let a = std::path::Path::new("/mnt/ramdisk/psbt/psbt").exists();
	let psbt_str: String;
	//if it does exist, read the psbt from file
	if a {
		psbt_str = match fs::read_to_string("/mnt/ramdisk/psbt/psbt"){
			Ok(psbt_str)=> psbt_str,
			Err(err)=> return Err(format!("{}", err.to_string()))
		};
		//if it doesn't exist then we can assume the psbt is still on the transfer CD
	}else{
		psbt_str = match fs::read_to_string("/mnt/ramdisk/CDROM/psbt"){
			Ok(psbt_str)=> psbt_str,
			Err(err)=> return Err(format!("{}", err.to_string()))
		};
	}
	//convert result to WalletCreateFundedPsbtResult struct
	let psbt: WalletCreateFundedPsbtResult = match serde_json::from_str(&psbt_str) {
		Ok(psbt)=> psbt,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	//calculate the fee 
	let fee = psbt.fee.to_btc();
	//convert the byte slice to a PartiallySignedTransaction Struct
	let psbt_bytes = base64::decode(&psbt.psbt).unwrap();
	let psbtx: PartiallySignedTransaction = PartiallySignedTransaction::deserialize(&psbt_bytes[..]).unwrap();
	//establish a baseline index for the output vector
	let mut x = 0;
	let length = psbtx.unsigned_tx.output.len();
	//attempt to filter out change output
	while length > x {
		//obtain amount of output
		let amount = psbtx.unsigned_tx.output[x].value;
		//obtain scriptpubkey for output at index x
		let script_pubkey = psbtx.unsigned_tx.output[x].script_pubkey.as_script(); 
		//derive address from scriptpubkey
		let address = match bitcoin::Address::from_script(script_pubkey, bitcoin::Network::Bitcoin){
			Ok(address)=> address,
			Err(err)=> return Err(format!("{}", err.to_string()))
        };
		//check if address ismine: true
		let address_info_result: Result<bitcoincore_rpc::json::GetAddressInfoResult, bitcoincore_rpc::Error> = client.call("getaddressinfo", &[address.to_string().into()]); 
        let address_info = match address_info_result {
			Ok(info)=>info,
			Err(err)=> return Err(format!("{}", err.to_string()))
		};
		//if the address is not recognized, return the results
		if address_info.is_mine == Some(false) {
			return Ok(format!("address={:?}, amount={:?}, fee={:?}", address, amount, fee))
		//else continue to iterate through the vector
		}else{
			x += 1;
		}
	}
	//fallback if the user is sending to their own wallet
	//obtain scriptpubkey for output at index 0
	let script_pubkey = psbtx.unsigned_tx.output[0].script_pubkey.as_script(); 
	//obtain amount of output
	let amount = psbtx.unsigned_tx.output[0].value;
	//derive address from scriptpubkey
	let address = match bitcoin::Address::from_script(script_pubkey, bitcoin::Network::Bitcoin){
		Ok(address)=> address,
		Err(err)=> return Err(format!("{}", err.to_string()))
    };
	Ok(format!("address={:?}, amount={:?}, fee={:?}", address, amount, fee))
}

//retrieve current median block time
#[tauri::command]
pub fn retrieve_median_blocktime() -> Result<String, String>{
    let auth = bitcoincore_rpc::Auth::UserPass("rpcuser".to_string(), "477028".to_string());
    let client = match bitcoincore_rpc::Client::new(&"127.0.0.1:8332".to_string(), auth){
		Ok(client)=> client,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
    let time_med = client.get_blockchain_info().unwrap().median_time;
	// let time_parsed: u64 = time_med.parse();
	let time = time_med - 1000;
    Ok(format!("{}", time.to_string()))
}



//simulated time machine pseudo code, take the user all the way to broadcast
#[tauri::command]
pub fn simulate_time_machine() -> Result<String, String>{
	//in the future, the user will have to obtain the time machine xprivs (or descriptors)
	//from the time machine operator and bring those xprivs back to their machine in order to construct the proper time machine descriptors & wallets

	//ALL these steps need to happen before this function fires...
	//1. obtain the psbt from the transfer CD
	// media/ubuntu/CDROM/psbt
	//2. prompt the user to insert the setup CD (needed to obtain the time machine keys).
	//3. copy setup CD to ramdisk

	//TODO potentially, rather than building the descriptors here...
	//let's build them at create_descriptor stage of things and keep them with the timemachinekeys, they can be encrypted while stored with the BPS


	//obtain the delayed descriptor from sensitive
	// /mnt/ramdisk/sensitive/descriptors/delayed_descriptor1

	//obtain the HW1 xpub
	// /mnt/ramdisk/sensitive/public_key1

	//modify the descriptor to use HW1 xpub

	//obtain the time_machine_xpriv1
	// media/ubuntu/CDROM/timemachinekeys/time_machine_private_key1

	//obtain the time_machine_xpriv2
	// media/ubuntu/CDROM/timemachinekeys/time_machine_private_key2

	//modify the descriptor to use Time_machine_xpriv1 and output as time_machine_descriptor1

	//modify the descriptor to use time_machine_xpriv2 and output as time_machine_descriptor2

	//create blank time_machine1 wallet

	//create blank time_machine2 wallet

	//import time_machine_descriptor1 into time_machine1 wallet

	//sign the psbt with time_machine1 wallet

	//import time_machine_descriptor2 into time_machine2 wallet

	//sign the new psbt with time_machine2 wallet

	//finalize the psbt and output to where delayedBroadcast expects to find it

	//take the user to broadcast (front end can handle this)

	Ok(format!("success"))
}
