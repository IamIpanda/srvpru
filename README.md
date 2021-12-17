# Srvpru

Srvpru is an alternative for srvpro, rewritten by Rust.

**WARNING**: This program is still in progress, any function can break down or be not implemented.

## It supports:
* room
* plugin

## Don't support:
* chatroom

## Example plugin
* debugger
* version check
* random match
* roomlist
* dialogue
* watch game on the fly
* reconnect
* send replay after match
* chat/name sensitive words check
* upload deck/duelresult to analyzer

## Before start
#### Prepare ygopro
You need to prepare a runnable full ygopro under server mode.
If your platform cannot run ygopro in server, but with docker and you have a srvpro image, you can try to use a srvpro container as ygopro server:
```
#!/bin/bash
if [[ "$1" == "0" ]]; then
   set -- $((RANDOM + 20000)) "${@:2}"
fi

docker run --rm -p $1:$1 -v {YOUR_CARD_CDB_HERE}:/ygopro-server/ygopro/cards.cdb -w /ygopro-server/ygopro srvpro /ygopro-server/ygopro/ygopro $@
```
If do so, due to docker's behaviour, you may need to set configuration `config/srvpru.yaml/ygopro/wait_start` (Personally 10 on My mac)

#### Configure
Srvpru use a config file for each plugin. The main application read `config/srvpru.yaml` before server start.

To correctly locate that config file, you need to set a env variable `SRVPRU_CONFIG_PATH` towards the config folder. Its different location is `./config`.

Before start service, pick some plugin you think needed and put their config file in config folder. Config file need to have the same name with plugin and can be with any of following extensions:
- yaml
- toml
- json

#### Migrate config from srvpro
Add a environment variable `SRVPRO_CONFIG_PATH` towards your srvpro config.

**This will replace influenced config file in srvpru config folder.**

## Deploy
#### Run srvpru with raw binary
- Configure your ygopro server position on `config/srvpru/ygopro/binary`, and other fields.
- Pick some you plugins you like.
- Run
```
cargo build --release
export RUST_LOG=srvpru 
export SRVPRU_CONFIG_PATH=${YOUR_CONFIG_PATH_HERE}
./srvpru
```

##### Run srvpru in docker
```
$ docker build -t srvpru .
$ docker run -d --name srvpru srpru
```
You can change your ygopro supplier in `Dockerfile` if needed.

#### Run srvpru in K8s with K8s ygopro distribution
coming soon
