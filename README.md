# beacon-metrics-gazer

Collects network wide participation metrics given a range of indexes

<!-- HELP_START -->
```
Collects network wide participation metrics given a range of indexes

Usage: beacon-metrics-gazer [OPTIONS] <URL>

Arguments:
  <URL>  Beacon HTTP API URL: http://1.2.3.4:4000

Options:
  -H, --headers <HEADERS>          Extra headers sent to each request to the beacon node API at `url`. Same format as curl: `-H "Authorization: Bearer {token}"`
      --ranges <RANGES>            Index ranges to group IDs as JSON or TXT. Example: `{"0..100": "lh-geth-0", "100..200": "lh-geth-1"}
      --ranges-file <RANGES_FILE>  Local path or URL containing a file with index ranges with the format as defined in --ranges
      --dump                       Dump participation ranges print to stderr on each fetch
  -p, --port <PORT>                Metrics server port [default: 8080]
      --address <ADDRESS>          Metrics server bind address [default: 127.0.0.1]
  -v, --verbose                    Increase verbosity level
  -h, --help                       Print help
  -V, --version                    Print version

```
<!-- HELP_END -->

It's convenient to upload the ranges file somewhere persistent like a Github gist

```
docker run dapplion/beacon-metrics-gazer http://80.1.2.80:4000 --ranges-file https://pastebin.com/raw/FfJdfJrV
```

The format of the ranges file is very flexible, can be JSON, YAML or plain text:

```
0-500 Nethermind lighthouse-0
500-1000 Nethermind lighthouse-1
1000-1500 Nethermind teku-0
1500-2000 Nethermind teku-1
2000-2500 Nethermind lodestar-0
2500-3750 Gateway lh + nethermind
3750-5000 Gateway lh + nethermind
```

## From dockerhub

```
docker run dapplion/beacon-metrics-gazer --help
```

## bin usage

```
cargo install beacon-metrics-gazer
```
```
beacon-metrics-gazer --help
```
