//WARNING: Never use snake_case for function params that will be invoked by tauri, it converts them to camelCase and breaks the app

//NOTE: Backend functions typically return Result<String, String> so that either Ok() results or Err() results can be interpreted by the front end, logs can be observed within the application's debug console

#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use bitcoincore_rpc::{Client};
use std::sync::{Mutex};
use std::process::{Command, Stdio};
use std::fs;
use std::fs::File;
use home::home_dir;
use std::thread;
use std::io::{self, Write, BufRead, BufReader};
use std::path::Path;

mod error;
use error::Error;

//import helper.rs module
mod helper;
use helper::{
	bash, get_user, get_home, is_dir_empty, get_uuid, get_cd_path, eject_disc,
	write, retrieve_decay_time_integer, parse_fdisk_result, run_fdisk, find_new_device
};

//import init.rs module
mod init;
use init::{
	pre_install, init_iso, create_bootable_usb, 
};

//import setup.rs module
mod setup;
use setup::{
	create_setup_cd, generate_store_key_pair, 
	generate_store_simulated_time_machine_key_pair, create_descriptor, install_cold_deps, distribute_shards_hw2, 
	distribute_shards_hw3, distribute_shards_hw4, distribute_shards_hw5, distribute_shards_hw6,
	distribute_shards_hw7, create_backup, make_backup, install_warm_deps,
};

//import bitcoin.rs module
mod bitcoin_wallet;
use bitcoin_wallet::{
	get_address, get_balance, get_transactions, generate_psbt, start_bitcoind, init_bitcoind, check_bitcoin_sync_status,
	stop_bitcoind, decode_processed_psbt, broadcast_tx, sign_processed_psbt, export_psbt, get_blockchain_info, 
	load_wallet, get_descriptor_info, decode_funded_psbt, sign_funded_psbt, retrieve_median_blocktime,
};

struct TauriState(Mutex<Option<Client>>);

#[tauri::command]
fn test_function() -> Result<String, Error> {
    println!("runnig test_function");
    bash("ls", &vec!["-a", "~/testdir"], false)
}

#[tauri::command]
//config lives in $HOME, conditional logic evaluates config params and sets application state on the front end
fn read() -> std::string::String {
    let mut config_file = home_dir().expect("could not get home directory");
    println!("{}", config_file.display());
    config_file.push("config.txt");
	//read the config file to string
    let contents = match fs::read_to_string(&config_file) {
        Ok(ct) => ct,
        Err(_) => {
        	"".to_string()
        }
    };
	//split the config string
    for line in contents.split("\n") {
        let parts: Vec<&str> = line.split("=").collect();
        if parts.len() == 2 {
            let (n,v) = (parts[0],parts[1]);
            println!("read line: {}={}", n, v);
        }
    }
    format!("{}", contents)
}

