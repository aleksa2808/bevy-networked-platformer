# bevy-networked-platformer

Prototype game initially made for the first bevy-jam. The networking library used is [crystalorb](https://github.com/ErnWong/crystalorb) and its use was heavily inspired by the [orbgame](https://github.com/vilcans/orbgame) repo.

At the moment the basic game mechanics are in place. However, the networking part still needs work as the server keeps correcting the clients in a very intrusive manner, making them feel glitchy.

## Running

Run the server:

```
cargo run --package platformer-server
```

Then run two clients:

```
cargo run --package platformer-client
```