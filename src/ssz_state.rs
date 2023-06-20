use crate::config::ConfigSpec;
use anyhow::{anyhow, Context, Result};
use byteorder::{ByteOrder, LittleEndian};
use bytes::{Buf, Bytes};
use std::ops::Range;

#[derive(Debug)]
pub struct StatePartial {
    pub slot: u64,
    pub previous_epoch_participation: Vec<u8>,
    pub current_epoch_participation: Vec<u8>,
    pub inactivity_scores: Vec<u64>,
}

// class BeaconState(Container):
//     # Versioning
//     genesis_time: uint64 - 8 bytes
//     genesis_validators_root: Root - 32 bytes
//     slot: Slot - 8 bytes
//     fork: Fork - 4+4+8 = 16 bytes
//     # History
//     latest_block_header: BeaconBlockHeader - 8+8+32+32+32 = 112 bytes
//     block_roots: Vector[Root, SLOTS_PER_HISTORICAL_ROOT] - 32*SLOTS_PER_HISTORICAL_ROOT
//     state_roots: Vector[Root, SLOTS_PER_HISTORICAL_ROOT] - 32*SLOTS_PER_HISTORICAL_ROOT
//     historical_roots: List[Root, HISTORICAL_ROOTS_LIMIT] - 4 bytes (offset)
//     # Eth1
//     eth1_data: Eth1Data - 32+8+32 = 72 bytes
//     eth1_data_votes: List[Eth1Data, EPOCHS_PER_ETH1_VOTING_PERIOD * SLOTS_PER_EPOCH] - 4 bytes (offset)
//     eth1_deposit_index: uint64 - 8 bytes
//     # Registry
//     validators: List[Validator, VALIDATOR_REGISTRY_LIMIT] - 4 bytes (offset)
//     balances: List[Gwei, VALIDATOR_REGISTRY_LIMIT] - 4 bytes (offset)
//     # Randomness
//     randao_mixes: Vector[Bytes32, EPOCHS_PER_HISTORICAL_VECTOR] - 32*EPOCHS_PER_HISTORICAL_VECTOR
//     # Slashings
//     slashings: Vector[Gwei, EPOCHS_PER_SLASHINGS_VECTOR] - 8*EPOCHS_PER_SLASHINGS_VECTOR
//     # Participation
//     previous_epoch_participation: List[ParticipationFlags, VALIDATOR_REGISTRY_LIMIT] - 4 bytes (offset)
//     current_epoch_participation: List[ParticipationFlags, VALIDATOR_REGISTRY_LIMIT] - 4 bytes (offset)
//     # Finality
//     justification_bits: Bitvector[JUSTIFICATION_BITS_LENGTH] - 1 byte
//     previous_justified_checkpoint: Checkpoint - 8+32 = 40 bytes
//     current_justified_checkpoint: Checkpoint - 8+32 = 40 bytes
//     finalized_checkpoint: Checkpoint - 8+32 = 40 bytes
//     # Inactivity
//     inactivity_scores: List[uint64, VALIDATOR_REGISTRY_LIMIT] - 4 bytes (offset)
//     # Sync
//     current_sync_committee: SyncCommittee  # [New in Altair]
//     next_sync_committee: SyncCommittee  # [New in Altair]

// const SLOTS_PER_HISTORICAL_ROOT: usize = usize::pow(2, 13);
// const EPOCHS_PER_HISTORICAL_VECTOR: usize = usize::pow(2, 16);
// const EPOCHS_PER_SLASHINGS_VECTOR: usize = usize::pow(2, 13);

pub fn deserialize_partial_state(config: &ConfigSpec, state: &Bytes) -> Result<StatePartial> {
    // Const derived from config
    let slot_offset = 8 + 32;
    let slot = read_u64(state, slot_offset).context("slot_offset out of bounds")?;
    let previous_epoch_participation_offset_offset = 8
        + 32  // genesis_validators_root
        + 8   // slot
        + 16  // fork
        + 112 // latest_block_header
        + 32 * config.slots_per_historical_root // block_roots
        + 32 * config.slots_per_historical_root // state_roots
        + 4   // historical_roots
        + 72  // eth1_data
        + 4   // eth1_data_votes
        + 8   // eth1_deposit_index
        + 4   // validators
        + 4   // balances
        + 32 * config.epochs_per_historical_vector // randao_mixes
        + 8 * config.epochs_per_slashings_vector; // slashings

    let current_epoch_participation_offset_offset = previous_epoch_participation_offset_offset + 4; // previous_epoch_participation

    let inactivity_scores_offset_offset = current_epoch_participation_offset_offset
        + 4   // current_epoch_participation
        + 1   // justification_bits
        + 40  // previous_justified_checkpoint
        + 40  // current_justified_checkpoint
        + 40; // finalized_checkpoint

    // Read offset values from state
    let previous_epoch_participation_offset =
        read_offset(state, previous_epoch_participation_offset_offset)
            .context("previous_epoch_participation_offset_offset out of bounds")?;
    let current_epoch_participation_offset =
        read_offset(state, current_epoch_participation_offset_offset)
            .context("current_epoch_participation_offset_offset out of bounds")?;
    let inactivity_scores_offset = read_offset(state, inactivity_scores_offset_offset)
        .context("inactivity_scores_offset_offset out of bounds")?;

    // Assume well-formed state, derive validator count from previous_epoch_participation size.
    // Altair state does not have any other variable size field after inactivity_scores, however Bellatrix state does.
    // So infering the size of inactivity_scores from previous_epoch_participation prevents this code from having
    // to be fork aware, for states after phase0.
    let validator_count = current_epoch_participation_offset - previous_epoch_participation_offset;

    // With offset values, read slices
    let previous_epoch_participation = slice(
        state,
        previous_epoch_participation_offset
            ..(previous_epoch_participation_offset + validator_count),
    )
    .context("previous_epoch_participation_offset out of bounds")?
    .to_vec();
    let current_epoch_participation = slice(
        state,
        current_epoch_participation_offset..(current_epoch_participation_offset + validator_count),
    )
    .context("current_epoch_participation_offset out of bounds")?
    .to_vec();
    let inactivity_scores = convert_u8_to_u64(
        &slice(
            state,
            inactivity_scores_offset..(inactivity_scores_offset + validator_count * 8),
        )
        .context("current_epoch_participation_offset out of bounds")?,
    );

    Ok(StatePartial {
        slot,
        previous_epoch_participation,
        current_epoch_participation,
        inactivity_scores,
    })
}

