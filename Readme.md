# pxlclnt

This is a client for a (Pixelflut)[https://wiki.cccgoe.de/wiki/Pixelflut] server. On my local server it manages to send up to 40Gb per second which is more than most network connections are capable of receiving.

## Compilation

``` bash
cargo build --release
```

## Usage
```bash
cargo run --release -- help # gives a short usage message
```

### Casting an image

``` bash
cargo run --release -- -d $DOMAIN -t $CPU_CORE_COUNT -t -l --size $IMAGE_SIZE image $UPPER_LEFT_CORNER_X $UPPER_LEFT_CORNER_Y $PATH_TO_IMAGE
```

Replace the Variables to your scenario.