//copy the contents of the currently inserted CD to ramdisk at /mnt/ramdisk/CDROM
#[tauri::command]
async fn copy_cd_to_ramdisk() -> Result<String, String> {
	Command::new("sleep").args(["4"]).output().unwrap();
	//obtain CD path
	let path = match get_cd_path(){
		Ok(path) => path,
        Err(er) => {
        	return Err(format!("{}", er))
        }
	};
	//check if a CDROM is inserted
	let a = std::path::Path::new(&path).exists();
	if a == false {
		let er = "ERROR in copy_cd_to_ramdisk: No CD inserted";
		return Err(format!("{}", er))
	}
	//remove stable psbts
	let b = std::path::Path::new("/mnt/ramdisk/CDROM/psbt").exists();
	if b {
		let output = Command::new("sudo").args(["rm", "/mnt/ramdisk/CDROM/psbt"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in removing stale psbt = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	//copy cd contents to ramdisk
	let output = Command::new("cp").args(["-R", &("/media/".to_string()+&get_user().unwrap()+"/CDROM"), "/mnt/ramdisk"]).output().unwrap();
	if !output.status.success() {
    	return Err(format!("ERROR in copying CD contents = {}", std::str::from_utf8(&output.stderr).unwrap()));
    }
	//open up permissions
	let output = Command::new("sudo").args(["chmod", "-R", "777", "/mnt/ramdisk/CDROM"]).output().unwrap();
	if !output.status.success() {
    	return Err(format!("ERROR in opening file permissions of CDROM = {}", std::str::from_utf8(&output.stderr).unwrap()));
    }

	Ok(format!("SUCCESS in coyping CD contents"))
}

//read the config file of the currently inserted CD/DVD/M-DISC
#[tauri::command]
fn read_cd() -> Result<String, String> {
	Command::new("sleep").args(["4"]).output().unwrap();
	//check if a CDROM is inserted
	let path = match get_cd_path(){
		Ok(path) => path,
        Err(er) => {
        	return Err(format!("{}", er))
        }
	};
	//check for config
	let config_file = &("/media/".to_string()+&get_user().unwrap()+"/CDROM/"+"config.txt");
    let contents = match fs::read_to_string(&config_file) {
        Ok(ct) => ct,
        Err(_) => {
        	"".to_string()
        }
    };
	//split the config string
    for line in contents.split("\n") {
        let parts: Vec<&str> = line.split("=").collect();
        if parts.len() == 2 {
            let (n,v) = (parts[0],parts[1]);
            println!("read line: {}={}", n, v);
        }
    }
    Ok(format!("{}", contents))
}

#[tauri::command]
//wipe and rewrite the currently inserted disc with the contents of /mnt/ramdisk/CDROM
async fn refresh_cd(psbt: bool) -> Result<String, String> {
	//create iso from psbt dir (if psbt true)
	if psbt {
		let output = Command::new("genisoimage").args(["-r", "-J", "-o", "/mnt/ramdisk/transferCD.iso", "/mnt/ramdisk/psbt"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR creating psbt iso with genisoimage = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	} else {
	//create iso from the standard CD dir (if psbt false)
	let output = Command::new("genisoimage").args(["-r", "-J", "-o", "/mnt/ramdisk/transferCD.iso", "/mnt/ramdisk/CDROM"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR refreshing CD with genisoimage = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	}
	//check if the CDROM is blank
	let dir_path = "/media/ubuntu/CDROM";
	let is_empty = is_dir_empty(dir_path);
	let dir_path_exists = std::path::Path::new(dir_path).exists();
	let path = match get_cd_path(){
		Ok(path) => path,
        Err(er) => {
        	return Err(format!("{}", er))
        }
	};
	//unmount the disc
	Command::new("sudo").args(["umount", &path]).output().unwrap();
	//if not blank, wipe the CD
	if dir_path_exists == true && is_empty == false{
		let output = Command::new("sudo").args(["wodim", "-v", &("dev=".to_string()+&path), "blank=fast"]).output().unwrap();
		if !output.status.success() {
			//attempt alternative wipe method
			Command::new("sudo").args(["wodim", "-v", &("dev=".to_string()+&path), "blank=fast"]).output().unwrap();
		}
	}
	//burn setupCD iso to the setupCD
	let output = Command::new("sudo").args(["wodim", &("dev=".to_string()+&path), "-v", "-data", "/mnt/ramdisk/transferCD.iso"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in refreshing CD with burning iso = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//eject the disc
	match eject_disc(){
		Ok(res) => res,
		Err(e) => return Err(format!("ERROR in ejecting CD = {}", e))
	};

	Ok(format!("SUCCESS in refreshing CD"))
}

//eject the current disc
//please note that this is different than the helper function eject_disc() which is called internally only
#[tauri::command]
async fn eject_cd() -> Result<String, String> {
	//eject the current optical disc
	match eject_disc(){
		Ok(res) => return Ok(format!("SUCCESS in ejecting CD")),
		Err(e) => return Err(format!("ERROR in ejecting CD = {}", e))
	}
}

//pack up and encrypt the contents of the sensitive directory into an encrypted.gpg file in $HOME
#[tauri::command]
async fn packup(hwnumber: String) -> Result<String, String> {
	//verify that we are not overwriting the current encrypted tarball with an empty directory
	let a = std::path::Path::new(&("/mnt/ramdisk/sensitive/private_key".to_string()+&hwnumber.to_string())).exists();
	let b = std::path::Path::new(&("/mnt/ramdisk/sensitive/public_key".to_string()+&hwnumber.to_string())).exists();
	if a == false || b == false {
		return Err(format!("Error in Packup, empty sensitive dir found, aborting overwrite"))
	}
	println!("Key material found, packing up sensitive info");
	//remove stale encrypted dir
	let c = std::path::Path::new(&(get_home().unwrap()+"/encrypted.gpg")).exists();
	if c {
		Command::new("sudo").args(["rm", &(get_home().unwrap()+"/encrypted.gpg")]).output().unwrap();
	}
	//remove stale tarball
	let d = std::path::Path::new("/mnt/ramdisk/unecrypted.tar").exists();
	if d {
		Command::new("sudo").args(["rm", "/mnt/ramdisk/unecrypted.tar"]).output().unwrap();
	}
	//pack the sensitive directory into a tarball
	let output = Command::new("tar").args(["cvf", "/mnt/ramdisk/unencrypted.tar", "/mnt/ramdisk/sensitive"]).output().unwrap();
	if !output.status.success() {
    	return Err(format!("ERROR in packup with compression = {}", std::str::from_utf8(&output.stderr).unwrap()));
    }
	//encrypt the tarball 
	let output = Command::new("gpg").args(["--batch", "--passphrase-file", "/mnt/ramdisk/CDROM/masterkey", "--output", &(get_home().unwrap()+"/encrypted.gpg"), "--symmetric", "/mnt/ramdisk/unencrypted.tar"]).output().unwrap();
	if !output.status.success() {
    	return Err(format!("ERROR in packup with encryption = {}", std::str::from_utf8(&output.stderr).unwrap()));
    }
	//Flush the Filesystem Buffers
	Command::new("sync").output().unwrap();
	Ok(format!("SUCCESS in packup"))
}

//decrypt & unpack the contents of encrypted.gpg from $HOME into ramdisk /mnt/ramdisk/sensitive
#[tauri::command]
async fn unpack() -> Result<String, String> {
	println!("unpacking sensitive info");
	//remove stale tarball
	let a = std::path::Path::new("/mnt/ramdisk/decrypted.out").exists();
	if a {
		Command::new("sudo").args(["rm", "/mnt/ramdisk/decrypted.out"]).output().unwrap();
	}
	//decrypt sensitive directory
	let output = Command::new("gpg").args(["--batch", "--passphrase-file", "/mnt/ramdisk/CDROM/masterkey", "--output", "/mnt/ramdisk/decrypted.out", "--decrypt", &(get_home().unwrap()+"/encrypted.gpg")]).output().unwrap();
	if !output.status.success() {
    	return Err(format!("ERROR in unpack with decrypting = {}", std::str::from_utf8(&output.stderr).unwrap()));
    }
	// unpack sensitive directory tarball
	let output = Command::new("tar").args(["xvf", "/mnt/ramdisk/decrypted.out", "-C", "/mnt/ramdisk/"]).output().unwrap();
	if !output.status.success() {
    	return Err(format!("ERROR in unpack with extracting = {}", std::str::from_utf8(&output.stderr).unwrap()));
    }
    // copy sensitive dir to ramdisk
	let output = Command::new("cp").args(["-R", "/mnt/ramdisk/mnt/ramdisk/sensitive", "/mnt/ramdisk"]).output().unwrap();
	if !output.status.success() {
    	return Err(format!("ERROR in unpack with copying dir = {}", std::str::from_utf8(&output.stderr).unwrap()));
    }
	// remove nested sensitive tarball output
	Command::new("sudo").args(["rm", "-r", "/mnt/ramdisk/mnt"]).output().unwrap();
	// #NOTES:
	// #can use this to append files to a decrypted tarball without having to create an entire new one
	// #tar rvf output_tarball ~/filestobeappended
	Ok(format!("SUCCESS in unpack"))
}

//create and mount the ramdisk path for senstive data at /mnt/ramdisk
#[tauri::command]
async fn create_ramdisk() -> Result<String, String> {
	//check if the ramdisk already exists and has been used by Arctica this session
	let a = std::path::Path::new("/mnt/ramdisk/sensitive").exists();
	let b = std::path::Path::new("/mnt/ramdisk/CDROM").exists();
    if a  || b {
		return Ok(format!("Ramdisk already exists"));
	}
	else{
		//disable swapiness
		let output = Command::new("sudo").args(["swapoff", "-a"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in disabling swapiness {}", std::str::from_utf8(&output.stderr).unwrap()));
			}
		//open arctica directory permissions (putting this here because this script always runs at startup)
		let output = Command::new("sudo").args(["chmod", "777", &(get_home().unwrap()+"/arctica")]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in opening file permissions of $HOME/arctica = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
		//check if ramdisk exists
		let c = std::path::Path::new("/mnt/ramdisk").exists();
		if c == false {
			//if not, create the ramdisk
			let output = Command::new("sudo").args(["mkdir", "/mnt/ramdisk"]).output().unwrap();
			if !output.status.success() {
			return Err(format!("ERROR in making /mnt/ramdisk dir {}", std::str::from_utf8(&output.stderr).unwrap()));
			}
		}
		//allocate the RAM for ramdisk 
		let output = Command::new("sudo").args(["mount", "-t", "ramfs", "-o", "size=250M", "ramfs", "/mnt/ramdisk"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in Creating Ramdisk = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
		//open ramdisk file permissions
		let output = Command::new("sudo").args(["chmod", "777", "/mnt/ramdisk"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in Creating Ramdisk = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
		//make the target dir for encrypted payload to or from Hardware Wallets
		let output = Command::new("mkdir").args(["/mnt/ramdisk/sensitive"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in Creating /mnt/ramdiskamdisk/sensitive = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
		//make the debug.log file
		let output = Command::new("touch").args(["/mnt/ramdisk/debug.log"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in Creating debug.log = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	Ok(format!("SUCCESS in Creating Ramdisk"))
	}
}

#[tauri::command]
//mount the internal storage drive at /media/$USER/$UUID and symlink internal .bitcoin/chainstate and ./bitcoin/blocks
//assumes a default ubuntu install on the internal disk without any custom partitioning TODO: make this more robust if possible
async fn mount_internal() -> Result<String, String> {
	//check for an existing mount
	let uuid = match get_uuid(){
		Ok(uuid) => uuid,
		Err(Error::UUIDNotFound())=>"none".to_string(),
		Err(e)=> return Err(format!("Error getting UUID"))
	};
	if uuid == "none"{
		//mount the internal drive if NVME
		Command::new("udisksctl").args(["mount", "--block-device", "/dev/nvme0n1p2"]).output().unwrap();
		//mount internal drive if SATA
		Command::new("udisksctl").args(["mount", "--block-device", "/dev/sda2"]).output().unwrap();
		//pgrep bitcoind to check if running
		let output = Command::new("pgrep").arg("bitcoind").output().unwrap();
		if !output.stdout.is_empty(){
			//Attempt to shut down bitcoin core
			let output = Command::new(&(get_home().unwrap()+"/bitcoin-25.0/bin/bitcoin-cli")).args(["stop"]).output().unwrap();
			//bitcoin core shutdown fails (meaning it was not running)...
			if output.status.success() {
				//function succeeds, core is running for some reason, wait 15s for daemon to stop
				Command::new("sleep").args(["15"]).output().unwrap();
			}
		}
		//obtain the UUID of the currently mounted internal storage drive
		let uuid = match get_uuid(){
			Ok(uuid) => uuid,
			Err(Error::UUIDNotFound())=> return Err(format!("ERROR could not find a valid UUID in /media/$user")),
			Err(e)=> return Err(format!("Error getting UUID"))
		};
		//obtain the username of the internal storage device
		let host = Command::new(&("ls")).args([&("/media/".to_string()+&get_user().unwrap()+"/"+&(uuid.to_string())+"/home")]).output().unwrap();
		if !host.status.success() {
			return Err(format!("ERROR in parsing /media/user/uuid/home {}", std::str::from_utf8(&host.stderr).unwrap()));
		} 
		let host_user = std::str::from_utf8(&host.stdout).unwrap().trim();
		//open the file permissions for local host user dir
		let output = Command::new("sudo").args(["chmod", "777", &("/media/".to_string()+&get_user().unwrap()+"/"+&(uuid.to_string())+"/home/"+&(host_user.to_string()))]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in opening internal storage dir file permissions {}", std::str::from_utf8(&output.stderr).unwrap()));
		} 
		//make internal storage bitcoin dotfiles at /media/ubuntu/$UUID/home/$HOST_USER/.bitcoin/blocks & /media/ubuntu/$UUID/home/$HOST_USER/.bitcoin/chainstate
		let c = std::path::Path::new(&("/media/".to_string()+&get_user().unwrap()+"/"+&(uuid.to_string())+"/home/"+&(host_user.to_string())+"/.bitcoin/blocks")).exists();
		let d = std::path::Path::new(&("/media/".to_string()+&get_user().unwrap()+"/"+&(uuid.to_string())+"/home/"+&(host_user.to_string())+"/.bitcoin/chainstate")).exists();
		if c == false && d == false{
			let output = Command::new("sudo").args(["mkdir", "--parents", &("/media/".to_string()+&get_user().unwrap()+"/"+&(uuid.to_string())+"/home/"+&(host_user.to_string())+"/.bitcoin/blocks"), &("/media/".to_string()+&get_user().unwrap()+"/"+&(uuid.to_string())+"/home/"+&(host_user.to_string())+"/.bitcoin/chainstate") ]).output().unwrap();
			if !output.status.success() {
			return Err(format!("ERROR in removing stale ./bitcoin/chainstate dir {}", std::str::from_utf8(&output.stderr).unwrap()));
			}
		}
		//open file permissions of internal storage dotfile dirs
		//recursive open TODO add some error handling here, if this is successful it makes the rest of the chmods below deprecated
		Command::new("sudo").args(["chmod", "-R", "777", &("/media/".to_string()+&get_user().unwrap()+"/"+&(uuid.to_string())+"/home/"+&(host_user.to_string())+"/.bitcoin")]).output().unwrap();
		let output = Command::new("sudo").args(["chmod", "777", &("/media/".to_string()+&get_user().unwrap()+"/"+&(uuid.to_string())+"/home/"+&(host_user.to_string())+"/.bitcoin")]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in opening file permissions of internal storage .bitcoin dirs {}", std::str::from_utf8(&output.stderr).unwrap()));
		} 
		//open blocks dir
		let output = Command::new("sudo").args(["chmod", "777", &("/media/".to_string()+&get_user().unwrap()+"/"+&(uuid.to_string())+"/home/"+&(host_user.to_string())+"/.bitcoin/blocks")]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in opening file permissions of internal blocks storage .bitcoin dirs {}", std::str::from_utf8(&output.stderr).unwrap()));
		} 
		//open chainstate dir
		let output = Command::new("sudo").args(["chmod", "777", &("/media/".to_string()+&get_user().unwrap()+"/"+&(uuid.to_string())+"/home/"+&(host_user.to_string())+"/.bitcoin/chainstate")]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in opening file permissions of internal chainstate storage .bitcoin dirs {}", std::str::from_utf8(&output.stderr).unwrap()));
		} 
		//open file permissions for settings.tmp
		Command::new("sudo").args(["chmod", "777", &("/media/".to_string()+&get_user().unwrap()+"/"+&(uuid.to_string())+"/home/"+&(host_user.to_string())+"/.bitcoin/settings.tmp")]).output().unwrap();
		//verify the mount
		let e = std::path::Path::new(&("/media/ubuntu/".to_string()+&(uuid.to_string()))).exists();
		if e {
			Ok(format!("SUCCESS in mounting the internal drive"))
		}else{
			Err(format!("ERROR mounting internal drive, final check failed"))
		}
	//if get_uuid() returns a valid uuid assume the drive is already mounted from a previous session
	}else {
		Ok(format!("SUCCESS internal drive is already mounted"))
	}
}

//calculate time until next decay
#[tauri::command]
async fn calculate_decay_time(file: String) -> Result<String, String> {
	//retrieve start time
	let current_time_str = match retrieve_median_blocktime(){
		Ok(current_time)=> current_time,
		Err(err)=> return Err(format!("{}", err.to_string()))
	};
	let current_time: i64 = current_time_str.parse().unwrap();
	//retrieve immediate_decay
	let decay_time = retrieve_decay_time_integer(file.to_string());
	//subtract start_time from immediate decay
	let time = decay_time - current_time;
	//convert to years, months, days, hours, minutes
	let years = time / 31536000; //divide by number of seconds in a year
	let mut remainder = time % 31536000;
	let months = remainder / 2592000; //divide by number of seconds in a month
	remainder = remainder % 2592000;
	let weeks = remainder / 604800; //divide by number of seconds in a week
	remainder = remainder % 604800;
	let days = remainder / 86400; //divide by number of seconds in a day
	remainder = remainder % 86400;
	let hours = remainder / 3600; //divide by number of seconds in an hour
	remainder = remainder % 3600;
	let minutes = remainder / 60;
	remainder = remainder % 60;
	//if the decay has finished
	if years <= 0 && months <= 0 && weeks <= 0 && days <= 0 && hours <= 0 && minutes <= 0 {
		Ok(format!("decay complete"))
	}
	//if the decay has not finished
	else{
		Ok(format!("years={}, months={}, weeks={}, days={}, hours={}, minutes={}, seconds={}", years, months, weeks, days, hours, minutes, remainder))
	}
}

//used to reconstitute shards into an encryption/decryption masterkey
#[tauri::command]
async fn combine_shards() -> Result<String, String> {
	//check for stale shards list
	let shards_list = std::path::Path::new("/mnt/ramdisk/shards.txt").exists();
	if shards_list == true{
		let output = Command::new("sudo").args(["rm", "/mnt/ramdisk/shards.txt"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in combine_shards, with removing stale shards_list = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	//check for stale output produced by combine-shards.sh script
	let untrimmed = std::path::Path::new("/mnt/ramdisk/masterkey_untrimmed.txt").exists();
	if untrimmed == true{
		let output = Command::new("sudo").args(["rm", "/mnt/ramdisk/masterkey_untrimmed.txt"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in combine_shards, with removing masterkey_untrimmed = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	//check for stale combine-shards.sh script
	let combine_script = std::path::Path::new(&(get_home().unwrap()+"/arctica/combine-shards.sh")).exists();
	if combine_script == true{
		let output = Command::new("sudo").args(["rm", &(get_home().unwrap()+"/arctica/combine-shards.sh")]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in combine_shards, with removing combine-shards script = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	println!("combining shards in /mnt/ramdisk/CDROM/shards");
	let shard_dir = Path::new("/mnt/ramdisk/CDROM/shards");
	let mut shards_list = match File::create("/mnt/ramdisk/shards.txt"){
		Ok(file)=>file,
		Err(e)=>return Err(format!("ERROR creating shards.txt: {}", e.to_string()))
	};
	//iterate over directory
	let read_dir_result = fs::read_dir(shard_dir);
	if read_dir_result.is_err(){
		return Err(format!("ERROR reading the shard dir"))
	}
	//read each file from the directory
	let mut count = 0;
	for entry in read_dir_result.unwrap(){
		let entry = match entry{
			Ok(e) => e,
			Err(e)=> return Err(format!("ERROR reading shard entry: {}", e.to_string()))
		};
		let path = entry.path();

		// Process only if it's a file
		if path.is_file() {
			let file = match File::open(&path) {
				Ok(f) => f,
				Err(e) => return Err(format!("ERROR opening shard file: {}", e.to_string())),
			};
			let mut reader = BufReader::new(file);
			let mut contents = String::new();
			if reader.read_line(&mut contents).is_err() {
				continue;
			}
			if let Err(e) = writeln!(shards_list, "{}", contents.trim_end()) {
				return Err(format!("ERROR writing shard to shard list: {}", e.to_string()));
			}
			// Increment the count and check if we've processed 5 files
			count += 1;
			if count >= 5 {
				break;
			}
		}
	}
	//create combine-shards.sh script
	let file = File::create(&(get_home().unwrap()+"/arctica/combine-shards.sh")).unwrap();
	//populate combine-shards.sh with bash
	let output = Command::new("echo").args(["-e", 
	"#combine 5 key shards inside of shards.txt to reconstitute masterkey\n
	ssss-combine -t 5 < /mnt/ramdisk/shards.txt 2> /mnt/ramdisk/masterkey_untrimmed.txt"])
	.stdout(file).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR with creating combine-shards.sh: {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//execute the combine-shards bash script
	let output = Command::new("bash")
		.args([get_home().unwrap()+"/arctica/combine-shards.sh"])
		.output()
		.expect("failed to execute process");
	let content = match fs::read_to_string("/mnt/ramdisk/masterkey_untrimmed.txt"){
		Ok(res)=>res,
		Err(e)=>return Err(format!("ERROR reading masterkey_untrimmed to string: {}", e.to_string()))
	};
	let trimmed = content.trim_start_matches("Resulting secret:").trim();
	let mut file = match fs::File::create("/mnt/ramdisk/CDROM/masterkey"){
		Ok(res)=>res,
		Err(e)=>return Err(format!("ERROR trimming masterkey: {}", e.to_string()))
	};
	match write!(file, "{}", trimmed){
		Ok(_)=>return Ok(format!("Success combining shards into a Masterkey")),
		Err(e)=>return Err(format!("ERROR writing masterkey to file: {}", e.to_string()))
	}
}

//updates config file params
#[tauri::command]
async fn async_write(name: &str, value: &str) -> Result<String, String> {
    write(name.to_string(), value.to_string());
    println!("{}", name);
	//Flush the Filesystem Buffers
	Command::new("sync").output().unwrap();
    Ok(format!("completed with no problems"))
}

//check the currently inserted disc for an encryption/decryption masterkey 
//note that this is checking the CDROM path in ramdisk, not the actual cd mount path, copy_cd_to_ramdisk must be run first
#[tauri::command]
async fn check_for_masterkey() -> bool {
    let a = std::path::Path::new("/mnt/ramdisk/CDROM/masterkey").exists();
    if a {
        return true
    }
	else{
        return false
    }
}

//used to check for a config.txt in $HOME, if it exists we can reasonably assume the user is a returning user
#[tauri::command]
async fn check_for_config() -> bool {
    let a = std::path::Path::new(&(get_home().unwrap()+"/config.txt")).exists();
    if a {
        return true
    }
	else{
        return false
    }
}

#[tauri::command]
//used to store decryption shards gathered from various Hardware Wallets to eventually be reconstituted into a masterkey when attempting to log in manually
async fn recovery_initiate() -> Result<String, String> {
	//create the CDROM dir if it does not already exist
	let a = std::path::Path::new("/mnt/ramdisk/CDROM").exists();
	if a == false{
		let output = Command::new("mkdir").args(["/mnt/ramdisk/CDROM"]).output().unwrap();
		if !output.status.success() {
		return Err(format!("ERROR in creating recovery CD, with making CDROM dir = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	}
	//create recoveryCD config, this informs the front end on BOOT whether or not the user is attempting to manually recover login or attempting to sign a PSBT
	let file = File::create("/mnt/ramdisk/CDROM/config.txt").unwrap();
	let output = Command::new("echo").args(["type=recoverycd" ]).stdout(file).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in creating recovery CD, with creating config = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//collect shards from Hardware Wallets for export to transfer CD
	let output = Command::new("cp").args(["-R", &(get_home().unwrap()+"/shards"), "/mnt/ramdisk/CDROM"]).output().unwrap();
	if !output.status.success() {
    	return Err(format!("ERROR in creating recovery CD with copying shards from HW = {}", std::str::from_utf8(&output.stderr).unwrap()));
    }
	//create iso from transferCD dir
	let output = Command::new("genisoimage").args(["-r", "-J", "-o", "/mnt/ramdisk/transferCD.iso", "/mnt/ramdisk/CDROM"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR creating recovery CD with creating ISO = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//obtain CD path
	let path = match get_cd_path(){
		Ok(path) => path,
        Err(er) => {
        	return Err(format!("{}", er))
        }
	};
	//wipe the CD 
	Command::new("sudo").args(["umount", &path]).output().unwrap();
	let output = Command::new("sudo").args(["wodim", "-v", &("dev=".to_string()+&path), "blank=fast"]).output().unwrap();
	if !output.status.success() {
		//attempt alternative wipe method
		Command::new("sudo").args(["wodim", "-v", &("dev=".to_string()+&path), "blank=fast"]).output().unwrap();
	}
	//burn transferCD iso to the transfer CD
	let output = Command::new("sudo").args(["wodim", &("dev=".to_string()+&path), "-v", "-data", "/mnt/ramdisk/transferCD.iso"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR converting to transfer CD with burning ISO to CD = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//eject the disc
	match eject_disc(){
		Ok(res) => res,
		Err(e) => return Err(format!("ERROR in ejecting CD = {}", e))
	};
	Ok(format!("SUCCESS in creating recovery CD"))
}

//calculate the number of encryption shards currently in the ramdisk
#[tauri::command]
async fn calculate_number_of_shards() -> u32 {
	let mut x = 0;
    for _file in fs::read_dir("/mnt/ramdisk/CDROM/shards").unwrap() {
		x = x + 1;
	}
	return x;
}

//collect shards from the recovery CD
#[tauri::command]
async fn collect_shards() -> Result<String, String> {
	println!("collecting shards");
	//obtain a list of all of the filenames in $HOME/shards
	let shards = Command::new(&("ls")).args([&(get_home().unwrap()+"/shards")]).output().unwrap();
	if !shards.status.success() {
	return Err(format!("ERROR in collect_shards() with parsing $HOME/shards = {}", std::str::from_utf8(&shards.stderr).unwrap()));
	} 
	//convert the list of shards into a vector of results
	let shards_output = std::str::from_utf8(&shards.stdout).unwrap();
	let split = shards_output.split('\n');
	let shards_vec: Vec<_> = split.collect();
	//iterate through the vector and copy each file to /mnt/ramdisk/CDROM/shards
	for i in &shards_vec{
		if i == &""{
			continue
		}else{
			let output = Command::new("cp").args([&(get_home().unwrap()+"/shards/"+&(i.to_string())), "/mnt/ramdisk/CDROM/shards"]).output().unwrap();
			if !output.status.success() {
				return Err(format!("Error in collect_shards() with copying shard {} = {}, Shards-vec: {:?}", i.to_string(), std::str::from_utf8(&output.stderr).unwrap(), &shards_vec))
			}
		}
	} 
	Ok(format!("SUCCESS in collecting shards"))
}

#[tauri::command]
//convert the completed recovery CD to a Transfer CD via config file
async fn convert_to_transfer_cd() -> Result<String, String> {
	//remove stale config
	let output = Command::new("sudo").args(["rm", "/mnt/ramdisk/CDROM/config.txt"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("Error in convert to transfer CD with removing stale config = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//create transferCD config
	let file = File::create("/mnt/ramdisk/CDROM/config.txt").unwrap();
	let output = Command::new("echo").args(["type=transfercd" ]).stdout(file).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in converting to transfer CD, with creating config = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	Ok(format!("SUCCESS in converting config to transfer CD"))
}

//used to display a QR encoded receive address with eog
#[tauri::command]
async fn display_qr() -> Result<String, String>{
	let output = Command::new("eog").args(["--disable-gallery", "--new-instance", "/mnt/ramdisk/qrcode.png"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in displaying QR code with EOG = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	Ok(format!("successfully displayed QR code"))
}

//TODO can potentially remove the external script used below with a bash -c command
// #[tauri::command]
// async fn enable_webcam_qr_scan() -> Result<String, String> {
//     let output = Command::new("zbarcam").output().unwrap();
// 	let output_str = std::str::from_utf8(&output.stdout).unwrap().trim();    
// 	let qr_data: Vec<&str> = output_str.split(':').collect();    
// 	if qr_data.len() > 1 {
//       return Ok(format!("{}", qr_data[1]))
//     }else{
//         return Err(format!("ERROR reading QR code"))
//     }
// }


//enable webcam with zbarcam and force stop zbarcam after it receives a valid string
#[tauri::command]
async fn enable_webcam_qr_scan() -> Result<String, String> {
	let script = std::path::Path::new(&(get_home().unwrap()+"/arctica/enable-webcam-scan.sh")).exists();
	if script == false{
		//create enable-webcam-scan.sh script
		let file = File::create(&(get_home().unwrap()+"/arctica/enable-webcam-scan.sh")).unwrap();
		//populate enable-webcam-scan.sh with bash
		let output = Command::new("echo").args(["-e", "zbarcam | head -n 1"]).stdout(file).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR with creating enable-webcam-scan.sh: {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	//run enable webcam scan bash script, pipes zbarcam output to head
	let output = Command::new("bash").args([&(get_home().unwrap()+"/arctica/enable-webcam-scan.sh")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in running enable-webcam-scan.sh {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	//split the result and return the address if it's greater than 1 character
	let output_str = std::str::from_utf8(&output.stdout).unwrap().trim();
    let qr_data: Vec<&str> = output_str.split(':').collect();
	if qr_data.len() == 1 {
		Ok(format!("{}", qr_data[0]))
	}
    else if let Some(pos) = qr_data.iter().position(|&s| s == "bitcoin"){
		if pos + 1 < qr_data.len(){
			Ok(format!("{}", qr_data[pos + 1]))
		}
        else{
			Err(format!("ERROR, no data after \'bitcoin:\' in payload"))
		}
    } else {
        Err(format!("ERROR reading QR code"))
    }
}

//copy a receive address to a users clipboard
#[tauri::command]
async fn copy_to_clipboard(address: String) -> Result<String, String>{
	let filepath = "/mnt/ramdisk/address";
	fs::write(&filepath, address);
	//use xclip to select the receive address
	let output = Command::new("xclip").args(["-selection", "clipboard", &filepath]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in copying address to clipboard = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//remove the stale receive address to prevent reuse
	Command::new("sudo").args(["rm", "/mnt/ramdisk/address"]).output().unwrap();
	Ok(format!("successfully copied to clipboard"))
}

//enables networking capabilities
#[tauri::command]
async fn enable_networking() -> Result<String, String>{
	//ping linux servers once to check for connectivity
	let output = Command::new("ping").args(["-c", "1", "linux.org"]).output().unwrap();
	if output.status.success() {
		return Ok(format!("SUCCESS networking is already enabled"))
	}
	//enable networking if ping failed
	let output = Command::new("sudo").args(["nmcli", "networking", "on"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR disabling networking = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//open wifi settings panel to prompt the user to connect
	Command::new("gnome-control-center").output().unwrap();
	Ok(format!("SUCCESS enabled networking"))
}

//clear psbt from ramdisk, used when extending the current session after signing a psbt
#[tauri::command]
async fn clear_psbt() -> Result<String, String>{
	let output = Command::new("sudo").args(["rm", "-r", "/mnt/ramdisk/psbt"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in clearing psbt {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
    Ok(format!("success clearing psbt"))
}

//export the contents of backup CD to the staged writable dir
#[tauri::command]
async fn export_backup() -> Result<String, String>{
	//verify that writable exists, if not throw err
	let a = std::path::Path::new(&("/media/".to_string()+&get_user().unwrap()+"/writable/")).exists();
	if a == false{
		return Err(format!("ERROR in export backup, persistent dir not found at /media/$USER/writable"))
	}
	//cp shards dir to writable/upper/home/$USER
	let output = Command::new("sudo").args(["cp", "-r", &("/media/".to_string()+&get_user().unwrap()+"/CDROM/shards"), &("/media/".to_string()+&get_user().unwrap()+"/writable/upper/home/ubuntu")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in exporting backup with copying shards dir {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	//cp encrypted.gpg to writable/upper/home/$USER
	let output = Command::new("sudo").args(["cp", &("/media/".to_string()+&get_user().unwrap()+"/CDROM/encrypted.gpg"), &("/media/".to_string()+&get_user().unwrap()+"/writable/upper/home/ubuntu")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in exporting backup with copying encrypted.gpg {}", std::str::from_utf8(&output.stderr).unwrap()));
		}

    Ok(format!("success in exporting backup"))
}

//return a stringified list of all of the target devices on the local machine, mount point, and storage capacity
#[tauri::command]
async fn get_device_info(root: String) -> Result<String, String> {
	//sleep for hasty user
	Command::new("sleep").args(["3"]).output().unwrap();
	//if password is not blank...
	if root.len() > 0{
	//query local devices by piping in the password
		let mut sudo = Command::new("sudo")
		.arg("-S").args(["fdisk", "-l"])
		.stdin(Stdio::piped()) //pipe password
		.stdout(Stdio::piped()) //capture stdout
		.spawn()
		.unwrap();
	
		//pipe in root password
		sudo.stdin.as_mut().unwrap().write_all(root.as_bytes());
		//sleep for piping to complete
		Command::new("sleep").args(["2"]).output().unwrap();
		let output = sudo.wait_with_output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in get_device_info with querying fdisk & piping root = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	//sort the fdisk result into something clean for the front end
	let output_str = String::from_utf8_lossy(&output.stdout);
    let mut raw_data = String::new();
    let mut lines = output_str.lines().peekable();    
	while let Some(line) = lines.next() {
        if line.starts_with("Disk /dev/") && !line.contains("/dev/loop") {
            let path_parts: Vec<&str> = line.split_whitespace().collect();
            let path = path_parts[1].trim_end_matches(':');            
			if let Some(model_line) = lines.peek() {
                if model_line.starts_with("Disk model:") {
                    let model_parts: Vec<&str> = model_line.split(":").collect();
                    let model = model_parts[1].trim();                    
					let size = path_parts[2];
                    let unit = path_parts[3];                    
					raw_data.push_str(&format!("Mount: {} - Model: {} - Size: {} {}\n", path, model, size, unit));
                    lines.next();  // Consume the model line as it has been processed
                }
            }
        }
    }
	//trim trailing comma
	let devices: Vec<String> = raw_data
	.split(',')
	.filter(|&item| item.len() >= 2)
	.map(String::from)
	.collect();
	//return the result
	let result = devices.join(",");
    Ok(result)
	}else{ //handle a blank password
	//query local devices
    let output = Command::new("sudo")
        .arg("fdisk")					
        .arg("-l")
        .output()
        .unwrap();    
		if !output.status.success() {
			return Err(format!("ERROR in querying fdisk = {}", std::str::from_utf8(&output.stderr).unwrap()));
    	}   
	//sort the fdisk result into something clean for the front end
	let output_str = String::from_utf8_lossy(&output.stdout);
    let mut raw_data = String::new();
    let mut lines = output_str.lines().peekable();    
	while let Some(line) = lines.next() {
        if line.starts_with("Disk /dev/") && !line.contains("/dev/loop") {
            let path_parts: Vec<&str> = line.split_whitespace().collect();
            let path = path_parts[1].trim_end_matches(':');            
			if let Some(model_line) = lines.peek() {
                if model_line.starts_with("Disk model:") {
                    let model_parts: Vec<&str> = model_line.split(":").collect();
                    let model = model_parts[1].trim();                    
					let size = path_parts[2];
                    let unit = path_parts[3];      
					if unit.contains("TiB"){
						lines.next();  // Consume the model line as it has been processed
					}else{
						raw_data.push_str(&format!("Mount: {} - Model: {} - Size: {} {}\n", path, model, size, unit));
						lines.next();  // Consume the model line as it has been processed
					}           
                }
            }
        }
    }
	//trim trailing comma
	let devices: Vec<String> = raw_data
	.split(',')
	.filter(|&item| item.len() >= 2)
	.map(String::from)
	.collect();
	//return the result
	let result = devices.join(",");
    Ok(result)
	} 

}

//gets a baseline device list
#[tauri::command]
async fn get_baseline() -> Result<String, String>{
let query = match run_fdisk(){
	Ok(result) => result,
	Err(e) => 
	return Err(e.to_string())
};
let output = parse_fdisk_result(&query);
Ok(format!("{:?}", &output))
}

//ping the linux servers to check for network connectivity
#[tauri::command]
async fn check_network() -> Result<String, String>{
	//ping linux servers once to check for connectivity
	let output = Command::new("ping").args(["-c", "1", "linux.org"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR ping failed networking is not enabled"))
	}
	Ok(format!("SUCCESS network connection enabled"))
}


fn main() {
  	tauri::Builder::default()
	//export tauri commands to be called on the front end
  	.manage(TauriState(Mutex::new(None))) 
  	.invoke_handler(tauri::generate_handler![
        test_function,
		pre_install,
        create_bootable_usb,
        create_setup_cd,
        read_cd,
        copy_cd_to_ramdisk,
		eject_cd,
        init_iso,
        async_write,
        read,
        combine_shards,
        mount_internal,
        create_ramdisk,
        packup,
        unpack,
        install_cold_deps,
		install_warm_deps,
        refresh_cd,
		calculate_decay_time,
        distribute_shards_hw2,
        distribute_shards_hw3,
        distribute_shards_hw4,
        distribute_shards_hw5,
        distribute_shards_hw6,
        distribute_shards_hw7,
    	create_descriptor,
        create_backup,
        make_backup,
		init_bitcoind,
        start_bitcoind,
		check_bitcoin_sync_status,
		stop_bitcoind,
        check_for_masterkey,
		check_for_config,
        recovery_initiate,
        calculate_number_of_shards,
        collect_shards,
        convert_to_transfer_cd,
		generate_store_key_pair,
		generate_store_simulated_time_machine_key_pair,
		load_wallet,
		get_address,
		get_balance,
	    get_transactions,
		get_descriptor_info,
		get_blockchain_info,
		generate_psbt,
		export_psbt,
		sign_processed_psbt,
		sign_funded_psbt,
		broadcast_tx,
		decode_processed_psbt,
		decode_funded_psbt,
		display_qr,
		copy_to_clipboard,
		retrieve_median_blocktime,
		enable_networking, 
		enable_webcam_qr_scan,
		clear_psbt,
		export_backup,
		get_device_info,
		check_network,
		get_baseline,
        ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
