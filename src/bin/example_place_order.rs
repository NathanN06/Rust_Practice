//Imports in Rust are Crate_name(Library)::Module(Folders)::Item(Struct)
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta},
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
    transaction::Transaction,
    commitment_config::CommitmentConfig,
};
use std::{rc::Rc, str::FromStr};
use anyhow::Result;
use anchor_lang::{AnchorSerialize, AnchorDeserialize};
use anchor_lang::solana_program::hash::hash;
use anchor_lang::solana_program::instruction::Instruction;


//Borrowable Constant Which Defines Drifts Program ID
//Tells Client Where to send tx 
const DRIFT_DEVNET_ID: &str = "dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH";

//Function to Derive Account PDA(Function Takes in a Pubic Key and Program ID and Returns a PDA and an 8 bit bump)
//PDA Seperate to Main Wallet Which allows Code to control it
fn derive_user_pda(program_id: &Pubkey, authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"user", authority.as_ref()], program_id)
}

//Function to Derive Stats PDA (Same functionality as derive_user)
fn derive_user_stats(program_id: &Pubkey, authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"user_stats", authority.as_ref()], program_id)
}

//Function to Derive Global State for Drift
//Stores All Data (Updated after a tx)
fn derive_state(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"state"], program_id).0
}

//Derives Market PDA for a Specific Crypto Exchange
fn derive_market(program_id: &Pubkey, market_index: u16) -> Pubkey {
    let bytes = market_index.to_le_bytes();
    Pubkey::find_program_address(&[b"perp_market", &bytes], program_id).0
}

//Maps Market Indexes to their Pubkeys Allows The Code To Fetch Real World Data
fn get_oracle_pubkey(market_index: u16) -> Pubkey {

    match market_index {
        0 => Pubkey::from_str("EdVCmQ9FSPcVe5YySXDPCRmc8aDQLKJ9xvYBMZPie1Vw").unwrap(), 
        1 => Pubkey::from_str("HovQMDrbAgAYPCmHVSrezcSmkMtXSSUsLDFANExrZh2J").unwrap(), 
        2 => Pubkey::from_str("CtJ8EkqLmeYyGB8PB2afdHDQYHE2a4Cbc4WLQoe8vFsP").unwrap(),
        _ => panic!("Not Supported market index: {}", market_index),
    }
}

//Macro Which Activates these Traits(AnchorSerialize, AnchorDeserialize) To The Struct
#[derive(AnchorSerialize, AnchorDeserialize)]
//Struct Which Defines WWhich Formats are used for each Instruction Param
pub struct PlaceOrderInstruction {
    pub order_type: u8,
    pub market_index: u16,
    pub direction: u8,
    pub base_asset_amount: u64,
    pub price: u64,
    pub reduce_only: bool,
    pub immediate_or_cancel: bool,
    pub post_only: bool,
}


