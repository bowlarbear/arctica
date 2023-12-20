use bitcoincore_rpc::RpcApi;
use bitcoincore_rpc::bitcoincore_rpc_json::{Timestamp};
use bitcoincore_rpc::bitcoincore_rpc_json::{WalletProcessPsbtResult, WalletCreateFundedPsbtResult};
use bitcoin;
use std::process::Command;
use std::fs;
use std::fs::File;
use std::io::Write;
use home::home_dir;
use secp256k1::{rand, Secp256k1, SecretKey};
use serde_json::{to_string};
use std::collections::HashSet;
use regex::Regex;

use crate::Stdio;

use crate::Error;

//get the current $HOME path //TODO: two to_strings?
pub fn get_home() -> Result<String, Error> {
    Ok(home_dir().ok_or(Error::HomeNotFound())?.to_str().ok_or(Error::HomeNotFound())?.to_string())
}

pub fn get_user() -> Result<String, Error> {
    Ok(get_home()?.split("/").collect::<Vec<&str>>()[2].to_string())
}

pub fn bash(command: &str, args: &Vec<&str>, new_thread: bool) -> Result<String, Error> {
    let mut cmd = Command::new(&command);
    cmd.args(args.as_slice());
    println!("bash: {:?}", cmd);
    if (new_thread) { //TODO: Figure out to see if an error is returned in the first 2 seconds
        let mut child = cmd.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.stdin(Stdio::piped())
		.spawn()?;
        std::thread::sleep(std::time::Duration::from_millis(5000));
        match child.try_wait()? {
            Some(status) => {
                let output = child.wait_with_output()?;
                if status.success() {
                    return Ok(String::from_utf8(output.stdout)?)
                }
                return Err(Error::CommandFailed(format!("{:?}", cmd), String::from_utf8(output.stderr)?))
            },
            None => {}
        }
        return Ok("Command is still running on new thread.".to_string());
    }
    let output = cmd.output()?;
    if !output.status.success() {
        return Err(Error::CommandFailed(format!("{:?}", cmd), String::from_utf8(output.stderr)?));
    }
    Ok(std::str::from_utf8(&output.stdout)?.to_string())
}

// Check for a device mounted in /media with a name that is 36 Characters.
pub fn get_uuid() -> Result<String, Error> {
    let output: String = bash("ls", &vec![&("/media/".to_string()+&get_user()?)], false)?;
    let devices: Vec<&str> = output.split('\n').collect();
	for device in devices {
		if device.chars().count() == 36 {
            return Ok(device.trim().to_string());
		} 
	}
    Err(Error::UUIDNotFound())
}

//check if target path is empty
pub fn is_dir_empty(path: &str) -> bool {
	if let Ok(mut entries) = fs::read_dir(path){
		return entries.next().is_none();
	}
	false
}

//used to store a string param as a file
pub fn store_string(string: String, file_name: &String) -> Result<String, String> {
	let mut file_ref = match std::fs::File::create(file_name) {
		Ok(file) => file,
		Err(err) => return Err(err.to_string()),
	};
	file_ref.write_all(&string.as_bytes()).expect("Could not write string to file");
	Ok(format!("SUCCESS stored with no problems"))
}

//used to store a PSBT param as a file
pub fn store_psbt(psbt: &WalletProcessPsbtResult, file_name: String) -> Result<String, String> {
    let mut file_ref = match std::fs::File::create(file_name) {
        Ok(file) => file,
        Err(err) => return Err(err.to_string()),
    };
    let psbt_json = to_string(&psbt).unwrap();
    file_ref.write_all(&psbt_json.to_string().as_bytes()).expect("Could not write string to file");
    Ok(format!("SUCCESS stored with no problems"))
 }

 //used to store a PSBT param as a file
