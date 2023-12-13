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

// #[derive(BorshSerialize, BorshDeserialize, Debug)]
// enum VestingInstruction {
//     Initialize { owner: Pubkey },
// }


#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum VestingInstruction {
    Initialize { owner: Pubkey },
    CreateVestingSchedule { 
        user: Pubkey,
        amount: u64,
        start_date: u64,
        cliff: u64,
        duration: u64 
    }
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
    let instruction = VestingInstruction::try_from_slice(instruction_data)?;
    msg!("heyy");
    match instruction {
        VestingInstruction::Initialize {owner} => {
            let it: &mut std::slice::Iter<'_, AccountInfo<'_>> =&mut accounts.into_iter();
            let system = next_account_info( it)?;
            let rent = next_account_info( it)?;
            let clock = next_account_info( it)?;
            let signer = next_account_info( it)?;
            let state = next_account_info( it)?;        
            initialize(program_id, owner, rent, signer, state)
        },
        VestingInstruction::CreateVestingSchedule { user, amount, start_date, cliff, duration } =>
        create_vesting_schedule(program_id, accounts, user, amount, start_date, cliff, duration),
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

//The owner able to create new vesting schedules.
fn create_vesting_schedule(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    user: Pubkey,
    amount: u64,
    start_date: u64,
    cliff: u64,
    duration: u64,
) -> ProgramResult {
    // Ensure there are enough accounts passed to the function
    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

        // Unpack the accounts
    let accounts_iter = &mut accounts.iter();
    let state_account = next_account_info(accounts_iter)?; // State account holding owner info
    let vesting_account = next_account_info(accounts_iter)?; // Account to store vesting schedule
    let owner_account = next_account_info(accounts_iter)?; // Account of the owner

    // Verify the caller is the owner of the contract
    if state_account.owner != program_id || !owner_account.is_signer {
        return Err(ProgramError::IllegalOwner);
    }

    // Deserialize the state to access owner pubkey
    let state = State::try_from_slice(&state_account.try_borrow_data()?)?;

    if state.owner != *owner_account.key {
        return Err(ProgramError::IllegalOwner);
    }

    // Check if vesting schedule already exists for the user
    if vesting_account.owner == program_id {
        let existing_schedule_result = Vesting::try_from_slice(&vesting_account.try_borrow_data()?);

        if let Ok(existing_schedule) = existing_schedule_result{
            if existing_schedule.start_date != 0 {
                // User already has a vesting schedule, return an error or handle as per requirement
                return Err(ProgramError::AccountAlreadyInitialized);
            }
        } else {
            // Handle the case where deserialization fails (e.g., no data or invalid data)
            return Err(ProgramError::InvalidAccountData);
        }
    }

    // Create a new vesting schedule
    let new_vesting_schedule = Vesting {
        duration,
        amount,
        cliff,
        start_date,
    };

    // Serialize and save the new vesting schedule to the vesting account
    new_vesting_schedule.serialize(&mut *vesting_account.try_borrow_mut_data()?)?;
    
    Ok(())
}



// Sanity tests
#[cfg(test)]
mod test {
    use super::*;
    use solana_program::{clock::Epoch, instruction::AccountMeta};
    use solana_program_test::*;
    use solana_sdk::{
        account::Account,
        account_info::AccountInfo,
        pubkey::Pubkey,
        rent::Rent,
    };
    use std::str::FromStr;


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

        let accounts: Vec<AccountInfo<'_>> = vec![account.clone(),account.clone(),account.clone(),account.clone(), account.clone()];

        process_instruction(&program_id, &accounts, &instruction_data).unwrap();

        // assert_eq!(
        //     GreetingAccount::try_from_slice(&accounts[0].data.borrow())
        //         .unwrap()
        //         .counter,
        //     0
        // );
    }


    #[test]
    fn test_create_vesting_schedule() {
        let program_id = Pubkey::new_unique();
        let owner_key = Pubkey::new_unique();
        let user_key = Pubkey::new_unique();
        let state_key = Pubkey::new_unique();
        let vesting_account_key = Pubkey::new_unique();
        let mut lamports = Rent::default().minimum_balance(std::mem::size_of::<State>());
        let state = State {
            owner: owner_key,
            token: Pubkey::default(), // Assuming a default token for simplicity
        };

        let mut lamports_for_owner = 0;
        let mut lamports_for_vesting = Rent::default().minimum_balance(std::mem::size_of::<Vesting>());
        let mut lamports_for_state = Rent::default().minimum_balance(std::mem::size_of::<State>());
        let mut lamports_for_rent = 0;



        let mut state_account = Account::new(0, 0, &program_id);

        let mut vesting_account = Account::new(lamports, std::mem::size_of::<Vesting>(), &program_id);


        let rent_key = rent::id();
        let mock_rent = MockRent {
            lamports_per_byte_year: 3, // example value
            exemption_threshold: 2.0, // example value
            burn_percent: 10, // example value
        };
        let serialized_rent = mock_rent.try_to_vec().unwrap();
        let mut data = serialized_rent; // Now data contains a serialized Rent object     

        let system_rent_clock_account_info = AccountInfo::new(
            &rent_key,
            false,
            false,
            &mut lamports_for_rent,
            &mut data,
            &owner_key,
            true,
            Epoch::default(),
        );
    
        let owner_account_info = AccountInfo::new(
            &owner_key,
            true, // is_signer
            false, // is_writable
            &mut lamports_for_owner,
            &mut [], // data
            &program_id,
            false, // executable
            Epoch::default(),
        );

        let vesting_account_info = AccountInfo::new(
            &vesting_account_key,
            false, // is_signer
            true,  // is_writable
            &mut lamports_for_vesting,
            &mut vesting_account.data,
            &program_id,
            false, // executable
            Epoch::default(),
        );
        
        
        state.serialize(&mut state_account.data).unwrap();
        let state_date_len = state_account.data.len();
        let state_account_info = AccountInfo::new(
            &state_key,
            false, // is_signer
            true,  // is_writable
            &mut lamports_for_state,
            &mut state_account.data,
            &program_id,
            false, // executable
            Epoch::default(),
        );
        
        assert_eq!(state_date_len, std::mem::size_of::<State>());

        let accounts = vec![state_account_info.clone(), vesting_account_info.clone(), owner_account_info.clone()];

        let amount = 1000;
        let start_date = 1234567890;
        let cliff = 3600; // 1 hour
        let duration = 86400; // 1 day


        // Prepare instruction data for create_vesting_schedule
        let instruction_data = VestingInstruction::CreateVestingSchedule {
            user: user_key,
            amount,
            start_date,
            cliff,
            duration,
        }
        .try_to_vec().unwrap();

        // Simulate the process_instruction call
        process_instruction(
            &program_id,
            &accounts,
            &instruction_data,
        )
        .unwrap();

        let vesting_data = Vesting::try_from_slice(&vesting_account.data).unwrap();
        assert_eq!(vesting_data.amount, amount);
        assert_eq!(vesting_data.start_date, start_date);
        assert_eq!(vesting_data.cliff, cliff);
        assert_eq!(vesting_data.duration, duration);
    }
    
}
