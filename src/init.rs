use std::process::Command;
use std::fs;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use regex::Regex;
use std::fs::OpenOptions;
use std::io::Read;

//import helper.rs module
use crate::helper::{
    get_user, get_home, write, generate_keypair, store_string, is_dir_empty,
	get_cd_path, parse_fdisk_result, run_fdisk, find_new_device
};

//pre-install script
#[tauri::command]
pub async fn pre_install(root: String, target: String) -> Result<String, String> {
    //ping linux servers once to check for connectivity
	let output = Command::new("ping").args(["-c", "1", "linux.org"]).output().unwrap();
	if !output.status.success() {
		return Err(format!("Error in Pre_install ping failed networking is not enabled"))
	}
    //create destination directory
    let arc = std::path::Path::new(&(get_home().unwrap()+"/arctica-tmp")).exists();
	if arc == false {
		let output = Command::new("mkdir").arg(&(get_home().unwrap()+"/arctica-tmp")).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in pre_install with creating arctica-tmp directory = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}

	//TODO fetch the iso from a remote server...right now it must be placed locally

	//NOTE users may manually bypass the prebuilt ubuntu iso and build the iso themselves using our utility 
	//https://github.com/bowlarbear/arctica-iso-builder 
	//users should create the arctica-tmp directory in their home dir and move the arctica-ubuntu-22.04-amd64.iso created with the utility into it

    //check if ubuntu iso already exists, and if not, obtain
	// let b = std::path::Path::new(&(get_home().unwrap()+"/arctica-tmp/arctica-ubuntu-22.04-amd64.iso")).exists();
    // if b == false{
    // 	let output = Command::new("wget").args(["-O", "arctica-ubuntu-22.04-amd64.iso", "https://old-releases.ubuntu.com/releases/jammy/ubuntu-22.04-desktop-amd64.iso"]).output().unwrap();
    // 	if !output.status.success() {
    // 		return Err(format!("ERROR in pre-install with downloading ubuntu iso = {}", std::str::from_utf8(&output.stderr).unwrap()));
    // 	}
    // }

	//flash a pre-install usb with dd, with root pass
	if root.len() > 0{
		//DD target device by piping in the password
		let mut sudo = Command::new("sudo")
		.args(["dd", &("if=".to_string()+&get_home().unwrap()+"/arctica-tmp/arctica-ubuntu-22.04-amd64.iso"), &("of=".to_string()+&target), "bs=16M", "oflag=sync", "status=progress"])
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
			return Err(format!("ERROR in pre_install with Burning ISO with DD & piping root = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	//flash pre-install with no dd, no root pass, TODO not yet tested
	else{
		let output = Command::new("sudo")
        .args(["dd", &("if=".to_string()+&get_home().unwrap()+"/arctica-tmp/arctica-ubuntu-22.04-amd64.iso"), &("of=".to_string()+&target), "bs=16M", "oflag=sync", "status=progress"])
        .output()
        .unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in pre_install with Burning ISO with DD & blank sysPass = {}", std::str::from_utf8(&output.stderr).unwrap()));
    	}
	}
	//TODO remove the arctica temp dir. keep it for now until bulk of dev work and testing is finished
	//Flush the Filesystem Buffers
	Command::new("sync").output().unwrap();
    Ok(format!("pre-install complete!"))
}


//build the arctica iso for initial flash
#[tauri::command]
pub async fn init_iso() -> Result<String, String> {
	//install updates
	Command::new("sudo").args(["apt", "-y", "update"]).output().unwrap();
	Command::new("sudo").args(["apt", "-y", "upgrade"]).output().unwrap();
	Command::new("sudo").args(["apt", "install", "curl"]).output().unwrap();
	Command::new("sudo").args(["apt", "-y", "update"]).output().unwrap();
	//obtain the application's current working directory
	let initial_cwd_buf = match env::current_dir(){
		Ok(data) => data,
		Err(err) => return Err(format!("Error obtaining initial cwd buf {}", err.to_string()))
	};
	//convert cwd to string
	let initial_cwd = initial_cwd_buf.to_str();
	//create the application directory
	let arc = std::path::Path::new(&(get_home().unwrap()+"/arctica")).exists();
	if arc == false {
		let output = Command::new("mkdir").arg(&(get_home().unwrap()+"/arctica")).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in init iso with creating arctica directory = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	//remove stale iso mount if exist from a previous session
	let iso = std::path::Path::new(&("/media/".to_string()+&get_user().unwrap()+"/Ubuntu 22.04.4 LTS amd64")).exists();
	if iso == true{
		let output = Command::new("sudo").args(["umount", &("/media/".to_string()+&get_user().unwrap()+"/Ubuntu 22.04.4 LTS amd64")]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in init iso with unmounting stale ubuntu iso = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	//create warm dependencies directory if it does not exist
	let warm = std::path::Path::new(&(get_home().unwrap()+"/arctica/warm")).exists();
	if warm == false {
		let output = Command::new("mkdir").arg(&(get_home().unwrap()+"/arctica/warm")).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in init iso with creating arctica warm deps directory = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	//create cold dependencies directory if it does not exist
	let cold = std::path::Path::new(&(get_home().unwrap()+"/arctica/cold")).exists();
	if cold == false {
		let output = Command::new("mkdir").arg(&(get_home().unwrap()+"/arctica/cold")).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in init iso with creating arctica cold deps directory = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	//set the current working directory to the warm deps directory
	env::set_current_dir(&(get_home().unwrap()+"/arctica/warm"));
    //download system level dependencies required for warm Hardware Wallets
	Command::new("sudo").args(["apt", "download", "xclip", "tor", "zbar-tools", "libzbar0", "libmagickwand-6.q16-6", "imagemagick-6-common", "libmagickcore-6.q16-6", "libheif1", "liblqr-1-0", "libaom3", "libdav1d5", "libde265-0", "libx265-199"]).output().unwrap();
	//set the current working directory to the cold deps directory
	env::set_current_dir(&(get_home().unwrap()+"/arctica/cold"));
	//download system level dependencies required for cold Hardware Wallets
	Command::new("sudo").args(["apt", "download", "wodim", "genisoimage", "ssss", "qrencode", "libqrencode4"]).output().unwrap();
	//set the current working directory to the application directory
	env::set_current_dir(&(get_home().unwrap()+"/arctica"));
	//check if ubuntu iso & bitcoin core already exists, and if no, obtain
	let b = std::path::Path::new(&(get_home().unwrap()+"/arctica/ubuntu-22.04.4-desktop-amd64.iso")).exists();
	let c = std::path::Path::new(&(get_home().unwrap()+"/arctica/bitcoin-25.0-x86_64-linux-gnu.tar.gz")).exists();
	if b == false{
		let output = Command::new("curl").args(["-o", "ubuntu-22.04.4-desktop-amd64.iso", "https://releases.ubuntu.com/jammy/ubuntu-22.04.4-desktop-amd64.iso"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in init iso with downloading ubuntu iso = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	if c == false{
		let output = Command::new("wget").args(["https://bitcoincore.org/bin/bitcoin-core-25.0/bitcoin-25.0-x86_64-linux-gnu.tar.gz"]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in init iso with downloading bitcoin core = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	//remove stale persistent isos
	Command::new("sudo").args(["rm", "persistent-ubuntu.iso"]).output().unwrap();
	Command::new("sudo").args(["rm", "persistent-ubuntu1.iso"]).output().unwrap();
	//make the scripts dir if it doesn't exist
	let d = std::path::Path::new(&(get_home().unwrap()+"/arctica/scripts")).exists();
	if d == false {
		let output = Command::new("mkdir").arg(&(get_home().unwrap()+"/arctica/scripts")).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in creating the scripts directory {}", std::str::from_utf8(&output.stderr).unwrap()));
		} 
	}
	//create sed1 script
	let file = File::create(&(get_home().unwrap()+"/arctica/scripts/sed1.sh")).unwrap();
	//populate sed1.sh with bash
	let output = Command::new("echo").args(["-e", "< ubuntu-22.04.4-desktop-amd64.iso sed 's/maybe-ubiquity/  persistent  /' > persistent-ubuntu1.iso"]).stdout(file).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR with creating sed1.sh: {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//create sed2 script
	let file = File::create(&(get_home().unwrap()+"/arctica/scripts/sed2.sh")).unwrap();
	//populate sed2.sh with bash
	let output = Command::new("echo").args(["-e", "< persistent-ubuntu1.iso sed 's/set timeout=30/set timeout=1 /' > persistent-ubuntu.iso"]).stdout(file).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR with creating sed2.sh: {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//execute sed1.sh which modifies the ubuntu iso to have persistence
	let output = Command::new("bash").args([&(get_home().unwrap()+"/arctica/scripts/sed1.sh")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in running sed1 {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	//verify that the iso has been modified
	let exists = Path::new(&(get_home().unwrap()+"/arctica/persistent-ubuntu1.iso")).exists();
	if !exists {
		return Err(format!("ERROR in running sed1, script completed but did not create iso"));
	}
	//execute sed2.sh which modifies ubuntu iso to have a shorter timeout at boot screen
	let output = Command::new("bash").args([&(get_home().unwrap()+"/arctica/scripts/sed2.sh")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in running sed2 {}", std::str::from_utf8(&output.stderr).unwrap()));
	} 
	//verify that the iso has been modified
	let exists = Path::new(&(get_home().unwrap()+"/arctica/persistent-ubuntu.iso")).exists();
	if !exists {
		return Err(format!("ERROR in running sed2, script completed but did not create iso"));
	}
	//remove the stale persistent iso
	Command::new("sudo").args(["rm", "persistent-ubuntu1.iso"]).output().unwrap();
	//reset the current working directory
	env::set_current_dir(get_home().unwrap());
	Ok(format!("SUCCESS in init_iso"))
}

//used to create a bootable hardware wallet
#[tauri::command]
pub async fn create_bootable_usb(number: String, setup: String, awake: bool, baseline: String) -> Result<String, String> {
	//TODO check for existing arctica installation and return error if found
	//OR should we let the software overwrite an existing installation in case a user wants to retry after a failed attempt with the same stick?
	
	//obtain application's current working directory
	let initial_cwd_buf = match env::current_dir(){
		Ok(data) => data,
		Err(err) => return Err(format!("ERROR in create_bootable with obtaining initial cwd buf {}", err.to_string()))
	};
	//convert cwd to string
	let initial_cwd = initial_cwd_buf.to_str();
	//calculate & define target device
	let target = match find_new_device(&baseline){
		Ok(result) => result,
		Err(e) => 
		return Err(e.to_string())
	};
	//unmount the target device
	Command::new("sudo").args(["umount", &target]).output().unwrap();
	//calculate & define target device
	let target = match find_new_device(&baseline){
		Ok(result) => result,
		Err(e) => 
		return Err(e.to_string())
	};
	//burn iso with dd
	let output = Command::new("sudo")
        .args(["dd", &("if=".to_string()+&get_home().unwrap()+"/arctica/persistent-ubuntu.iso"), &("of=".to_string()+&target), "bs=4M", "oflag=sync", "status=progress"])
        .output()
        .unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in create_bootable with Burning ISO with DD = {}", std::str::from_utf8(&output.stderr).unwrap()));
    	}
	//Flush the Filesystem Buffers
	Command::new("sync").output().unwrap();
	//unmount the target device
	Command::new("sudo").args(["umount", &target]).output().unwrap();
	//calculate & define target device
	let target = match find_new_device(&baseline){
		Ok(result) => result,
		Err(e) => 
		return Err(e.to_string())
	};
	//create the persistent partition table
	let script = std::path::Path::new(&(get_home().unwrap()+"/arctica/scripts/create_partition.sh")).exists();
	if script == true{
		//remove stale create_partition.sh script
		let output = Command::new("sudo").args(["rm", &(get_home().unwrap()+"/arctica/scripts/create_partition.sh")]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in create_bootable with removing stale create_partition.sh script: {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	}
	//create persistent partition shell script
	let file = File::create(&(get_home().unwrap()+"/arctica/scripts/create_partition.sh")).unwrap();
	//populate script with bash
	let output = Command::new("echo").args(["-e", &("(\necho n\necho 4\necho \necho \necho w\n) | sudo fdisk --wipe=always ".to_string()+&target)]).stdout(file).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in create_bootable with creating create_partition.sh: {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//make create_partition.sh executable
	let output = Command::new("sudo").args(["chmod", "+x", &(get_home().unwrap()+"/arctica/scripts/create_partition.sh")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in create_bootable with making create_partition.sh executable: {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//execute persistent partition shell script
	Command::new("bash").arg(&(get_home().unwrap()+"/arctica/scripts/create_partition.sh")).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in create_bootable with executing create_partition.sh: {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//remove persistent partition shell script
	let output = Command::new("rm").arg(&(get_home().unwrap()+"/arctica/scripts/create_partition.sh")).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in create_bootable with removing create_partition.sh: {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//sleep for 2 seconds
	Command::new("sleep").args(["2"]).output().unwrap();
	//Flush the Filesystem Buffers
	Command::new("sync").output().unwrap();
	//attempt to unmount all partitions
	Command::new("sudo").args(["umount", &target]).output().unwrap();
	Command::new("sudo").args(["umount", &(target.to_string()+"1")]).output().unwrap();
	Command::new("sudo").args(["umount", &(target.to_string()+"2")]).output().unwrap();
	Command::new("sudo").args(["umount", &(target.to_string()+"3")]).output().unwrap();
	Command::new("sudo").args(["umount", &(target.to_string()+"4")]).output().unwrap();
	//refresh device list
	// Command::new("sudo").args(["partx", "-u", &target]).output().unwrap();
	Command::new("sudo").arg("partprobe").output().unwrap();
	//sleep 2 seconds
	Command::new("sleep").args(["2"]).output().unwrap();
	//calculate & define target device
	let target = match find_new_device(&baseline){
		Ok(result) => result,
		Err(e) => 
		return Err(e.to_string())
	};
	//make the partition file system
	let output = Command::new("sudo").args(["mkfs.ext4", &(target.to_string()+"4")]).output().unwrap();
	if !output.status.success(){
		return Err(format!("ERROR in create_bootable with making partition file system: {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//Label the partition
	let output = Command::new("sudo")
	.args(["e2label", &(target.to_string()+"4"), "writable"])
	.output()
	.unwrap();
	if !output.status.success(){
		return Err(format!("ERROR in create_bootable with labeling writable partition"))
	}
	//unmount stale writable
	Command::new("sudo").args(["umount", &("/media/".to_string()+&get_user().unwrap()+"/writable")]).output().unwrap();
	Command::new("sudo").args(["rm", "-r", &("/media/".to_string()+&get_user().unwrap()+"/writable")]).output().unwrap();
	let output = Command::new("sudo").args(["mkdir", &("/media/".to_string()+&get_user().unwrap()+"/writable")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in create_bootable with creating writable directory = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//mount the writable partition
	let output = Command::new("sudo").args(["mount", &(target.to_string()+"4"), &("/media/".to_string()+&get_user().unwrap()+"/writable")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in create_bootable with mounting writable partition = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//open file permissions for persistent directory
	let output = Command::new("sudo").args(["chmod", "777", &("/media/".to_string()+&get_user().unwrap()+"/writable")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in create_bootable with opening file permissions of persistent dir = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//make the user directory and arctica directory
	let output = Command::new("mkdir").args(["-p", &("/media/".to_string()+&get_user().unwrap()+"/writable/upper/home/ubuntu/arctica")]).output().unwrap();
		if !output.status.success() {
			return Err(format!("ERROR in icreate_bootable with creating writable/upper/home/user directory = {}", std::str::from_utf8(&output.stderr).unwrap()));
		}
	//copying warm dependencies
	let output = Command::new("cp").args(["-r", &(get_home().unwrap()+"/arctica/warm"), &("/media/".to_string()+&get_user().unwrap()+"/writable/upper/home/ubuntu/dependencies")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in create_bootable with copying warm deps = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//copying cold dependencies
	let output = Command::new("cp").args(["-r", &(get_home().unwrap()+"/arctica/cold"), &("/media/".to_string()+&get_user().unwrap()+"/writable/upper/home/ubuntu/dependencies")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in create_bootable with copying cold deps = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//copy over arctica binary
	let output = Command::new("cp").args([format!("{}/Arctica", initial_cwd.unwrap()), format!("/media/{}/writable/upper/home/ubuntu", get_user().unwrap())]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in create_bootable with copying arctica binary = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//extract bitcoin core
	let output = Command::new("tar").args(["-xzf", &(get_home().unwrap()+"/arctica/bitcoin-25.0-x86_64-linux-gnu.tar.gz"), "-C", &("/media/".to_string()+&get_user().unwrap()+"/writable/upper/home/ubuntu")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in create_bootable with extracting bitcoin core = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//create target device .bitcoin dir
	let output = Command::new("mkdir").args([&("/media/".to_string()+&get_user().unwrap()+"/writable/upper/home/ubuntu/.bitcoin")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in create_bootable with making target .bitcoin dir = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//create bitcoin.conf on target device
	let file = File::create(&("/media/".to_string()+&get_user().unwrap()+"/writable/upper/home/ubuntu/.bitcoin/bitcoin.conf")).unwrap();
	let output = Command::new("echo").args(["-e", "rpcuser=rpcuser\nrpcpassword=477028\nspendzeroconfchange=1"]).stdout(file).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in create_bootable, with creating bitcoin.conf = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//init awake val
	let mut awake_val = "false";
	if awake {
		awake_val = "true";
	}
	//write device type to config, values provided by front end
	let file = File::create(&("/media/".to_string()+&get_user().unwrap()+"/writable/upper/home/ubuntu/config.txt")).unwrap();
	Command::new("echo").args(["type=hardwareWallet\nhwNumber=".to_string()+&number.to_string()+&"\nsetupStep=".to_string()+&setup.to_string()+&"\nawake=".to_string()+&awake_val.to_string()]).stdout(file).output().unwrap();
	println!("creating bootable ubuntu device writing config...HW {} Setupstep {}", number, setup);
	// sleep for 3 seconds
	Command::new("sleep").args(["3"]).output().unwrap();
	//open file permissions for config
	let output = Command::new("sudo").args(["chmod", "777", &("/media/".to_string()+&get_user().unwrap()+"/writable/upper/home/ubuntu/config.txt")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in creating bootable with opening file permissions = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	//flush the filesystem buffers
	Command::new("sync").output().unwrap();
	//unmount writable
	let output = Command::new("sudo").args(["umount", &("/media/".to_string()+&get_user().unwrap()+"/writable")]).output().unwrap();
	if !output.status.success() {
		return Err(format!("ERROR in create_bootable with unmounting writable = {}", std::str::from_utf8(&output.stderr).unwrap()));
	}
	Ok(format!("SUCCESS in creating bootable device"))
}
