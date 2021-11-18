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

## Installation
### Before start
You need to prepare a runnable full ygopro under server mode.
If your platform cannot run ygopro in server, but with docker and you have a srvpro image, you can try to use a srvpro container as ygopro server:
```
#!/bin/bash
if [[ "$1" == "0" ]]; then
   set -- $((RANDOM + 20000)) "${@:2}"
fi

docker run --rm -p $1:$1 -v {YOUR_CARD_CDB_HERE}:/ygopro-server/ygopro/cards.cdb -w /ygopro-server/ygopro srvpro /ygopro-server/ygopro/ygopro $@
```
If do so, due to docker's behaviour, you may need to set configuration `config/srvpru/ygopro/wait_start` (Personally 10 on My mac)
### Raw
- Configure your ygopro server position on `config/srvpru/ygopro/binary`, and other fields.
- Pick some you plugins you like.
- Run
```
cargo build --release
export RUST_LOG=srvpru 
export SRVPRU_CONFIG_PATH=${YOUR_CONFIG_PATH_HERE}
./srvpru
```

### With docker
```
$ docker build -t srvpru .
$ docker run -d --name srvpru srpru
```
You can change your ygopro supplier in `Dockerfile` if needed.

### With K8s
coming soon
