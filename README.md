# Fusogen Interchain

Fusogen is a platform designed to facilitate mergers and acquisitions (M&A) like actions for decentralized autonomous organizations (DAOs). As decentralized communities and blockchain-based entities grow, there is a growing need for DAOs to consolidate resources, collaborate, and merge effectively—much like traditional businesses. However, DAOs operate under different rules, relying on smart contracts and decentralized governance, so traditional M&A processes don't apply. Fusogen addresses this gap by providing a framework to streamline the merging of DAOs, allowing them to combine assets, governance structures, and treasuries in a secure and automated manner. Simply put, Fusogen faciliates fair and secure value sharing across communities as they grow and evolve.

Fusogen began life in the Solana Colosseum and aims to be a cross-chain solution enabling greater interoperability. You can see the Solana POC here: https://www.fusogen.io/

## Prerequisites

To interact with this demo, you will need the following dependencies installed:

### Go
```bash
go version go1.23.2 darwin/arm64
```

### Cargo
```bash
cargo 1.83.0 (5ffbef321 2024-10-29)
```

### Docker
```bash
Docker version 27.3.1, build ce12230
```

### Local Interchain
```bash
local-ic version v8.8.0
```

### Hermes
```bash
hermes 1.10.4
```

## Setup Instructions

You can run the mocked test by navigating to `juno-merger` and running `cargo test`.

