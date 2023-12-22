use std::process::Command;
use std::fs;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;

//import helper.rs module
use crate::helper::{
    get_user, get_home, write, generate_keypair, store_string, is_dir_empty,
	get_cd_path, eject_disc,

};

//import bitcoin_wallet.rs module
use crate::bitcoin_wallet::{
    create_wallet, import_descriptor, build_high_descriptor, build_med_descriptor,
	build_low_descriptor,
};

#[tauri::command]
//generates a public and private key pair and stores them as a text file
pub async fn generate_store_key_pair(number: String) -> Result<String, String> {
	//number corresponds to currentHW here and is provided by the front end
	let private_key_file = "/mnt/ramdisk/sensitive/private_key".to_string()+&number;
	let public_key_file = "/mnt/ramdisk/sensitive/public_key".to_string()+&number;
	let private_change_key_file = "/mnt/ramdisk/sensitive/private_change_key".to_string()+&number;
	let public_change_key_file = "/mnt/ramdisk/sensitive/public_change_key".to_string()+&number;
    //generate an extended private and public keypair
    let (xpriv, xpub) = match generate_keypair() {
		Ok((xpriv, xpub)) => (xpriv, xpub),
		Err(err) => return Err("ERROR could not generate keypair: ".to_string()+&err.to_string())
	}; 
	//note that change xkeys and standard xkeys are the same but simply given different derviation paths, they are stored seperately for ease of use
	//change keys are assigned /1/* and external keys are assigned /0/*
    //store the xpriv as a file
	match store_string(xpriv.to_string()+"/0/*", &private_key_file) {
		Ok(_) => {},
		Err(err) => return Err("ERROR could not store private key: ".to_string()+&err)
	}
    //store the xpub as a file
	match store_string(xpub.to_string()+"/0/*", &public_key_file) {
		Ok(_) => {},
		Err(err) => return Err("ERROR could not store public key: ".to_string()+&err)
	}
	//store the change_xpriv as a file
	match store_string(xpriv.to_string()+"/1/*", &private_change_key_file) {
		Ok(_) => {},
		Err(err) => return Err("ERROR could not store private change key: ".to_string()+&err)
	}
	//store the change_xpub as a file
	match store_string(xpub.to_string()+"/1/*", &public_change_key_file) {
		Ok(_) => {},
		Err(err) => return Err("ERROR could not store public change key: ".to_string()+&err)
	}
	//make the pubkey dir in the setupCD staging area if it does not already exist
	let a = std::path::Path::new("/mnt/ramdisk/CDROM/pubkeys").exists();
    if a == false{
		let output = Command::new("mkdir").args(["--parents", "/mnt/ramdisk/CDROM/pubkeys"]).output().unwrap();
		if !output.status.success() {
		return Err(format!("ERROR in creating /mnt/ramdisk/CDROM/pubkeys dir {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	//copy public key to setupCD dir
	let output = Command::new("cp").args([&("/mnt/ramdisk/sensitive/public_key".to_string()+&number), "/mnt/ramdisk/CDROM/pubkeys"]).output().unwrap();
	if !output.status.success() {
    	return Err(format!("ERROR in generate store key pair with copying pub key= {}", std::str::from_utf8(&output.stderr).unwrap()));
    }
	//copy public change key to setupCD dir
	let output = Command::new("cp").args([&("/mnt/ramdisk/sensitive/public_change_key".to_string()+&number), "/mnt/ramdisk/CDROM/pubkeys"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in generate store key pair with copying pub change key= {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	Ok(format!("SUCCESS generated and stored Private and Public Key Pair"))
}

//this function simulates the creation of a time machine key. Eventually this creation will be performed by the BPS and 
//the pubkeys will be shared with the user instead. 4 Time machine Keys are needed so this function will be run 4 times in total.
//eventually these will need to be turned into descriptors and we will need an encryption scheme for the descriptors/keys that will be held by the BPS so as not to be privacy leaks
//decryption key will be held within encrypted tarball on each Hardware Wallet
#[tauri::command]
pub async fn generate_store_simulated_time_machine_key_pair(number: String) -> Result<String, String> {
	//make the time machine key dir in the setupCD staging area if it does not already exist
	let a = std::path::Path::new("/mnt/ramdisk/CDROM/timemachinekeys").exists();
    if a == false{
		let output = Command::new("mkdir").args(["--parents", "/mnt/ramdisk/CDROM/timemachinekeys"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in creating /mnt/ramdisk/CDROM/timemachinekeys dir {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	//TODO NOTE THAT THESE KEYS ARE STORED ALL OVER THE PLACE, fine for now but they will need to be properly stored once BPS is integrated
	//number param is provided by the front end
	let private_key_file = "/mnt/ramdisk/CDROM/timemachinekeys/time_machine_private_key".to_string()+&number;
	let public_key_file = "/mnt/ramdisk/CDROM/timemachinekeys/time_machine_public_key".to_string()+&number;
	let private_change_key_file = "/mnt/ramdisk/CDROM/timemachinekeys/time_machine_private_change_key".to_string()+&number;
	let public_change_key_file = "/mnt/ramdisk/CDROM/timemachinekeys/time_machine_public_change_key".to_string()+&number;
	let (xpriv, xpub) = match generate_keypair() {
		Ok((xpriv, xpub)) => (xpriv, xpub),
		Err(err) => return Err("ERROR could not generate keypair: ".to_string()+&err.to_string())
	};
	//note that change xkeys and standard xkeys are the same but simply given different derviation paths, they are stored seperately for ease of use
	//change keys are assigned /1/* and external keys are assigned /0/*
    //store the xpriv as a file
	match store_string(xpriv.to_string()+"/0/*", &private_key_file) {
		Ok(_) => {},
		Err(err) => return Err("ERROR could not store private key: ".to_string()+&err)
	}
    //store the xpub as a file
	match store_string(xpub.to_string()+"/0/*", &public_key_file) {
		Ok(_) => {},
		Err(err) => return Err("ERROR could not store public key: ".to_string()+&err)
	}
	//store the change_xpriv as a file
	match store_string(xpriv.to_string()+"/1/*", &private_change_key_file) {
		Ok(_) => {},
		Err(err) => return Err("ERROR could not store private change key: ".to_string()+&err)
	}
	//store the change_xpub as a file
	match store_string(xpub.to_string()+"/1/*", &public_change_key_file) {
		Ok(_) => {},
		Err(err) => return Err("ERROR could not store public change key: ".to_string()+&err)
	}
	//copy public key to setupCD dir
	let output = Command::new("cp").args([&("/mnt/ramdisk/CDROM/timemachinekeys/time_machine_public_key".to_string()+&number), "/mnt/ramdisk/CDROM/pubkeys"]).output().unwrap();
	if !output.status.success() {
    	return Err(format!("ERROR in generate store key pair with copying pub key to CDROM= {}", std::str::from_utf8(&output.stderr).unwrap()));
    }
	//copy public change key to setupCD dir
	let output = Command::new("cp").args([&("/mnt/ramdisk/CDROM/timemachinekeys/time_machine_public_change_key".to_string()+&number), "/mnt/ramdisk/CDROM/pubkeys"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in generate store key pair with copying pub change key to CDROM= {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//copy public key to sensitive dir
	let output = Command::new("cp").args([&("/mnt/ramdisk/CDROM/timemachinekeys/time_machine_public_key".to_string()+&number), "/mnt/ramdisk/sensitive"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in generate store key pair with copying pub key to sensitive = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//copy public change key to sensitive dir
	let output = Command::new("cp").args([&("/mnt/ramdisk/CDROM/timemachinekeys/time_machine_public_change_key".to_string()+&number), "/mnt/ramdisk/sensitive"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in generate store key pair with copying pub change key to sensitive= {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//copy private key to sensitive dir
	let output = Command::new("cp").args([&("/mnt/ramdisk/CDROM/timemachinekeys/time_machine_private_key".to_string()+&number), "/mnt/ramdisk/sensitive"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in generate store key pair with copying private key to sensitive = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//copy private change key to sensitive dir
	let output = Command::new("cp").args([&("/mnt/ramdisk/CDROM/timemachinekeys/time_machine_private_change_key".to_string()+&number), "/mnt/ramdisk/sensitive"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in generate store key pair with copying private change key to sensitive = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	Ok(format!("SUCCESS generated and stored Private and Public Time Machine Key Pair"))
}

//create arctica descriptors
//High Descriptor is the time locked 5 of 11 with decay (the 4 keys in the 2 of 4 timelock will be held by BPS)
//Medium Descriptor is the 2 of 7 with decay
//Low Descriptor is the 1 of 7 and may be removed
//acceptable params are "1", "2", "3", "4", "5", "6", "7"
#[tauri::command]
pub async fn create_descriptor(hwnumber: String) -> Result<String, String> {
	// sleep for 6 seconds
	Command::new("sleep").args(["6"]).output().unwrap();
   println!("creating descriptors from 7 xpubs & 4 time machine keys");
   println!("creating key array");
    //convert all 11 xpubs in ramdisk to an array vector
   let mut key_array = Vec::new();
   let mut change_key_array = Vec::new();
   //push the first 7 xpubs into the key_array vector 
   println!("pushing 7 standard pubkeys into key array");
   for i in 1..=7{
       let key = match fs::read_to_string(&("/mnt/ramdisk/CDROM/pubkeys/public_key".to_string()+&(i.to_string()))){
        Ok(key)=> key,
        Err(err)=> return Err(format!("{}", err.to_string()))
    };
       key_array.push(key);
       println!("pushed key");
   }
   //push the 4 time machine public keys into the key_array vector, (only for HW 1).
	println!("pushing 4 time machine pubkeys into key array");
	for i in 1..=4{
		let key = match fs::read_to_string(&("/mnt/ramdisk/CDROM/pubkeys/time_machine_public_key".to_string()+&(i.to_string()))){
			Ok(key)=> key,
			Err(err)=> return Err(format!("{}", err.to_string()))
		};
		key_array.push(key);
		println!("pushed key");
	}
	println!("printing key array");
	println!("{:?}", key_array);

	//push the 7 public change keys into the change_key_array vector
	println!("pushing 7 pub change keys into change key array");
	for i in 1..=7{
		let key = match fs::read_to_string(&("/mnt/ramdisk/CDROM/pubkeys/public_change_key".to_string()+&(i.to_string()))){
			Ok(key)=> key,
			Err(err)=> return Err(format!("{}", err.to_string()))
		};
		change_key_array.push(key);
		println!("pushed key");
	}
	  //push the 4 time machine public keys into the key_array vector, (only for HW 1).
	   println!("pushing 4 time machine pub change keys into change key array");
	   for i in 1..=4{
		   let key = match fs::read_to_string(&("/mnt/ramdisk/CDROM/pubkeys/time_machine_public_change_key".to_string()+&(i.to_string()))){
			   Ok(key)=> key,
			   Err(err)=> return Err(format!("{}", err.to_string()))
		   };
		   change_key_array.push(key);
		   println!("pushed key");
	   }
	println!("printing change key array");
   	println!("{:?}", change_key_array);
   //create the descriptors directory inside of ramdisk
   println!("Making descriptors dir");
   Command::new("mkdir").args(["/mnt/ramdisk/sensitive/descriptors"]).output().unwrap();
   //build the delayed wallet descriptor
   println!("building high descriptor");
   let high_descriptor = match build_high_descriptor(&key_array, &hwnumber, false) {
	Ok(desc) => desc,
	Err(err) => return Err("ERROR could not build High Descriptor ".to_string()+&err)
   };
   //verify that the descriptor creation output did not fail
   if high_descriptor.contains("No such file or directory") {
		return Err("ERROR could not build High Descriptor".to_string())
   }
   //store the delayed wallet descriptor in the sensitive dir
   let high_file_dest = &("/mnt/ramdisk/sensitive/descriptors/delayed_descriptor".to_string()+&hwnumber.to_string()).to_string();
   println!("storing high descriptor");
   match store_string(high_descriptor.to_string(), high_file_dest) {
       Ok(_) => {},
       Err(err) => return Err("ERROR could not store High Descriptor: ".to_string()+&err)
   };
   //build delayed wallet change descriptor
   println!("building high change descriptor");
   let high_change_descriptor = match build_high_descriptor(&change_key_array, &hwnumber, true) {
	Ok(desc) => desc,
	Err(err) => return Err("ERROR could not build High Change Descriptor ".to_string()+&err)
   };
   let high_change_file_dest = &("/mnt/ramdisk/sensitive/descriptors/delayed_change_descriptor".to_string()+&hwnumber.to_string()).to_string();
   //store the delayed wallet change descriptor in the sensitive dir
   println!("storing high change descriptor");
   match store_string(high_change_descriptor.to_string(), high_change_file_dest) {
       Ok(_) => {},
       Err(err) => return Err("ERROR could not store High Change Descriptor: ".to_string()+&err)
   };
   //create the delayed wallet
   println!("creating delayed wallet");
   match create_wallet("delayed".to_string(), &hwnumber){
	Ok(_) => {},
	Err(err) => return Err("ERROR could not create Delayed Wallet: ".to_string()+&err)
   };
   //import the delayed wallet descriptor
   println!("importing delayed descriptor");
   match import_descriptor("delayed".to_string(), &hwnumber, false){
	Ok(_) => {},
	Err(err) => return Err("ERROR could not import Delayed Descriptor: ".to_string()+&err)
   };
	//import delayed change descriptor
	println!("importing delayed change descriptor");
	match import_descriptor("delayed".to_string(), &hwnumber, true){
	Ok(_) => {},
	Err(err) => return Err("ERROR could not import Delayed change Descriptor: ".to_string()+&err)
	};

   //build the immediate wallet descriptor
   println!("building med descriptor");
   let med_descriptor = match build_med_descriptor(&key_array, &hwnumber, false) {	
	Ok(desc) => desc,
	Err(err) => return Err("ERROR could not build Immediate Descriptor ".to_string()+&err)
   };
   //verify that the descriptor creation output did not fail
   if med_descriptor.contains("No such file or directory") {
		return Err("ERROR could not build Med Descriptor".to_string())
	}
   //store the immediate wallet descriptor in the sensitive dir
   let med_file_dest = &("/mnt/ramdisk/sensitive/descriptors/immediate_descriptor".to_string()+&hwnumber.to_string()).to_string();
   println!("storing med descriptor");
   match store_string(med_descriptor.to_string(), med_file_dest) {
       Ok(_) => {},
       Err(err) => return Err("ERROR could not store Immediate Descriptor: ".to_string()+&err)
   };
   //build the immediate change descriptor
   println!("building med change descriptor");
   let med_change_descriptor = match build_med_descriptor(&change_key_array, &hwnumber, true) {
	Ok(desc) => desc,
	Err(err) => return Err("ERROR could not build Immediate Change Descriptor ".to_string()+&err)
   };
   let med_change_file_dest = &("/mnt/ramdisk/sensitive/descriptors/immediate_change_descriptor".to_string()+&hwnumber.to_string()).to_string();
   //store the immediate change descriptor
   println!("storing med change descriptor");
   match store_string(med_change_descriptor.to_string(), med_change_file_dest) {
       Ok(_) => {},
       Err(err) => return Err("ERROR could not store Immediate Change Descriptor: ".to_string()+&err)
   };
   //create the immediate wallet
   println!("creating immediate wallet");
   match create_wallet("immediate".to_string(), &hwnumber){
	Ok(_) => {},
	Err(err) => return Err("ERROR could not create Immediate Wallet: ".to_string()+&err)
   };
   //import the immediate wallet descriptor
   println!("importing immediate descriptor");
   match import_descriptor("immediate".to_string(), &hwnumber, false){
	Ok(_) => {},
	Err(err) => return Err(format!("ERROR could not import Immediate Descriptor: {}", err))
   };
	//import immediate change descriptor
	println!("importing immediate change descriptor");
	match import_descriptor("immediate".to_string(), &hwnumber, true){
	Ok(_) => {},
	Err(err) => return Err("ERROR could not import Immediate change Descriptor: ".to_string()+&err)
	};

//TODO POTENTIALLY SCRAP THIS 1 OF 7 WALLET
//    //build the low security descriptor
//    println!("building low descriptor");
//    let low_descriptor = match build_low_descriptor(&key_array, &hwnumber, false) {
// 	Ok(desc) => desc,
// 	Err(err) => return Err("ERROR could not build Low Descriptor ".to_string()+&err)
//    };
//    let low_file_dest = &("/mnt/ramdisk/sensitive/descriptors/low_descriptor".to_string()+&hwnumber.to_string()).to_string();
//    //store the low security descriptor in the sensitive dir
//    println!("storing low descriptor");
//    match store_string(low_descriptor.to_string(), low_file_dest) {
//        Ok(_) => {},
//        Err(err) => return Err("ERROR could not store Low Descriptor: ".to_string()+&err)
//    };

//    //build the low change descriptor
//    println!("building low change descriptor");
//    let low_change_descriptor = match build_low_descriptor(&change_key_array, &hwnumber, true) {
// 	Ok(desc) => desc,
// 	Err(err) => return Err("ERROR could not build Low Change Descriptor ".to_string()+&err)
//    };
//    let low_change_file_dest = &("/mnt/ramdisk/sensitive/descriptors/low_change_descriptor".to_string()+&hwnumber.to_string()).to_string();
//    //TODO store the low change descriptor
//    println!("storing low change descriptor");
//    match store_string(low_change_descriptor.to_string(), low_change_file_dest) {
//        Ok(_) => {},
//        Err(err) => return Err("ERROR could not store Low Change Descriptor: ".to_string()+&err)
//    };
//    //creating low wallet
//    println!("creating low wallet");
//    match create_wallet("low".to_string(), &hwnumber){
// 	Ok(_) => {},
// 	Err(err) => return Err("ERROR could not create Low Wallet: ".to_string()+&err)
//    };
//    //importing low descriptor
//    println!("importing low descriptor");
//    match import_descriptor("low".to_string(), &hwnumber, false){
// 	Ok(_) => {},
// 	Err(err) => return Err("ERROR could not import Low Descriptor: ".to_string()+&err)
//    };
//    //import low change descriptor
//    println!("importing low change descriptor");
//    match import_descriptor("low".to_string(), &hwnumber, true){
// 	Ok(_) => {},
// 	Err(err) => return Err("ERROR could not import Low change Descriptor: ".to_string()+&err)
//    };

   println!("Success");
   Ok(format!("SUCCESS in creating descriptors"))
}

//function to creates the setupCD which is used to pass state between sessions during setup
#[tauri::command]
pub async fn create_setup_cd() -> Result<String, String> {
	//obtain CD path
	let path: String = match get_cd_path(){
		Ok(path) => path,
        Err(er) => {
        	return Err(format!("{}", er))
        }
	};
	println!("creating setup CD");
	//create local shards dir
	Command::new("mkdir").args([&(get_home().unwrap()+"/shards")]).output().unwrap();
	//install HW dependencies for genisoimage
	let output = Command::new("sudo").args(["apt", "install", &(get_home().unwrap()+"/dependencies/genisoimage_9%3a1.1.11-3.2ubuntu1_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing genisoimage for create_setup_cd {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	//install HW dependencies for ssss
	let output = Command::new("sudo").args(["apt", "install", &(get_home().unwrap()+"/dependencies/ssss_0.5-5_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing ssss for create_setup_cd {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	//install HW dependencies for wodim
	let output = Command::new("sudo").args(["apt", "install", &(get_home().unwrap()+"/dependencies/wodim_9%3a1.1.11-3.2ubuntu1_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing wodim for create_setup_cd {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	//install library for qrencode
	let output = Command::new("sudo").args(["apt", "install", &(get_home().unwrap()+"/dependencies/libqrencode4_4.1.1-1_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing libqrencode for create_setup_cd {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	//install HW dependencies for qrencode
	let output = Command::new("sudo").args(["apt", "install", &(get_home().unwrap()+"/dependencies/qrencode_4.1.1-1_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing qrencode for create_setup_cd {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	//create setupCD config
	let file = File::create("/mnt/ramdisk/CDROM/config.txt").unwrap();
	Command::new("echo").args(["type=setupcd" ]).stdout(file).output().unwrap();
	//create masterkey and derive shards
	let output = Command::new("bash").args([&(get_home().unwrap()+"/scripts/create-setup-cd.sh")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in running create-setup-cd.sh {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	//TODO: EVENTUALLY THE APPROPRIATE SHARDS NEED TO GO TO THE BPS HERE
	//copy first 2 shards to HW 1
	let output = Command::new("sudo").args(["cp", "/mnt/ramdisk/shards/shard1.txt", &(get_home().unwrap()+"/shards")]).output().unwrap();
	if !output.status.success() {
    	return Err(format!("ERROR in copying shard1.txt in create setup CD = {}", std::str::from_utf8(&output.stderr).unwrap()));
    }
	let output = Command::new("sudo").args(["cp", "/mnt/ramdisk/shards/shard11.txt", &(get_home().unwrap()+"/shards")]).output().unwrap();
	if !output.status.success() {
    	return Err(format!("ERROR in copying shard11.txt in create setup CD = {}", std::str::from_utf8(&output.stderr).unwrap()));
    }
	//remove stale shard file
	let output = Command::new("sudo").args(["rm", "/mnt/ramdisk/shards_untrimmed.txt"]).output().unwrap();
	if !output.status.success() {
    	return Err(format!("ERROR in removing deprecated shards_untrimmed in create setup cd = {}", std::str::from_utf8(&output.stderr).unwrap()));
    }
	//stage setup CD dir with shards for distribution
	let output = Command::new("sudo").args(["cp", "-R", "/mnt/ramdisk/shards", "/mnt/ramdisk/CDROM"]).output().unwrap();
	if !output.status.success() {
    	return Err(format!("ERROR in copying shards to CDROM dir in create setup cd = {}", std::str::from_utf8(&output.stderr).unwrap()));
    }
	//create the decay directory
	Command::new("mkdir").args(["/mnt/ramdisk/CDROM/decay"]).output().unwrap();
	//create start time file
	let start_time = Command::new("date").args(["+%s"]).output().unwrap();
	let start_time_output = std::str::from_utf8(&start_time.stdout).unwrap();
	let start_time_int: i32 = start_time_output.trim().parse().unwrap();
	//these are the decay times as advertised in documentation
	// delayed_decay1
	let four_years: i32 = start_time_int + 126144000; //start_time + 4 years in seconds
	//delayed_decay2
	let four_years_two_months: i32 = start_time_int + 126144000 + 5184000; //start_time + 4 years in seconds + 2 months in seconds
	//delayed_decay3
	let four_years_four_months: i32 = start_time_int + 126144000 + 10368000; //start_time + 4 years in seconds + 4 months in seconds
	//delayed_decay4
	let four_years_six_months: i32 = start_time_int + 126144000 + 15552000; //start_time + 4 years in seconds + 6 months in seconds
	//delayed_decay5 == immediate_decay
	let four_years_eight_months: i32 = start_time_int + 126144000 + 20736000; //start_time + 4 years in seconds + 8 months in seconds

	//test times
	//TODO add a 'test' bool param here to enable better testing
	// //delayed_decay1
	// let four_years: i32 = start_time_int + 172800; //start_time + 2 days in seconds
	// //delayed_decay2
	// let four_years_two_months: i32 = start_time_int + 172800 + 86400; //start_time + 2 days in seconds + 1 day in seconds
	// //delayed_decay3
	// let four_years_four_months: i32 = start_time_int + 172800 + 172800; //start_time + 2 days in seconds + 2 days in seconds
	// //delayed_decay4
	// let four_years_six_months: i32 = start_time_int + 172800 + 259200; //start_time + 2 days in seconds + 3 days in seconds
	// //delayed_decay5 == immediate_decay
	// let four_years_eight_months: i32 = start_time_int + 172800 + 345600; //start_time + 2 days in seconds + 4 days in seconds

	//store start_time unix timestamp in the decay dir
	let mut file_ref = match std::fs::File::create("/mnt/ramdisk/CDROM/decay/start_time") {
		Ok(file) => file,
		Err(_) => return Err(format!("Could not create start time file")),
	};
	file_ref.write_all(&start_time_output.to_string().as_bytes()).expect("could not write start_time to file");
	//store delayed_decay1
	let mut file_ref = match std::fs::File::create("/mnt/ramdisk/CDROM/decay/delayed_decay1") {
		Ok(file) => file,
		Err(_) => return Err(format!("Could not create delayed_decay1 file")),
	};
	file_ref.write_all(&four_years.to_string().as_bytes()).expect("could not write delayed_decay1 to file");
	//store delayed_decay2
	let mut file_ref = match std::fs::File::create("/mnt/ramdisk/CDROM/decay/delayed_decay2") {
		Ok(file) => file,
		Err(_) => return Err(format!("Could not create delayed_decay2 file")),
	};
	file_ref.write_all(&four_years_two_months.to_string().as_bytes()).expect("could not write delayed_decay2 to file");
	//store delayed_decay3
	let mut file_ref = match std::fs::File::create("/mnt/ramdisk/CDROM/decay/delayed_decay3") {
		Ok(file) => file,
		Err(_) => return Err(format!("Could not create delayed_decay3 file")),
	};
	file_ref.write_all(&four_years_four_months.to_string().as_bytes()).expect("could not write delayed_decay3 to file");
	//store delayed_decay4
	let mut file_ref = match std::fs::File::create("/mnt/ramdisk/CDROM/decay/delayed_decay4") {
		Ok(file) => file,
		Err(_) => return Err(format!("Could not create delayed_decay4 file")),
	};
	file_ref.write_all(&four_years_six_months.to_string().as_bytes()).expect("could not write delayed_decay4 to file");
	//store delayed_decay5
	let mut file_ref = match std::fs::File::create("/mnt/ramdisk/CDROM/decay/delayed_decay5") {
		Ok(file) => file,
		Err(_) => return Err(format!("Could not create delayed_decay5 file")),
	};
	file_ref.write_all(&four_years_eight_months.to_string().as_bytes()).expect("could not write delayed_decay5 to file");
	//store immediate_decay/delayed_decay6 unix timestamp in the decay dir
	let mut file_ref = match std::fs::File::create("/mnt/ramdisk/CDROM/decay/immediate_decay") {
		Ok(file) => file,
		Err(_) => return Err(format!("Could not create immediate_decay file")),
	};
	file_ref.write_all(&four_years_eight_months.to_string().as_bytes()).expect("could not write immediate_decay to file");
	//copy decay dir to sensitive
	let output = Command::new("cp").args(["-r", "/mnt/ramdisk/CDROM/decay", "/mnt/ramdisk/sensitive"]).output().unwrap();
	if !output.status.success() {
    	return Err(format!("ERROR in copying decay dir from CDROM dir to sensitive dir= {}", std::str::from_utf8(&output.stderr).unwrap()));
    }
	//create iso from setupCD dir
	let output = Command::new("genisoimage").args(["-r", "-J", "-o", "/mnt/ramdisk/setupCD.iso", "/mnt/ramdisk/CDROM"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR create setupCD with genisoimage = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//check if the CDROM is blank, DEPRECATED LINES
	// let dir_path = "/media/ubuntu/CDROM";
	// let dir_path_exists = std::path::Path::new(dir_path).exists();
	// let is_empty = is_dir_empty(dir_path);
	//unmount the disc
	Command::new("sudo").args(["umount", &path]).output().unwrap();
	//wipe the CD, this fails if it's already blank
	let output = Command::new("sudo").args(["wodim", "-v", &("dev=".to_string()+&path), "blank=fast"]).output().unwrap();
	if !output.status.success() {
		//attempt alternative wipe method
		Command::new("sudo").args(["wodim", "-v", &("dev=".to_string()+&path), "blank=fast"]).output().unwrap();
	}
	//burn setupCD iso to the setupCD
	let output = Command::new("sudo").args(["wodim", &("dev=".to_string()+&path), "-v", "-data", "/mnt/ramdisk/setupCD.iso"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in create setupCD with burning iso = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//eject the disc
	match eject_disc(){
		Ok(res) => res,
		Err(e) => return Err(format!("ERROR in ejecting CD = {}", e))
	};
	Ok(format!("SUCCESS in Creating Setup CD"))
}

//run on any card with the config value awake set to true at application boot
#[tauri::command]
pub async fn install_warm_deps() -> Result<String, String> {
	//obtain & install recent security updates
	let output = Command::new("sudo").args(["apt", "update"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in install_warm_deps with apt update = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	let output = Command::new("sudo").args(["apt", "-y", "upgrade"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in install_warm_deps with apt upgrade = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//install xclip for copying address to desktop clipboard
	let output = Command::new("sudo").args(["apt", "-y", "install", &(get_home().unwrap()+"/dependencies/xclip_0.13-2_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing xclip {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	//install TOR for connecting to the BPS
	let output = Command::new("sudo").args(["apt", "-y", "install", &(get_home().unwrap()+"/dependencies/tor_0.4.6.10-1_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing tor {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	//install libzbar0 for zbar-tools
	let output = Command::new("sudo").args(["apt", "-y", "install", &(get_home().unwrap()+"/dependencies/libzbar0_0.23.92-4build2_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing libzbar0 {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	//install image magick for zbar-tools
	let output = Command::new("sudo").args(["apt", "-y", "install", &(get_home().unwrap()+"/dependencies/imagemagick-6-common_8%3a6.9.11.60+dfsg-1.3ubuntu0.22.04.3_all.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing image magick {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//install libaom3 for libheif1 for zbar-tools
	let output = Command::new("sudo").args(["apt", "-y", "install", &(get_home().unwrap()+"/dependencies/libaom3_3.3.0-1_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing libaom3 {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//install libdav1d5 for libheif1 for zbar-tools
	let output = Command::new("sudo").args(["apt", "-y", "install", &(get_home().unwrap()+"/dependencies/libdav1d5_0.9.2-1_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing libdav1d5 {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//install libde265-0 for libheif1 for zbar-tools
	let output = Command::new("sudo").args(["apt", "-y", "install", &(get_home().unwrap()+"/dependencies/libde265-0_1.0.8-1_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing libde265-0 {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//install libx265-199 for libheif1 for zbar-tools
	let output = Command::new("sudo").args(["apt", "-y", "install", &(get_home().unwrap()+"/dependencies/libx265-199_3.5-2_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing libx265-199 {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//install libheif1 for zbar-tools
	let output = Command::new("sudo").args(["apt", "-y", "install", &(get_home().unwrap()+"/dependencies/libheif1_1.12.0-2build1_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing libheif1 {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//install liblqr for zbar-tools
	let output = Command::new("sudo").args(["apt", "-y", "install", &(get_home().unwrap()+"/dependencies/liblqr-1-0_0.4.2-2.1_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing liblqr {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//install libmagickcore for zbar-tools
	let output = Command::new("sudo").args(["apt", "-y", "install", &(get_home().unwrap()+"/dependencies/libmagickcore-6.q16-6_8%3a6.9.11.60+dfsg-1.3ubuntu0.22.04.3_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing libmagickcore {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//install libmagickwand-6.q16-6 for zbar-tools
	let output = Command::new("sudo").args(["apt", "-y", "install", &(get_home().unwrap()+"/dependencies/libmagickwand-6.q16-6_8%3a6.9.11.60+dfsg-1.3ubuntu0.22.04.3_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing libmagickwand {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	//install zbar-tools for webcam reading qr codes
	let output = Command::new("sudo").args(["apt", "-y", "install", &(get_home().unwrap()+"/dependencies/zbar-tools_0.23.92-4build2_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing zbar-tools {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	return Ok(format!("Succes installing updates and warm depdencies!"))
}

#[tauri::command]
//install system level dependencies manually from the dependencies directory contained on each HW
//these are required for all application operations on both awake false and awake true cards
pub async fn install_hw_deps() -> Result<String, String> {
	println!("installing deps required by Hardware Wallet");
	//install HW dependencies for genisoimage
	let output = Command::new("sudo").args(["apt", "install", &(get_home().unwrap()+"/dependencies/genisoimage_9%3a1.1.11-3.2ubuntu1_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing genisoimage {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	//install HW dependencies for ssss
	let output = Command::new("sudo").args(["apt", "install", &(get_home().unwrap()+"/dependencies/ssss_0.5-5_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing ssss {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	//install HW dependencies for wodim
	let output = Command::new("sudo").args(["apt", "install", &(get_home().unwrap()+"/dependencies/wodim_9%3a1.1.11-3.2ubuntu1_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing wodim {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	//install library for qrencode
	let output = Command::new("sudo").args(["apt", "install", &(get_home().unwrap()+"/dependencies/libqrencode4_4.1.1-1_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing libqrencode {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	//install HW dependencies for qrencode
	let output = Command::new("sudo").args(["apt", "install", &(get_home().unwrap()+"/dependencies/qrencode_4.1.1-1_amd64.deb")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in installing qrencode {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	Ok(format!("SUCCESS in installing HW dependencies"))
}

//The following set of "distribute_shards" fuctions are for distributing encryption key shards to HW 2-7 during initial setup
#[tauri::command]
pub async fn distribute_shards_hw2() -> Result<String, String> {
	//create local shards dir
	Command::new("mkdir").args([&(get_home().unwrap()+"/shards")]).output().unwrap();
    //copy the shards to the target destination (primary shard)
	let output = Command::new("cp").args(["/mnt/ramdisk/CDROM/shards/shard2.txt", &(get_home().unwrap()+"/shards")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in distributing shards to HW 2 = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
    //copy the shards to the target destination (BPS shard backup)
	let output = Command::new("cp").args(["/mnt/ramdisk/CDROM/shards/shard10.txt", &(get_home().unwrap()+"/shards")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in distributing shards to HW 2 = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//copy the time_decay directory
	let output = Command::new("cp").args(["-r", "/mnt/ramdisk/CDROM/decay", "/mnt/ramdisk/sensitive"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in copying decay dir to sensitive = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	Ok(format!("SUCCESS in distributing shards to HW 2"))
}

#[tauri::command]
pub async fn distribute_shards_hw3() -> Result<String, String> {
	//create local shards dir
	Command::new("mkdir").args([&(get_home().unwrap()+"/shards")]).output().unwrap();
    //copy the shards to the target destination (primary shard)
	let output = Command::new("cp").args(["/mnt/ramdisk/CDROM/shards/shard3.txt", &(get_home().unwrap()+"/shards")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in distributing shards to HW 3 = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
    //copy the shards to the target destination (BPS shard backup)
	let output = Command::new("cp").args(["/mnt/ramdisk/CDROM/shards/shard9.txt", &(get_home().unwrap()+"/shards")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in distributing shards to HW 3 = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//copy the time_decay directory
	let output = Command::new("cp").args(["-r", "/mnt/ramdisk/CDROM/decay", "/mnt/ramdisk/sensitive"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in copying decay dir to sensitive = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	Ok(format!("SUCCESS in distributing shards to HW 3"))
}

#[tauri::command]
pub async fn distribute_shards_hw4() -> Result<String, String> {
	//create local shards dir
	Command::new("mkdir").args([&(get_home().unwrap()+"/shards")]).output().unwrap();
    //copy the shards to the target destination (primary shard)
	let output = Command::new("cp").args(["/mnt/ramdisk/CDROM/shards/shard4.txt", &(get_home().unwrap()+"/shards")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in distributing shards to HW 4 = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
    //copy the shards to the target destination (BPS shard backup)
	let output = Command::new("cp").args(["/mnt/ramdisk/CDROM/shards/shard8.txt", &(get_home().unwrap()+"/shards")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in distributing shards to HW 4 = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//copy the time_decay directory
	let output = Command::new("cp").args(["-r", "/mnt/ramdisk/CDROM/decay", "/mnt/ramdisk/sensitive"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in copying decay dir to sensitive = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	Ok(format!("SUCCESS in distributing shards to HW 4"))
}

//there are no BPS shard backups after HW 4 (TODO maybe add dupes to 5-7 for extra redundancy)
#[tauri::command]
pub async fn distribute_shards_hw5() -> Result<String, String> {
	//create local shards dir
	Command::new("mkdir").args([&(get_home().unwrap()+"/shards")]).output().unwrap();
    //copy the shards to the target destination (primary shard)
	let output = Command::new("cp").args(["/mnt/ramdisk/CDROM/shards/shard5.txt", &(get_home().unwrap()+"/shards")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in distributing shards to HW 5 = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//copy the time_decay directory
	let output = Command::new("cp").args(["-r", "/mnt/ramdisk/CDROM/decay", "/mnt/ramdisk/sensitive"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in copying decay dir to sensitive = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	Ok(format!("SUCCESS in distributing shards to HW 5"))
}

#[tauri::command]
pub async fn distribute_shards_hw6() -> Result<String, String> {
	//create local shards dir
	Command::new("mkdir").args([&(get_home().unwrap()+"/shards")]).output().unwrap();
    //copy the shards to the target destination (primary shard)
	let output = Command::new("cp").args(["/mnt/ramdisk/CDROM/shards/shard6.txt", &(get_home().unwrap()+"/shards")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in distributing shards to HW 6 = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//copy the time_decay directory
	let output = Command::new("cp").args(["-r", "/mnt/ramdisk/CDROM/decay", "/mnt/ramdisk/sensitive"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in copying decay dir to sensitive = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	Ok(format!("SUCCESS in distributing shards to HW 6"))
}

#[tauri::command]
pub async fn distribute_shards_hw7() -> Result<String, String> {
	//create local shards dir
	Command::new("mkdir").args([&(get_home().unwrap()+"/shards")]).output().unwrap();
    //copy the shards to the target destination (primary shard)
	let output = Command::new("cp").args(["/mnt/ramdisk/CDROM/shards/shard7.txt", &(get_home().unwrap()+"/shards")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in distributing shards to HW 7 = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//copy the time_decay directory
	let output = Command::new("cp").args(["-r", "/mnt/ramdisk/CDROM/decay", "/mnt/ramdisk/sensitive"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in copying decay dir to sensitive = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	Ok(format!("SUCCESS in distributing shards to HW 7"))
}

//Create a backup directory of the currently inserted HW (this does NOT include the ubuntu iso or the application binary due to space limitations)
//TODO backup the application binary if possible (defintely not doable on a CD 700mb capacity...)
#[tauri::command]
pub async fn create_backup(number: String) -> Result<String, String> {
	println!("creating backup directory of the current HW");
		//make backup dir for iso
		Command::new("mkdir").args(["/mnt/ramdisk/backup"]).output().unwrap();
		//Copy shards to backup
		let output = Command::new("cp").args(["-r", &(get_home().unwrap()+"/shards"), "/mnt/ramdisk/backup"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in creating backup with copying shards = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
		//Copy sensitive dir
		let output = Command::new("cp").args([&(get_home().unwrap()+"/encrypted.gpg"), "/mnt/ramdisk/backup"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in creating backup with copying sensitive dir = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
		//copy config
		let output = Command::new("cp").args([&(get_home().unwrap()+"/config.txt"), "/mnt/ramdisk/backup"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in creating backup with copying config.txt = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
		//copy dependencies
		let output = Command::new("cp").args(["-r", &(get_home().unwrap()+"/dependencies"), "/mnt/ramdisk/backup"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in creating backup with copying config.txt = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
		//create .iso from backup dir
		let output = Command::new("genisoimage").args(["-r", "-J", "-o", &("/mnt/ramdisk/backup".to_string()+&number+".iso"), "/mnt/ramdisk/backup"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in creating backup with creating iso = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
		Ok(format!("SUCCESS in creating backup of current HW"))
}

//make the existing backup directory into an iso and burn to the currently inserted CD/DVD/M-DISC
#[tauri::command]
pub async fn make_backup(number: String) -> Result<String, String> {
	//obtain CD path
	let path: String = match get_cd_path(){
		Ok(path) => path,
		Err(er) => {
			return Err(format!("{}", er))
		}
	};
	println!("making backup iso of the current HW and burning to CD");
	// sleep for 6 seconds
	Command::new("sleep").args(["6"]).output().unwrap();
	//wipe the CD
	Command::new("sudo").args(["umount", &path]).output().unwrap();
	//we don't mind if this fails on blank CDs
	let output = Command::new("sudo").args(["wodim", "-v", &("dev=".to_string()+&path), "blank=fast"]).output().unwrap();
	if !output.status.success() {
		//attempt alternative wipe method
		Command::new("sudo").args(["wodim", "-v", &("dev=".to_string()+&path), "blank=fast"]).output().unwrap();
	}
	//burn setupCD iso to the backup CD
	let output = Command::new("sudo").args(["wodim", &("dev=".to_string()+&path), "-v", "-data", &("/mnt/ramdisk/backup".to_string()+&number+".iso")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in making backup with burning iso to CD = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//eject the disc
	match eject_disc(){
		Ok(res) => res,
		Err(e) => return Err(format!("ERROR in ejecting CD = {}", e))
	};
	Ok(format!("SUCCESS in making backup of current HW"))
}