pub fn store_unsigned_psbt(psbt: &WalletCreateFundedPsbtResult, file_name: String) -> Result<String, String> {
    let mut file_ref = match std::fs::File::create(file_name) {
        Ok(file) => file,
        Err(err) => return Err(err.to_string()),
    };
    let psbt_json = to_string(&psbt).unwrap();
    file_ref.write_all(&psbt_json.to_string().as_bytes()).expect("Could not write string to file");
    Ok(format!("SUCCESS stored with no problems"))
 }

//update the config.txt with the provided params
pub fn write(name: String, value:String) {
	let mut config_file = home_dir().expect("could not get home directory");
    config_file.push("config.txt");
    let mut written = false;
    let mut newfile = String::new();
    //read the config file to a string
    let contents = match fs::read_to_string(&config_file) {
        Ok(ct) => ct,
        Err(_) => {
            "".to_string()       
        }
    };
    //split the contents of the string
    for line in contents.split("\n") {
        let parts: Vec<&str> = line.split("=").collect();
        if parts.len() == 2 {
           let (n,v) = (parts[0],parts[1]); 
           newfile += n;
           newfile += "=";
           if n == name {
            newfile += &value;
            written = true;
           } else {
            newfile += v;
           }
           newfile += "\n";
        }
    }
    if !written {
        newfile += &name;
        newfile += "=";
        newfile += &value;
    }
    let mut file = File::create(&config_file).expect("Could not Open file");
    //write the contents to the config file
    file.write_all(newfile.as_bytes()).expect("Could not rewrite file");
}

//generate an extended public and private keypair
pub fn generate_keypair() -> Result<(String, String), bitcoin::Error> {
	let secp = Secp256k1::new();
    let seed = SecretKey::new(&mut rand::thread_rng()).secret_bytes();
    let xpriv = bitcoin::bip32::ExtendedPrivKey::new_master(bitcoin::Network::Bitcoin, &seed).unwrap();
	let xpub = bitcoin::bip32::ExtendedPubKey::from_priv(&secp, &xpriv);
	Ok((bitcoin::base58::check_encode_slice(&xpriv.encode()), bitcoin::base58::check_encode_slice(&xpub.encode())))
}

//returns the checksum of the descriptor param
pub fn get_descriptor_checksum(descriptor: String) -> String {
    let auth = bitcoincore_rpc::Auth::UserPass("rpcuser".to_string(), "477028".to_string());
    let client = match bitcoincore_rpc::Client::new(&"127.0.0.1:8332".to_string(), auth){
		Ok(client)=> client,
		Err(err)=> return format!("{}", err.to_string())
	};
    //retrieve descriptor info
    let desc_info = match client.get_descriptor_info(&descriptor){
        Ok(info)=> info,
		Err(err)=> return format!("{}", err.to_string())
    };
    println!("Descriptor info: {:?}", desc_info);
    //parse the checksum
    let checksum = desc_info.checksum;
    println!("Checksum: {:?}", checksum);
    let output = &(descriptor.to_string() + "#" + &checksum.to_string());
    println!("output: {:?}", output);
    format!("{}", output)
}

//converts a unix timestamp to block height
pub fn unix_to_block_height(unix_timestamp: i64) -> i64 {
    let genesis_timestamp = 1231006505; //unix timestamp of genesis block
                            // 126230400 4 year period
    let block_interval = 600; //10 minutes in seconds
    let time_since_genesis = unix_timestamp - genesis_timestamp;
    let block_height = time_since_genesis / block_interval;
    block_height
}

//retrieve decay time from the file and output as Timestamp type
pub fn retrieve_decay_time(file: String) -> Timestamp {
	let decay_time_exists = std::path::Path::new(&("/mnt/ramdisk/sensitive/decay/".to_string()+&file.to_string())).exists();
	if decay_time_exists {
        //read the decay time file to a string
		let decay_time: String = match fs::read_to_string(&("/mnt/ramdisk/sensitive/decay/".to_string()+&file.to_string())){
			Ok(decay_time)=> decay_time,
			//return default timestamp
			Err(..)=> return Timestamp::Time(1676511266)
		};
        //parse the decay_time
		match decay_time.trim().parse() {
			Ok(result) => 
			return Timestamp::Time(result),
			Err(..) => 
			//return default timestamp 
			return Timestamp::Time(1676511266)
		};
	} else {
		//return default timestamp
		return Timestamp::Time(1676511266)
	}
}

