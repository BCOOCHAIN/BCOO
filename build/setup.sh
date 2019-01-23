#!/bin/bash
#
# Creates a fullnode configuration
#

here=$(dirname "$0")
# shellcheck source=multinode-demo/common.sh
source "$here"/common.sh

usage () {
  exitcode=0
  if [[ -n "$1" ]]; then
    exitcode=1
    echo "Error: $*"
  fi
  cat <<EOF
usage: $0 [-n num_tokens] [-l] [-p] [-t node_type]

Creates a fullnode configuration

 -n num_tokens  - Number of tokens to create
 -l             - Detect network address from local machine configuration, which
                  may be a private IP address unaccessible on the Intenet (default)
 -p             - Detect public address using public Internet servers
 -t node_type   - Create configuration files only for this kind of node.  Valid
                  options are validator or leader.  Creates configuration files
                  for both by default

EOF
  exit $exitcode
}

#go build the binaries
 
eval $go_build_buffett_bench_tps
eval $go_build_buffett_wallet
eval $go_build_buffett_drone
eval $go_build_buffett_fullnode
eval $go_build_buffett_fullnode_config
eval $go_build_buffett_fullnode_cuda
eval $go_build_buffett_genesis
eval $go_build_buffett_keygen
echo $go_build_buffett_ledger_tool
eval $go_build_buffett_ledger_tool

echo "Building completed, should check the build result."

ip_address_arg=-l
num_tokens=1000000000
node_type_leader=true
node_type_validator=true
node_type_client=true
while getopts "h?n:lpt:" opt; do
  case $opt in
  h|\?)
    usage
    exit 0
    ;;
  l)
    ip_address_arg=-l
    ;;
  p)
    ip_address_arg=-p
    ;;
  n)
    num_tokens="$OPTARG"
    ;;
  t)
    node_type="$OPTARG"
    case $OPTARG in
    leader)
      node_type_leader=true
      node_type_validator=false
      node_type_client=false
      ;;
    validator)
      node_type_leader=false
      node_type_validator=true
      node_type_client=false
      ;;
    client)
      node_type_leader=false
      node_type_validator=false
      node_type_client=true
      ;;
    *)
      usage "Error: unknown node type: $node_type"
      ;;
    esac
    ;;
  *)
    usage "Error: unhandled option: $opt"
    ;;
  esac
done


set -e

for i in "$BUFFETT_CONFIG_DIR" "$BUFFETT_CONFIG_VALIDATOR_DIR" "$BUFFETT_CONFIG_PRIVATE_DIR"; do
  echo "Cleaning $i"
  rm -rvf "$i"
  mkdir -p "$i"
done

if $node_type_client; then
  client_id_path="$BUFFETT_CONFIG_PRIVATE_DIR"/client-id.json
  echo "Evoke keymaker to create some stuff on client"
  $buffett_keygen -o "$client_id_path"
  ls -lhR "$BUFFETT_CONFIG_PRIVATE_DIR"/
fi

if $node_type_leader; then
  echo "Evoke keymaker to create some stuff on leader"  
  leader_address_args=("$ip_address_arg")
  leader_id_path="$BUFFETT_CONFIG_PRIVATE_DIR"/leader-id.json
  mint_path="$BUFFETT_CONFIG_PRIVATE_DIR"/mint.json
  echo $leader_id_path
  $buffett_keygen -o "$leader_id_path"

  echo "Creating $mint_path with $num_tokens tokens"
  $buffett_keygen -o "$mint_path"

  echo "Creating $BUFFETT_CONFIG_DIR/ledger"
  $buffett_genesis --tokens="$num_tokens" --ledger "$BUFFETT_CONFIG_DIR"/ledger < "$mint_path"

  echo "Creating $BUFFETT_CONFIG_DIR/leader.json"
  $buffett_fullnode_config --keypair="$leader_id_path" "${leader_address_args[@]}" > "$BUFFETT_CONFIG_DIR"/leader.json

  ls -lhR "$BUFFETT_CONFIG_DIR"/
  ls -lhR "$BUFFETT_CONFIG_PRIVATE_DIR"/
fi


if $node_type_validator; then
  validator_address_args=("$ip_address_arg" -b 9000)
  validator_id_path="$BUFFETT_CONFIG_PRIVATE_DIR"/validator-id.json

  $buffett_keygen -o "$validator_id_path"

  echo "Creating $BUFFETT_CONFIG_VALIDATOR_DIR/validator.json"
  $buffett_fullnode_config --keypair="$validator_id_path" "${validator_address_args[@]}" > "$BUFFETT_CONFIG_VALIDATOR_DIR"/validator.json

  ls -lhR "$BUFFETT_CONFIG_VALIDATOR_DIR"/
fi
