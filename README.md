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