The local demo draws heavily from the [polytone-workshop](https://github.com/kintsugi-tech/polytone-workshop/) relying on a modified docker container for Xion and a few other tweaks. Here are the steps to run the demo once you have the dependencies installed (addresses should be deterministic):

### 1. Launch Local-IC
```bash
ICTEST_HOME=${ICTEST_HOME_PATH} local-ic start juno_hub_custom.json
```

### 2. Verify Local-IC
```bash
# Check block heights
curl -s http://127.0.0.1:26157/status | jq .result.sync_info.latest_block_height
curl -s http://127.0.0.1:26057/status | jq .result.sync_info.latest_block_height
```

### 3. Set Docker Image CLI Aliases
```bash
# Juno Docker alias
alias junod-docker="docker run --rm --network host \
  -v ${ICTEST_HOME_PATH}/homedirs/.juno:/root/.juno \
  -it ghcr.io/cosmoscontracts/juno:v24.0.0 junod"

# Xion Docker alias
alias xiond-docker="docker run --rm --network host \
  -v ${ICTEST_HOME_PATH}/homedirs/.xion:/root/.xiond \
  -it ghcr.io/somthn0somthn/xion-local-ic:latest xiond"
```

### 4. Create Hermes Connections
```bash
# Create connection (performs handshake - may take a while)
hermes create connection --a-chain localjuno-1 --b-chain localxion-1

# Verify connections
hermes query connections --chain localjuno-1
hermes query connections --chain localxion-1
```

### 5. Deploy Contracts

#### On Juno
```bash
# Deploy Note & Listener
junod-docker tx wasm store /root/.juno/artifacts/polytone_note-aarch64.wasm --from acc1 -y
junod-docker tx wasm store /root/.juno/artifacts/polytone_listener-aarch64.wasm --from acc1 -y
junod-docker q wasm codes
```

#### On Xion
```bash
# Deploy Proxy
xiond-docker tx wasm store /root/.xiond/artifacts/polytone_proxy-aarch64.wasm \
--from xion-0 \
--gas auto \
--gas-adjustment 2 \
--gas-prices 0.01uxion \
-y
```

```bash
# Deploy Voice & Confirm Code-Ids
xiond-docker tx wasm store /root/.xiond/artifacts/polytone_voice-aarch64.wasm \
--from xion-0 \
--gas auto \
--gas-adjustment 2 \
--gas-prices 0.01uxion \
-y

xiond-docker q wasm codes
```

### 6. Instantiate Contracts

#### Instantiate Note on Juno
```bash
junod-docker tx wasm instantiate 1 '{"block_max_gas":"2000000"}' --label "polytone_note_to_hub" --no-admin -y --from acc1

# Query note address
junod-docker q wasm list-contract-by-code 1
```

#### Instantiate Voice on Xion
```bash
xiond-docker tx wasm instantiate 2 \
'{
  "proxy_code_id":"1",
  "block_max_gas":"2000000"
}' \
--label "polytone_voice" \
--no-admin -y \
--gas auto --gas-adjustment 2 --gas-prices 0.01uxion \
--from xion-0

# Query voice address
xiond-docker q wasm list-contract-by-code 2

# Query proxy address - shouldn't be instantiated yet
xiond-docker q wasm list-contract-by-code 1
```

#### Create Channels
```bash
# Create channel (takes ~1 minute)
hermes create channel \
  --a-chain       localjuno-1 \
  --a-connection  connection-0 \
  --a-port        "wasm.juno14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9skjuwg8" \
  --b-port        "wasm.xion1wug8sewp6cedgkmrmvhl3lf3tulagm9hnvy8p0rppz9yjw0g4wtqhn6wsj" \
  --channel-version polytone-1

# Verify channels
hermes query channels --chain localjuno-1
hermes query channels --chain localxion-1

# Start Hermes
hermes start
```

### 7. Store Additional Contracts

#### On Juno
```bash
junod-docker tx wasm store /root/.juno/artifacts/cw20_base.wasm --from acc1 -y
junod-docker tx wasm store /root/.juno/artifacts/juno_merger.wasm --from acc1 -y
junod-docker q wasm codes
```

#### On Xion
```bash
xiond-docker tx wasm store /root/.xiond/artifacts/cw20_base.wasm \
--from xion-0 \
--gas auto \
--gas-adjustment 2 \
--gas-prices 0.01uxion \
-y

xiond-docker tx wasm store /root/.xiond/artifacts/xion_minter.wasm \
--from xion-0 \
--gas auto \
--gas-adjustment 2 \
--gas-prices 0.01uxion \
-y

xiond-docker q wasm codes
```

### 8. Token Setup on Juno

#### Token A
```bash
# Instantiate Token A
junod-docker tx wasm instantiate 3 \
  '{
    "name":"Token A",
    "symbol":"TKNA",
    "decimals":6,
    "initial_balances":[],
    "mint":{
      "minter":"juno1hj5fveer5cjtn4wd6wstzugjfdxzl0xps73ftl", 
      "cap":null
    },
    "marketing":null
  }' \
  --label "cw20-base-TokenA" \
  --from acc1 \
  --no-admin \
  -y

# Query Token A address
junod-docker q wasm list-contract-by-code 3

# Mint Token A
junod-docker tx wasm execute juno1qg5ega6dykkxc307y25pecuufrjkxkaggkkxh7nad0vhyhtuhw3seew7v3 '{
  "mint": {
    "recipient": "juno1hj5fveer5cjtn4wd6wstzugjfdxzl0xps73ftl",
    "amount": "1000000"
  }
}' --from acc1 --gas-adjustment 1.3 --gas auto -y

# Check Token A balance
junod-docker q wasm contract-state smart \
juno1qg5ega6dykkxc307y25pecuufrjkxkaggkkxh7nad0vhyhtuhw3seew7v3 \
'{
  "balance": {
    "address": "juno1hj5fveer5cjtn4wd6wstzugjfdxzl0xps73ftl"
  }
}'
```

#### Token B
```bash
# Instantiate Token B
junod-docker tx wasm instantiate 3 \
  '{
    "name":"Token B",
    "symbol":"TKNB",
    "decimals":6,
    "initial_balances":[],
    "mint":{
      "minter":"juno1efd63aw40lxf3n4mhf7dzhjkr453axurv2zdzk", 
      "cap":null
    },
    "marketing":null
  }' \
  --label "cw20-base-TokenB" \
  --from acc2 \
  --no-admin \
  -y

# Query Token B address
junod-docker q wasm list-contract-by-code 3

# Mint Token B
junod-docker tx wasm execute juno17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9jfksztgw5uh69wac2pgszu8fr9 '{
  "mint": {
    "recipient": "juno1efd63aw40lxf3n4mhf7dzhjkr453axurv2zdzk",
    "amount": "1000000"
  }
}' --from acc2 --gas-adjustment 1.3 --gas auto -y

# Check Token B balance
junod-docker q wasm contract-state smart \
juno17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9jfksztgw5uh69wac2pgszu8fr9 \
'{
  "balance": {
    "address": "juno1efd63aw40lxf3n4mhf7dzhjkr453axurv2zdzk"
  }
}'
```

### 9. Setup Merged Token on Xion
```bash
# Instantiate xion-minter passing the code_id for cw20 base, which is 3 in this case

xiond-docker tx wasm instantiate 4 '{
  "token_name": "Fusogen Merged Token",
  "token_symbol": "FMRGT",
  "token_decimals": 6,
  "cw20_code_id": 3
}' \
--label "Merged Token" \
--from xion-0 \
--no-admin \
-y \
--gas-adjustment 2 \
--gas-prices 0.01uxion \
--gas auto


# Query mint contract address
xiond-docker q wasm list-contract-by-code 4

# Query the minter config using instantiated address - record the cw20 address
xiond-docker query wasm contract-state smart xion1wkwy0xh89ksdgj9hr347dyd2dw7zesmtrue6kfzyml4vdtz6e5wsx90sn0 '{
  "get_config": {}
}' --output json

```

### 10. Setup Juno-Merger Contract
```bash
# Instantiate Juno-Merger
junod-docker tx wasm instantiate 4 '{
  "note_contract": "juno14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9skjuwg8",
  "token_a": "juno1qg5ega6dykkxc307y25pecuufrjkxkaggkkxh7nad0vhyhtuhw3seew7v3",
  "token_b": "juno17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9jfksztgw5uh69wac2pgszu8fr9",
  "xion_mint_contract": "xion1wkwy0xh89ksdgj9hr347dyd2dw7zesmtrue6kfzyml4vdtz6e5wsx90sn0"
}' --label "juno-merger" --from acc1 --no-admin -y --gas-adjustment 1.3 --gas auto

# Query code ID
junod-docker q wasm list-contract-by-code 4

# Confirm Xion minter config
junod-docker q wasm contract-state smart juno1ghd753shjuwexxywmgs4xz7x2q732vcnkm6h2pyv9s6ah3hylvrq722sry '{"get_config":{}}'
```

### 11. Get Base64 Encoded Value
```bash
echo -n '{"lock":{"xion_meta_account":"xion1h495zmkgm92664jfnc80n9p64xs5xf56qrg4vc"}}' | base64
```

### 12. Execute Token Transactions
```bash
# Send transaction from Token A contract
junod-docker tx wasm execute juno1qg5ega6dykkxc307y25pecuufrjkxkaggkkxh7nad0vhyhtuhw3seew7v3 '{
  "send": {
    "contract": "juno1ghd753shjuwexxywmgs4xz7x2q732vcnkm6h2pyv9s6ah3hylvrq722sry",
    "amount": "1234",
    "msg": "eyJsb2NrIjp7Inhpb25fbWV0YV9hY2NvdW50IjoieGlvbjFoNDk1em1rZ205MjY2NGpmbmM4MG45cDY0eHM1eGY1NnFyZzR2YyJ9fQ=="
  }
}' --from acc1 --gas-adjustment 1.3 --gas auto -y

# Check balance on Xion meta account
xiond-docker q wasm contract-state smart xion17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9jfksztgw5uh69wac2pgsmvnkd6 '{
  "balance": {
    "address": "xion1h495zmkgm92664jfnc80n9p64xs5xf56qrg4vc"  
  }
}'

# Check Token A balance on Juno
junod-docker q wasm contract-state smart \
juno1qg5ega6dykkxc307y25pecuufrjkxkaggkkxh7nad0vhyhtuhw3seew7v3 \
'{
  "balance": {
    "address": "juno1hj5fveer5cjtn4wd6wstzugjfdxzl0xps73ftl"
  }
}'
```

### 13. For secondary accounts

```bash
echo -n '{"lock":{"xion_meta_account":"xion1sudtvm9y8xpgfnkmlrd4r9x56h5vg06rp9aed0"}}' | base64

junod-docker tx wasm execute juno17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9jfksztgw5uh69wac2pgszu8fr9 '{
  "send": {
    "contract": "juno1ghd753shjuwexxywmgs4xz7x2q732vcnkm6h2pyv9s6ah3hylvrq722sry",
    "amount": "5678",
    "msg": "eyJsb2NrIjp7Inhpb25fbWV0YV9hY2NvdW50IjoieGlvbjFzdWR0dm05eTh4cGdmbmttbHJkNHI5eDU2aDV2ZzA2cnA5YWVkMCJ9fQ=="
  }
}' --from acc2 --gas-adjustment 1.3 --gas auto -y

xiond-docker q wasm contract-state smart xion17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9jfksztgw5uh69wac2pgsmvnkd6 '{
  "balance": {
    "address": "xion1sudtvm9y8xpgfnkmlrd4r9x56h5vg06rp9aed0"  
  }
}'

# Check Token B balance decreased
junod-docker q wasm contract-state smart \
juno17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9jfksztgw5uh69wac2pgszu8fr9 \
'{
  "balance": {
    "address": "juno1efd63aw40lxf3n4mhf7dzhjkr453axurv2zdzk"
  }
}'

```