//retrieve start time from the decay_time file and output as integer
pub fn retrieve_decay_time_integer(file: String) -> i64 {
	let decay_time_exists = std::path::Path::new(&("/mnt/ramdisk/sensitive/decay/".to_string()+&file.to_string())).exists();
	if decay_time_exists {
        //read the decay_time file to a string
		let decay_time: String = match fs::read_to_string(&("/mnt/ramdisk/sensitive/decay/".to_string()+&file.to_string())){
			Ok(decay_time)=> decay_time,
			//return default time stamp
			Err(..)=> return 0
		};
        //parse the decay_time
		match decay_time.trim().parse() {
			Ok(result) => 
			return result,
			Err(..) => 
			//return default timestamp 
			return 0
		};
	} else {
		//return default timestamp
		return 0
	}
}

//get the mount point of the currently inserted disc
pub fn get_cd_path() -> Result<String, String> {
	//query /dev/sr?
	let output = Command::new("bash").arg("-c").arg("ls /dev/sr?").output().unwrap();
	if !output.status.success() {
	  return Err(format!("error querying /dev/sr?, no results found"));
	}
	let paths = std::str::from_utf8(&output.stdout).unwrap().trim();
	let devices: Vec<&str> = paths.trim().split_whitespace().collect();
	//if the result only contains one result we can assume this is the valid path
	match devices.len(){
		0 => Err("Error No valid path found".to_string()),
		1 => Ok(devices[0].to_string()),
		//unsure how to handle this condition beyond throwing an error
		_ => Err("Error multiple paths found".to_string())
	}
	
  }

 //query fdisk list
pub fn run_fdisk() -> Result<String, String> {
	let output = Command::new("sudo").args(["fdisk", "-l"]).output().unwrap();
	if output.status.success(){
		Ok(String::from_utf8_lossy(&output.stdout).to_string())
	}else{
		Err("Failed to run fdisk".to_string())
	}
}

//parse an fdisk list result
pub fn parse_fdisk_result(output: &str) -> String {
	//parse for device mountpoints with regex
	let re = Regex::new(r"Disk (/dev/\w+):").unwrap();
	output.lines()
	//filter results into a String (this is necesary so it may be stored on the front end)
	.filter_map(|line| re.captures(line))
	.filter_map(|caps| caps.get(1).map(|m| m.as_str()))
	.collect::<Vec<&str>>()
	.join(", ")
}

pub fn find_new_device(baseline: &str) -> Result<String, String> {
	//query current fdisk list
	let current_res = match run_fdisk(){
		Ok(result) => 
			result,
			Err(e) => 
			return Err(e.to_string())
	};
	//parse current fdisk list
	let current = parse_fdisk_result(&current_res);
	//split current fdisk and baseline fdisk list into Vectors
	let current_devices: Vec<String> = current.split(", ").map(|s| s.trim().to_string()).collect();
	let baseline_devices: Vec<String> = baseline.split(", ").map(|s| s.trim().to_string()).collect();
	//iterate the vectors into HashSets
	let current_hashset: HashSet<_> = current_devices.clone().into_iter().collect();
	let baseline_hashset: HashSet<_> = baseline_devices.clone().into_iter().map(|s| s.trim_matches('\"').to_string()).collect();
	//obtain a differential
	let new_devices: Vec<_> = current_hashset.difference(&baseline_hashset).cloned().collect();
	match new_devices.len(){
		//no devices found
		0 => Err("ERROR No device found".to_string()),
		//return the target device
		1 => Ok(new_devices[0].clone()),
		//more than one device found
		_ => Err("ERROR More than one device found".to_string())
	}	
}