fn slice(buf: &Bytes, range: Range<usize>) -> Result<Bytes> {
    if range.end > buf.len() {
        return Err(anyhow!(
            "range end out of bounds: {} > {}",
            range.end,
            buf.len()
        ));
    }
    Ok(buf.slice(range))
}

fn read_offset(buf: &Bytes, offset_position: usize) -> Result<usize> {
    Ok(slice(buf, offset_position..offset_position + 4)?.get_u32_le() as usize)
}

fn read_u64(buf: &Bytes, offset: usize) -> Result<u64> {
    Ok(slice(buf, offset..(offset + 8))?.get_u64_le())
}

fn convert_u8_to_u64(input: &[u8]) -> Vec<u64> {
    let num_u64s = input.len() / 8;
    let mut output = vec![0u64; num_u64s];
    LittleEndian::read_u64_into(input, &mut output);
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use serde::Deserialize;
    use std::{error::Error, fs, str::FromStr};

    #[derive(Deserialize, Debug)]
    struct StateJsonStr {
        slot: String,
        previous_epoch_participation: Vec<String>,
        current_epoch_participation: Vec<String>,
        inactivity_scores: Vec<String>,
    }

    fn from_vec_str<T: FromStr>(vec_str: &[String]) -> Result<Vec<T>>
    where
        T::Err: Error + Send + Sync + 'static,
    {
        let mut vec_uint: Vec<T> = Vec::with_capacity(vec_str.len());
        for b in vec_str {
            vec_uint.push(b.parse()?);
        }
        Ok(vec_uint)
    }

    const CONFIG_GNOSIS: ConfigSpec = ConfigSpec {
        seconds_per_slot: 5,
        slots_per_epoch: 16,
        slots_per_historical_root: 8192,
        epochs_per_historical_vector: 65536,
        epochs_per_slashings_vector: 8192,
    };

    const CONFIG_MAINNET: ConfigSpec = ConfigSpec {
        seconds_per_slot: 12,
        slots_per_epoch: 32,
        slots_per_historical_root: 8192,
        epochs_per_historical_vector: 65536,
        epochs_per_slashings_vector: 8192,
    };

    #[test]
    fn devnet_state() {
        for (filename, config) in [
            ("src/fixtures/state_148990", CONFIG_GNOSIS),
            (
                "src/fixtures/state_devnet6_genesistime-1686904523_slot-416",
                CONFIG_MAINNET,
            ),
        ] {
            let state_json = fs::read_to_string(format!("{}.json", filename)).unwrap();
            let state_bytes = fs::read(format!("{}.ssz", filename)).unwrap();
            let state_json: StateJsonStr = serde_json::from_str(&state_json).unwrap();
            let state_buf = BytesMut::from_iter(state_bytes.iter()).freeze();
            let state = deserialize_partial_state(&config, &state_buf).unwrap();

            assert_eq!(
                state.slot,
                state_json.slot.parse::<u64>().unwrap(),
                "slot {}",
                filename
            );

            assert_eq!(
                hex::encode(state.previous_epoch_participation),
                hex::encode(from_vec_str::<u8>(&state_json.previous_epoch_participation).unwrap()),
                "previous_epoch_participation {}",
                filename
            );
            assert_eq!(
                hex::encode(state.current_epoch_participation),
                hex::encode(from_vec_str::<u8>(&state_json.current_epoch_participation).unwrap()),
                "current_epoch_participation {}",
                filename
            );
            assert_eq!(
                state.inactivity_scores,
                from_vec_str::<u64>(&state_json.inactivity_scores).unwrap(),
                "inactivity_scores {}",
                filename
            );
        }
    }
}
