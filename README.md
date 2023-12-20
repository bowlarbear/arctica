<template>
    <div class="container" style="margin-top: 3rem;">
      <h2>Arctica. A secure & private Bitcoin cold storage solution</h2>
      <p><b>WARNING: WE ARE CURRENTLY IN BETA TESTING, Arctica is not currently feature complete, while spend thresholds will still decay down to 1 of 7, users who lose their setup CD will require atleast 3 wallets to recover their login password. The timelocked wallet is not easily accessible prior to timelock expiry without a competent working knowledge of bitcoin cli. 
       <br>this software overwrites external storage media and CDs without much warning, I advise you only run arctica on a dedicated machine, remove any extraneous external storage media, and only insert new/blank USB sticks or SD cards and CDs when prompted.</b> </p>
      <p><b>Minimum Specs: </b>
      You only need access to two relatively cheap computers. These computers will be dedicated to the purpose of running arctica. The primary laptop runs the bitcoin full node and should remain unused for other activities. The second laptop can be any device, but ideally is a dedicated signing device not used for other purposes. If you don't own a second laptop you can buy one from a big box store and return it after use if required or use your primary computer as a signer. Once Arctica is set up, it will work on any computer by simply inserting an Arctica HWW and rebooting the machine. On some motherboards it might require you to enter the BIOS.</p>
      <p>I recommend replacing the laptop you intend to use as a full node's internal storage drive with atleast a 1 TB SSD drive to improve the initial sync speed significantly and provide plenty of node storage space for the immediate future. A bit of RAM helps, the most performance intensive part will be your node's initial sync, but I'm not certain yet the absolute minimum you could get away with. Just about anything can be used as a signer, I've run arctica on an old T400 Thinkpad. You could potentially sync the full node on such a machine with plenty of patience. </p>
      <p>The primary full node computer's internal storage should be flashed with a clean installation of the latest Ubuntu release prior to installing Arctica.
      <p>Arctica is essentially a few seperate things...
      <ol>
        <li>Open Source Hardware Wallets: Arctica builds all of the software you require in real time, the user follows a series of simple 1 button prompts to complete installation. Very little input is required of the user except for swapping media storage devices.</li>
        <li>2 Distinct Bitcoin Vaults: An accompanying software desktop application that runs on top of bitcoin core and allows the user to easily navigate a preconfigured 2 of 7 and 5 of 7 timelocked multisignature bitcoin wallet, these vaults are designed to solve for both security and inheritance. 
        <li>Automated Full Node Builder: Anytime the user boots from HW 1 or HW 0 (v2) and runs arctica, the software will attempt to sync the bitcoin blockchain using the computer's available internal memory storage. 
      </ol>
      <p>Arctica is a Free and Open Source wrapper script that installs bitcoin core and then walks the user through setup of a highly secure & private cold storage solution. The software is designed to make Bitcoin more difficult to lose, steal, or extort than any other asset. This protocol contains both a high security and a medium security area and is designed for storage of amounts in excess of 1 BTC in mind.</p>
      <ul>
          <li>Arctica is a key management system built in Rust on top of Bitcoin Core Backend. The <a href="https://github.com/bowlarbear/arctica-frontend">Front End Repo</a> is built with Vue.js and runs as a standalone desktop application through tauri which emulates web view without requiring the use of a browser.</li>
          <li>Arctica requires users do what is needed for safe and secure bitcoin storage even when this requires more time and effort - the first task in the Arctica instructions is to setup trustworthy & dedicated Bitcoin laptops.</li>
          <li>Before beginning, users require:</li>
           -Atleast 1 dedicated laptop to use as a full node (additional laptops may be used to boot cold wallet signers), you need a CD drive and a webcam. These may be usb tethered, however consider the number of usb ports available to you (I use (2) E490 Thinkpads with 2TB WD Blue SATA SSD, built in webcam, and USB tethered ASUS Zendrive 8X Disc drive).
           <br>-8 good quality SD cards or USB sticks of 16GB minimum (I use Kingston Data Traveler Exodia 32GB USB sticks).
           <br>-10 CD-RWs (I use Verbatim CD-RW 700MB 12X).
           <br>-7 DVDs or M-DISC (I use Verbatim DVD-R 4.7GB 16X).
           <br>-7 envelopes. 
            <li>When setting up the laptops, one should have enough internal SATA storage space to hold the entire bitcoin blockchain, currently 1Tb or higher (I'd encourage you to just go for 2TB) this will be the Primary Computer that runs an online bitcoin full node. The second will just be used as a dedicated signing device. Both laptops should be erased and flashed with ubuntu. The user can optionally install bitcoin core on their primary machine and sync the bitcoin blockchain ahead of time if desired.</li>
            <li>The SD cards/USB sticks will be configured into open source hardware wallets (HW) with the help of the arctica software. The HW devices can be booted from any computer, with the except of HW 1 and HW 0 which should only be booted from your dedicated full node (primary). You are welcome to use as many or as few secondary machines for booting other wallets as you like. CDs & DVDs/M-DISC are used to help with initial installation and encrypted backups of each wallet. The user is not required to write down any physical key or wallet backup information for the system to be secure & recoverable.</li>
          <li>Private keys are stored in 7 encrypted Hardware Wallets, when required, keys are loaded into RAM while booted to an internal Linux Live System and running the Arctica Desktop Application. This allows arctica to function as a flexible & self contained key management system which can be run on a wide variety of hardware.</li>
          <li>Artica uses both an ecrypted 5 of 7 & 2 of 7 decaying multisig for bitcoin storage. This allows up to 6 keys to be lost without losing bitcoin and requires 5 locations to be compromised by an attacker to lose privacy or funds. This prioritizes recovery redundancy and privacy.</li>
          <li>Miniscript multisig is used so that you can recover all funds using only 5 signers (high security) or 2 signers (medium security), both of which eventually decay down to 1 of 7 after a predetermined time frame (4 years and eight months).</li>
          <li>Generic computing hardware is used. Hardware sold specifically for bitcoin storage requires trusting all parties from manufacturing to shipping. Omitting potential for modified Btcoin specific hardware to steal bitcoin.</li>
          <li>Minimal software beyond bitcoin core. Bitcoin core is far and away the most trustworthy bitcoin software. Unfortunately it does not yet provide a user friendly interface for establishing a multisig address or display and accept private keys in a human writable format. We have intentionally sought to limit dependencies on external software libraries in our design process. Ideally, an Arctica user could recover their funds without our software and only use bitcoin core (with a working knowledge of the Bitcoin-CLI)</li>
          <li>Open source and easily audited. One of the reasons bitcoin core is trustworthy is that it is the most scrutinized software. This makes it the least likely to contain a critical security flaw that has not been identified and fixed. Arctica will never be as trustworthy, but by minimizing the amount of code and primarily using Rust and console commands the effort required to verify that Arctica is performing as expected is minimized.</li>
          <li>Usable for non-technical users. By following simple instructions users with moderate computer literacy can use Arctica. This is important because trusting someone to help you establish your cold storage solution introduces considerable risk. We want Arctica to be the gold standard for newcomers to bitcoin to establish a secure self custody profile.</li>
          <li>Private keys & descriptors are stored in a non-descript and encrypted manner.</li>
          <li>Private. Unlike many popular hardware and software wallets that transmit your IP address (home address) and bitcoin balance to third party servers, Arctica uses a local bitcoin core full node. This means nothing is shared beyond what is required to create a bitcoin transaction. Arctica will also use Tor (planned for v2).</li>
          <li>Counterfeit prevention. The only way to be certain that your balance represents genuine bitcoin is to use a bitcoin full node - in fact that is the primary purpose of a bitcoin full node - to verify that the bitcoin balance is correct and full of only genuine bitcoins. Any solution that does not involve a full node requires you trust someone else to tell you if you have real bitcoin.</li>
          <li>Once you have booted into an arctica hardware wallet you should navigate to the home directory and double click on the arctica software executable</li>
          <li>The prompts are designed to be completed by non technologists with minimal effort.</li>
          <li>Software instructions for recovering and spending the bitcoin are included with on every Hardware Wallet to reduce the likelihood of loss and improve UX.</li>
      </ul>
      <p>Arctica provides the best balance of security, ease of use and privacy when storing significant sums of bitcoin, it has the following disadvantages that might not be expected:</p>
      <ul>
          <li>Time. To complete setup you will need to invest several hours spread over the course of a couple days. This time includes active participation in setting up devices by following on screen prompts, syncing the blockchain, and establishing a series of security protocols.</li>
          <li>Soft Shelf Life. Because Arctica is designed to have a decaying high & medium security storage area, you will find that Arctica's security assurances intentionally degrade over time. This decision has been taken to find a balance between high security assurance and inheritance in the event of a users untimely demise. A user is advised to repeat Arctica setup shortly before or during the 4 year threshold decay.</li>
          <li>Privacy. While using bitcoin core over Tor does provide significant privacy advantages over many cold storage solutions, using multisig is not very common. This means that someone could look at the blockchain and infer that the owner of the coins is probably using our software for cold storage. This will eventually be fixed through changes to bitcoin and it is worth the security and recovery benefit to use multisig and the type of multisig you are using is only exposed to the network when you spend from Arctica (not when you deposit funds).</li>
        </ul>


  <p> <a href="https://github.com/bowlarbear/arctica-iso-builder">ISO Builder utility </a> </p>
              <p>A comprehensive technial design document can be found <a href="https://docs.google.com/document/d/1xRGrQT8TMdm1fByosnF7j1ZKKSrY7U7Q_8lHs9Qc1pQ/edit?usp=sharing">here</a> </p>
        <p>NOTE: Arctica is currently in Beta and is not currently recommended for the storage of large amounts until we have completed more extensive Beta testing.</p>

  <p> <a href="https://github.com/bowlarbear/arctica-iso-builder">ISO Builder utility </a> </p>
    </div>
</template>

<u>Dev notes:</u>

<b>First time installation</b>

To build arctica from source first install the latest rustup toolchain

clone the git repo in your home directory

`git clone https://github.com/bowlarbear/arctica`

Navigate into the arctica directory from your home directory

`cd arctica`

Run the first time submodule install 

`git submodule update --init --recursive`

Install tauri dependencies

`sudo apt update`
`sudo apt install libwebkit2gtk-4.0-dev`
`sudo apt install build-essential`
`sudo apt install curl`
`sudo apt install wget`
`sudo apt install libssl-dev`
`sudo apt install libgtk-3-dev`
`sudo apt install libayatana-appindicator3-dev`
`sudo apt install librsvg2-dev`

Compile front end first

`cd arctica-frontend`
`npm install`
`npm run build`

compile backend 

`cd ..`
`cargo build`

run the application and start following the prompts

`cargo run`

NOTE: When running arctica after building it from source, the initial portion of setup will require you enter your super user password in the terminal occasionally. Keep an eye on it. 

<b>Installing updates</b>

submodule updates

`git submodule update --recursive --remote`

navigate to the front end

`cd arctica-frontend`

compile front end

`npm run build`

return to the main directory

`cd ..`

pull down the latest for the backend

`git pull`

compile binary 

`cargo build`


run the app

`cargo run`

Please Note, developers can enable a test sandbox by setting the first line of the config.txt in their home directory to
`type=test`

This sandbox will require some custom file architecture that the app will not yet provide entirely for you. I have a series of bash scripts I use to create this architecture and I am happy to share if you would like some help. 