//Main Function Which Takes no Input and Returns No Output and can Return any Error
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    
    //Set Destination To Drift by Changing Drift ID into A Public Key
    let program_id = Pubkey::from_str(DRIFT_DEVNET_ID)?;
    
    //Load KeyPair File
    let keypair = match read_keypair_file("drift-dev-wallet.json") {
    Ok(kp) => kp,
    Err(e) => {
        eprintln!("Failed to read keypair: {}", e);
        return Err(e.into());
    }
    };
    
    //Defining Payer(Signer) Using KeyPair. Rc is used so it can be cloned
    let payer = Rc::new(keypair);

    //Defining Retrieved User PDA (.0) Means Getting the Pubkey not bump
    let user_pda = derive_user_pda(&program_id, &payer.pubkey());
    println!("User PDA: {}", user_pda.0);
    
    //Defining Retrieved User Stats PDA 
    let user_stats_pda = derive_user_stats(&program_id, &payer.pubkey());
    println!("User Stats PDA: {}", user_stats_pda.0);
    
    //Defining Global Drift State
    let state = derive_state(&program_id);
    println!("State PDA: {}", state);
    
    //Define Which Market You Want To Use
    let market_index = 0;
    
    //Defining Market PDA
    let market = derive_market(&program_id, market_index);
    println!("Market PDA: {}", market);
    
    //Defining Market indexes 
    let oracle = get_oracle_pubkey(market_index);
    println!("Oracle: {} for market index {}", oracle, market_index);

    //Set Up Order Struct
    let order = PlaceOrderInstruction {
        order_type: 1,           
        market_index,           
        direction: 0,            
        base_asset_amount: 10_000, 
        price: 10_000_000,      
        reduce_only: false,
        immediate_or_cancel: false,
        post_only: true,        
    };

    
    //Hash The Instruction Name Taking The First 8 Bytes 
    let init_data = hash(b"global:initialize_user").to_bytes()[..8].to_vec();
    
    
    //Create Account Vector Params To Initialise In Drift
    let init_accounts = vec![
        AccountMeta::new(user_pda.0, false),
        AccountMeta::new(user_stats_pda.0, false),
        AccountMeta::new_readonly(state, false),
        AccountMeta::new_readonly(payer.pubkey(), true),
        AccountMeta::new_readonly(solana_program::system_program::ID, false),
    ];

    //Create Instruction Parameters 
    let init_instruction = Instruction {
        program_id,
        accounts: init_accounts,
        data: init_data,
    };
    
    //Create Order Parameters
    let order = PlaceOrderInstruction {
    order_type: 1,
    market_index,
    direction: 0,
    base_asset_amount: 10_000,
    price: 10_000_000,
    reduce_only: false,
    immediate_or_cancel: false,
    post_only: true,
    };

    //Hash Place Order 
    let mut order_data = hash(b"global:place_order").to_bytes()[..8].to_vec();
    
    //Serialise Order Parameters
    let mut order_serialized = order.try_to_vec()?;
    
    //Append Serialised order to OrderData
    order_data.append(&mut order_serialized);

    //Build Account "Vec![]" (List of Structs) and if the structs need to be signed
    let order_accounts = vec![
        AccountMeta::new(user_pda.0, false),    //User PDA      
        AccountMeta::new(user_stats_pda.0, false), //User Stats PDA
        AccountMeta::new_readonly(state, false),    //Global State PDA
        AccountMeta::new(market, false),    //Matket PDA
        AccountMeta::new_readonly(oracle, false),  //Oracle : Provides Real World Data from Market 
        AccountMeta::new_readonly(payer.pubkey(), true), //Defines That the Public Key Is Meant To Sign
        AccountMeta::new_readonly(solana_program::sysvar::rent::ID, false), //Provides the Rules For Account Creation
        AccountMeta::new_readonly(solana_program::system_program::ID, false), //Used by Program to know how much SOL is on your account
    ];
    
    //Create Instruction(Struct) For Solana
    let order_instruction = Instruction {
    program_id,
    accounts: order_accounts,
    data: order_data,
    };
    
    //Helper Comment(About to Send Tx)
    println!("Submitting transaction...");

    //Create an RPC(Rust CLient Struct that Talks to SOl) 
    //Tells Code to Send Requests To Devnet Address Only considered when Confirmed
    let rpc = RpcClient::new_with_commitment("https://api.devnet.solana.com".to_string(), CommitmentConfig::confirmed());
    
    //Get Latest Blockhash
    let blockhash = rpc.get_latest_blockhash()?;

    //Build Transaction using Instruction Struct Public key and blockhash
    let tx = Transaction::new_signed_with_payer(
    &[init_instruction, order_instruction],
    Some(&payer.pubkey()),
    &[payer.as_ref()],
    blockhash,
    );

    // Simulate Transaction
    let simulation = rpc.simulate_transaction(&tx)?;
    println!("Simulation logs:\n{:#?}", simulation.value.logs);

    //Send and confirm
    let sig = rpc.send_and_confirm_transaction(&tx)?;
    println!("Transaction sent! Signature: {}", sig);
    
    Ok(())
}