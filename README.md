## Aoraki-labs Decentralized contract demo requestor

### Build

Run `cargo build --release` to build the binary.
And then, 
`cp ./target/release/zkpool-demo-requestor .`

### Run

Run like this:
```
	./zkpool-demo-requestor -i xxxxxx -k xxxxxx -r xxxxxx -l xxxxxx

```
You can also refer to the usage help (`./zkpool-demo-requestor -h`) or app.yml(under ./src/ directory)
```
    -c, --contracts <contract>    ZKPool demo contract [default: 82340e0f080054db0d5098b8901a53efec628600]
    -i, --interval <interval>    The interval time to send dummy task [default: 300]
    -k, --key <key>              Set the private key to sign the blockchain request [default: ]
    -l, --listen <listen>        Set the rpc server api endpoint [default: 0.0.0.0:5678]
    -r, --relayer <relayer>      The relayer rpc endpoint [default: http://127.0.0.1:6789]
```




