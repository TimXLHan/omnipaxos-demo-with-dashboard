# omnipaxos-demo-with-dashboard
## Usage
Build images and start containers in detached mode:
```bash
$ docker compose up --build -d
```
Stop all containers:
```bash
$ docker kill $(docker ps -q)
```
Attach to network-actor to give commands to the cluster:
```bash
$ docker attach coordinator
```

# Demo commands

## Recovery from partition or disconnection
1. Propose batch when fully-connected
```bash
batch 10000
```
2. Create partition
```bash
scenario qloss
```
3. Show down-time and how leader changed.

## Other commands
### Disconnect a node completely:
```bash
connection <node_id> false
```

### Restore connections
```bash
scenario restore
```
