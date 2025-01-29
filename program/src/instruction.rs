//! Program instructions

use {
    crate::{error::VoteError, id},
    bytemuck::Pod,
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
    },
    spl_pod::bytemuck::{pod_from_bytes, pod_get_packed_len},
};

/// Instructions supported by the program
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, TryFromPrimitive, IntoPrimitive)]
pub enum VoteInstruction {
    /// TODO: fill in with actual representation and update comments
    NotarizationVote,
    /// TODO: fill in with actual representation and update comments
    SkipVote,
    /// TODO: fill in with actual representation and update comments
    FinalizationVote,
}

/// Utility function for encoding instruction data
#[allow(dead_code)]
pub(crate) fn encode_instruction<D: Pod>(
    accounts: Vec<AccountMeta>,
    instruction: VoteInstruction,
    instruction_data: &D,
) -> Instruction {
    let mut data = vec![u8::from(instruction)];
    data.extend_from_slice(bytemuck::bytes_of(instruction_data));
    Instruction {
        program_id: id(),
        accounts,
        data,
    }
}

/// Utility function for decoding just the instruction type
#[allow(dead_code)]
pub(crate) fn decode_instruction_type(input: &[u8]) -> Result<VoteInstruction, ProgramError> {
    if input.is_empty() {
        Err(ProgramError::InvalidInstructionData)
    } else {
        VoteInstruction::try_from(input[0]).map_err(|_| VoteError::InvalidInstruction.into())
    }
}

/// Utility function for decoding instruction data
#[allow(dead_code)]
pub(crate) fn decode_instruction_data<T: Pod>(input_with_type: &[u8]) -> Result<&T, ProgramError> {
    if input_with_type.len() != pod_get_packed_len::<T>().saturating_add(1) {
        Err(ProgramError::InvalidInstructionData)
    } else {
        pod_from_bytes(&input_with_type[1..])
    }
}
