# Blockstream

A web3 event subscriber for tracking and capturing smart contract events in real-time.

![Dynamic TOML Badge](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Fromanovich23%2Fblockstream%2Fmain%2FCargo.toml&query=%24.package.version&prefix=v&style=for-the-badge&label=version&color=%231E90FF)
![GitHub License](https://img.shields.io/github/license/romanovich23/blockstream?style=for-the-badge&color=%23AFEEEE)
![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/romanovich23/blockstream/ci.yml?style=for-the-badge&label=CI&color=%2390EE90)

## Testing

To test the application, you can use the following commands:

```shell

docker run -p 8545:8545 romanovich23/anvil-node

cargo run

git clone git@github.com:romanovich23/dummy-contracts.git YOUR_WORKSPACE_PATH/dummy-contracts

cd YOUR_WORKSPACE_PATH/dummy-contracts

forge create src/DummyContract.sol:DummyContract --rpc-url http://localhost:8545 --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80

cast send --rpc-url http://localhost:8545 --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512 "createDummyStruct(uint256,int256)" 42 -- -5

```

## Configuration

### Environment Variables

The project uses the following environment variables:

- `BLOCKCHAIN_PROTOCOL`: Protocol to connect to the blockchain (default: `ws`)
- `BLOCKCHAIN_HOST`: Host of the blockchain node (default: `localhost`)
- `BLOCKCHAIN_PORT`: Port of the blockchain node (default: `8545`)
- `BLOCKCHAIN_PATH`: Path to the blockchain node (default: empty)
- `CONTRACT_ADDRESS`: Address of the smart contract to subscribe to (default:
  `0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512`)

### YAML Configuration

The configuration is stored in `resources/application.yml` and can be overridden by environment-specific files like
`resources/application-{env}.yml`.

Example `application.yml`:

```yaml
network:
  protocol: ${BLOCKCHAIN_PROTOCOL:ws}
  host: ${BLOCKCHAIN_HOST:localhost}
  port: ${BLOCKCHAIN_PORT:8545}
  path: ${BLOCKCHAIN_PATH:}

subscriptions:
  - contract_address: ${CONTRACT_ADDRESS:0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512}
    events:
      - DummyStructCreated(uint256,uint256,int256,bool,address,string,bytes32)
  - contract_address: ${CONTRACT_ADDRESS:0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512}
    events:
      - DummyStructUpdated(uint256,uint256,int256,bool,address,string,bytes32)
```
