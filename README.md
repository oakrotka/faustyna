# Faustyna
Chess engine with a UCI interface.

## Usage
After compilation with the `--release` flag, the binary should be in `target/release/faustyna`.
You can then pass this binary to any program that accepts UCI chess engines.

For example, using `cutechess-cli`
```
cutechess-cli \
    -engine name=Faustyna cmd=./target/release/faustyna \
    -engine name=OtherEngine cmd=path/to/other/engine \
    -each proto=uci tc=5+0.5 -rounds 2
```

Or, in the cutechess GUI:
- Go to `Tools -> Settings -> Engines`
- Click the plus icon in the bottom left corner
- Specify the path to engine's binary, make sure the protocol is `uci`, save the changes
- Go to `Game -> New…`, then choose `CPU -> Faustyna` as the opponent
