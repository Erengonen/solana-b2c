use std::convert::TryInto;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    sysvar::{rent, Sysvar},
    program::invoke_signed,
    system_instruction,
    system_program,
    program_error::ProgramError,
    pubkey::Pubkey, rent::Rent,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct State {
    pub owner: Pubkey,
    pub token: Pubkey,    
}

// user => vesting
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Vesting {
    pub duration: u64,
    pub amount: u64,
    pub cliff: u64,
    pub start_date: u64
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
enum VestingInstruction {
    Initialize { owner: Pubkey },
}

//
// Linear, One Token, Flexible for owner
// 
// Vesting:
// - Duration
// - Amount
// - Cliff - time
// - Start date
// 
// 
// Accounts
// - Program
// - State
// - (Single) Vault for the vesting token
// - (Multiple) Vesting data per user
// 

// Declare and export the program's entrypoint
entrypoint!(process_instruction);

// Program entrypoint's implementation
pub fn process_instruction(
    program_id: &Pubkey, // Public key of the account the hello world program was loaded into
    accounts: &[AccountInfo], // The account to say hello to
    instruction_data: &[u8], // Ignored, all helloworld instructions are hellos
) -> ProgramResult {

    let it =&mut accounts.into_iter();
    let system = next_account_info( it)?;
    let rent = next_account_info( it)?;
    let clock = next_account_info( it)?;
    let signer = next_account_info( it)?;
    let state = next_account_info( it)?;

    let instruction = VestingInstruction::try_from_slice(instruction_data)?;
    
    match instruction {
        VestingInstruction::Initialize {owner} => initialize(program_id, owner, rent, signer, state),
        _ => unreachable!(),
    }

    // msg!("Hello World Rust program entrypoint");

    // // Iterating accounts is safer than indexing
    // let accounts_iter = &mut accounts.iter();

    // // Get the account to say hello to
    // let account = next_account_info(accounts_iter)?;

    // // The account must be owned by the program in order to modify its data
    // if account.owner != program_id {
    //     msg!("Greeted account does not have the correct program id");
    //     return Err(ProgramError::IncorrectProgramId);
    // }

    // // Increment and store the number of times the account has been greeted
    // let mut greeting_account = GreetingAccount::try_from_slice(&account.data.borrow())?;
    // greeting_account.counter += 1;
    // greeting_account.serialize(&mut *account.try_borrow_mut_data()?)?;

    // msg!("Greeted {} time(s)!", greeting_account.counter);
}

const STATE_SEED: &[u8] = "STATE".as_bytes();

fn initialize<'a>(program_id: &Pubkey, owner: Pubkey, rent_info: &AccountInfo<'a>, signer: &AccountInfo<'a>, state: &AccountInfo<'a>) -> ProgramResult {

    let (account_address, bump) = Pubkey::find_program_address(&[STATE_SEED], &program_id);

    let space= std::mem::size_of::<State>();
    msg!("fff{:?} {:?}",rent_info,rent::check_id(rent_info.key));
    let rent = &Rent::from_account_info(rent_info)?;

    msg!("fff");
    let pay = rent.minimum_balance(space);
    msg!("minumum balance passed");
    invoke_signed(
        &system_instruction::create_account(
            &owner,
            &account_address,
            pay,
            space as u64,
            &system_program::ID
        ),
        &[signer.clone(), state.clone()],
        &[&[&STATE_SEED, &[bump]]],
    )?;
    Ok(())
}
// Sanity tests
#[cfg(test)]
mod test {
    use super::*;
    use solana_program::{clock::Epoch, instruction::AccountMeta};
    use solana_sdk::account;
    use std::mem;

    #[derive(BorshSerialize, BorshDeserialize)]
    struct MockRent {
        lamports_per_byte_year: u64,
        exemption_threshold: f64,
        burn_percent: u8,
    }

    #[test]
    fn test_sanity() {
        let program_id = Pubkey::default();
        let key = rent::id();
        let mut lamports = 0;
        let mock_rent = MockRent {
            lamports_per_byte_year: 3, // example value
            exemption_threshold: 2.0, // example value
            burn_percent: 10, // example value
        };
        let serialized_rent = mock_rent.try_to_vec().unwrap();
        let mut data = serialized_rent; // Now data contains a serialized Rent object        
        let owner = Pubkey::default();
        let account = AccountInfo::new(
            & key,
            false,
            false,
            &mut lamports,
            &mut data,
            &owner,
            true,
            Epoch::default(),
        );
    
        let instruction_data: Vec<u8> = VestingInstruction::Initialize { owner:Pubkey::default() }.try_to_vec().unwrap();

        let accounts = vec![account.clone(),account.clone(),account.clone(),account.clone(), account.clone()];

        process_instruction(&program_id, &accounts, &instruction_data).unwrap();

        // assert_eq!(
        //     GreetingAccount::try_from_slice(&accounts[0].data.borrow())
        //         .unwrap()
        //         .counter,
        //     0
        // );
    }
}
