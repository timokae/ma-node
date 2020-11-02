# Node App
## Requirements

The node was tested and build with version **1.47.0** (18bf6b4f0 2020-10-07) of the rust programming language.

---

**If you wish to build the binary yourself you need rust installed with the version above. If not, skip this section and continue with the section Installation.**

---

If you don't have rust installed follow the instruction on https://www.rust-lang.org/tools/install to install rust on your system.

After you installed Rust run 
```bash
rustc --version
```

It should print out
```bash
rustc 1.47.0 (18bf6b4f0 2020-10-07)
```

If not run the following command to update your rust compiler version
```bash
rustup update
```

## Installation
Clone the repository to a preferred location.
```bash
git clone https://github.com/timokae/ma-node.git
```

Step into the downloaded repo
```bash
cd ma-node
```

Create a new directory
```bash
mkdir run
```

### Binary
Download the binary for your operating system (mac/linux) from Github 
https://github.com/timokae/ma-node/releases/latest and place the binary file inside the created folder `run` and rename the binary to `node-app`

### Build yourself
If you want to build the binary yourself. Inside the `ma-node` folder
1. Run `cargo build --release`
2. Copy the binary into the run folder `cp target/release/node-app run`


## Configuration
Move the state folder into the `run` directory with
```bash
mv state run/
```

Go into the `run` directory
```bash
cd run
```

The directory `run` should look like this now
```bash
$ tree
.
├── node-app
└── state
    ├── config.json
    ├── files
    ├── file_state.json
    └── stat_state.json
```

In `state/config.json`
- change the `fingerprint` to a unique name, for example a the first to letters of your name with some random numbers. It has to be string!
- (optional) change the `port` property to the port the node should be running on. It must be visible from outside your network, for example by port forwarding!

In `state/stat_state.json` change the values of
- `uptime`: Time of the day your computer is usually online.  As an example, if your online from 8:00 to 12:00, insert [8, 12], 
- `capacity`: Amount of space you want to offer to the network (in bytes).
- `connection`: Bandwidth of your internet connection (in bits/s)
  
Do no change `first_online`, `region` and `uptime`!

The files should look like this, with your input instead

state/config.json
```json
{
  "fingerprint": "tk7331",
  "port": 8082,
  "manager_addr": "http://manager.peerdata.9e-staging.cloud/"
}
```

state/stat_store.json
```json
{
  "first_online": 0,
  "region": "europe",
  "uptime": { "value": [10, 16], "weight": 0.2 },
  "capacity": { "value": 100000000, "weight": 0.3 },
  "connection": { "value": 120000, "weight": 0.2 },
  "uptime_counter": { "value": 0, "weight": 0.5 }
}
```

## Starting the node

To start the node run
```bash
./node-app ./state
```

If everything was done correctly, you should see something close to his
```bash
[INFO][2020-10-28 15:15:17] Assigned to monitor on address http://167.99.248.254
[INFO][2020-10-28 15:15:17] Node registered with address 45.138.43.136:8082
[INFO][2020-10-28 15:15:17] FileStore initialized: []
[INFO][2020-10-28 15:15:17] Region: europe
[INFO][2020-10-28 15:15:17] Services started
[INFO][2020-10-28 15:15:17] Startet server on 0.0.0.0:8082
[INFO][2020-10-28 15:15:17] Starting ping service.
[INFO][2020-10-28 15:15:17] Starting recover service
[INFO][2020-10-28 15:15:17] Starting distribution service
```

To test if other node can talk to your node, copy the the address from line two from the output above. In this case `45.138.43.136:8082` and append the path `ping` like this
```
http://45.138.43.136:8080/ping
```
Open this URL in your browser. If everything works fine, your node should respond with `pong`. 

Otherwise check your settings for exposing your local machine. If your computer is not reachable, other devices are not able to download files from your node.

## Using the network
To Upload a file, open `http://manager.peerdata.9e-staging.cloud/` in your browser. Use the form to select a file from your computer. By submitting the form, the file gets uploaded to a node. The node then replicates the data. After your upload the manager shows your the hash of your file. Keep this hash if you want to download your later later.

To download the file visit `http://manager.peerdata.9e-staging.cloud/download`. Enter the hash of the file you want to download. The manager then searches for your file in the network. If a node with the given hash is reachable, the download starts automatically.

## Troubleshooting

### Wrong Manager Address
```bash
thread 'main' panicked at 'builder error: relative URL without a base',
```
Make sure the `manager_addr` property in the `state/config.json` is a valid url like `http://manager.peerdata.9e-staging.cloud`

### Port already in use
```bash
thread 'main' panicked at 'error binding to 0.0.0.0:8080: error creating server listener: Address already in use (os error 98)
```
Make sure, that the choosen port is not blocked or already in use. Otherwise choose another port.

###