use cosmwasm_std::{
    to_binary, from_binary, log, Api, Binary, Env, Extern, HandleResponse, HandleResult, InitResponse, Querier,
    StdError, StdResult, Storage, Uint128, HumanAddr, CanonicalAddr, CosmosMsg
};
use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage};

use crate::msg::{ConfigResponse, HandleMsg, HandleAnswer, HandleReceiveMsg, InitMsg, QueryMsg, ResponseStatus::Success};
use crate::state::{Config, save, load, may_load, remove, CONFIG_KEY, PRNG_SEED_KEY, PREFIX_ALIAS_TO_ADDR,
PREFIX_CUSTOM_ALIAS, PREFIX_TOKEN_CONTRACT_INFO};

use crate::rand::{sha_256, Prng};

use std::fmt::Write;

//Snip 20 usage
use secret_toolkit::{snip20::handle::{register_receive_msg,transfer_msg}};




/// pad handle responses and log attributes to blocks of 256 bytes to prevent leaking info based on
/// response size
pub const BLOCK_SIZE: usize = 256;



pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let config = Config {
        admin: deps.api.canonical_address(&msg.admin)?,
        active: true,
        fee: msg.fee.u128(),
        fee_decimals: msg.fee_decimals,

    };

    let prng_seed: Vec<u8> = sha_256(base64::encode(msg.entropy).as_bytes()).to_vec();

    save(&mut deps.storage, CONFIG_KEY, &config)?;
    save(&mut deps.storage, PRNG_SEED_KEY, &prng_seed)?;

    // Store sscrt in registered contracts
    let mut snip_contract_storage = PrefixedStorage::new(PREFIX_TOKEN_CONTRACT_INFO, &mut deps.storage);
    save(&mut snip_contract_storage, msg.sscrt_addr.0.as_bytes(), &msg.sscrt_hash)?;


    Ok(InitResponse {
        messages: vec![
            register_receive_msg(
                env.contract_code_hash,
                None,
                BLOCK_SIZE,
                msg.sscrt_hash,
                msg.sscrt_addr
            )?
        ],
        log: vec![],
    })
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Receive { sender, from, amount, msg } => receive(deps, env, sender, from, amount, msg),
        HandleMsg::RegisterToken { snip20_addr, snip20_hash } => register_token(deps, env, snip20_addr, snip20_hash),
        HandleMsg::ChangeFee { new_fee } => change_fee(deps, env, new_fee),
        HandleMsg::ChangeAdmin { new_admin } => change_admin(deps, env, new_admin),
    }
}




/// For receiving SNIP20s
pub fn receive<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    sender: HumanAddr,
    _from: HumanAddr,
    amount: Uint128,
    msg: Option<Binary>,
) -> HandleResult {


    let mut config: Config = load(&deps.storage, CONFIG_KEY)?;


    if let Some(bin_msg) = msg {
        match from_binary(&bin_msg)? {
            HandleReceiveMsg::ReceiveTokens {
                recipient,
            } => {
                send_funds(
                    deps,
                    env,
                    &mut config,
                    recipient,
                    amount,      
                )
            }
            HandleReceiveMsg::SetAlias {
                alias,
            } => {
                set_alias(
                    deps,
                    env,
                    &mut config,
                    alias,
                    sender   
                )
            }
        }
     } else {
        Err(StdError::generic_err("data should be given"))
     }
}





/// Allows user to send funds to an alias key
/// 
/// # Arguments
/// 
/// * `deps` - a mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `config` - a mutable reference to the Config
/// * `recipient` - alias of person recieving snip20 tokens
/// * `amount` - amount of snip20 tokens being sent
pub fn send_funds<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    config: &mut Config,
    recipient: String,
    amount: Uint128
) -> StdResult<HandleResponse> {


    if !config.active {
        return Err(StdError::generic_err(
            "Contract is currently disabled",
        ));
    }



    let mut msg_list: Vec<CosmosMsg> = vec![];


    // Finds hash associated with snip20 contract
    let snip_contract_storage = ReadonlyPrefixedStorage::new(PREFIX_TOKEN_CONTRACT_INFO, &mut deps.storage);
    let snip20_address: HumanAddr = env.message.sender;
    let callback_code_wrapped: Option<String> = may_load(&snip_contract_storage, snip20_address.0.as_bytes())?;
    let callback_code_hash: String;
    
    if callback_code_wrapped == None {
        return Err(StdError::generic_err(
            "This token is not registered with this contract. Please register it",
        ));
    }

    else {
        callback_code_hash = callback_code_wrapped.unwrap();
    }


    // Finds address paired to alias
    let alias_storage = ReadonlyPrefixedStorage::new(PREFIX_ALIAS_TO_ADDR, &mut deps.storage);
    let addr: HumanAddr;

    let addr_wrapped: Option<CanonicalAddr> = may_load(&alias_storage, recipient.clone().as_bytes())?;  
    if addr_wrapped == None {
        return Err(StdError::generic_err(
            "This alias is not linked to an address",
        ));
    }

    else {
        addr = deps.api.human_address(&addr_wrapped.unwrap())?;
    }





    let padding: Option<String> = None;
    let block_size = BLOCK_SIZE;

    // Fee calculation
    let decimal_places : u32 = config.fee_decimals.into();
    let rate :u128 = (config.fee) * (10 as u128).pow(decimal_places);
    let fee_amount = Uint128((amount.u128() * rate) / (100 as u128).pow(decimal_places));

    let remaining_amount = Uint128(amount.u128() - fee_amount.u128());



    // Send fee to admin
    let cosmos_msg = transfer_msg(
        deps.api.human_address(&config.admin)?,
        fee_amount,
        padding.clone(),
        block_size.clone(),
        callback_code_hash.clone(),
        snip20_address.clone(),
    )?;
    msg_list.push(cosmos_msg);



    // Send funds to recipient
    let cosmos_msg = transfer_msg(
        addr,
        remaining_amount,
        padding.clone(),
        block_size.clone(),
        callback_code_hash.clone(),
        snip20_address.clone(),
    )?;
    msg_list.push(cosmos_msg);





    Ok(HandleResponse {
        messages: msg_list,
        log: vec![],
        data: None,
    })
}






/// Allows user to set alias key to receive funds
/// 
/// # Arguments
/// 
/// * `deps` - a mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `config` - a mutable reference to the Config
/// * `alias` - optional custom alias for sender
/// * `sender` - address to be connected to the alias
pub fn set_alias<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    config: &mut Config,
    alias: Option<String>,
    sender: HumanAddr
) -> StdResult<HandleResponse> { 
    let sender_raw = deps.api.canonical_address(&sender)?;


    if !config.active {
        return Err(StdError::generic_err(
            "Contract is currently disabled",
        ));
    }


    //For output purposes on completion
    let mut alias_string = String::new();



    // If user elects to use a custom alias
    if alias != None {

        // Grabs old alias option to check if it exists later
        let mut custom_storage = PrefixedStorage::new(PREFIX_CUSTOM_ALIAS, &mut deps.storage);
        let old_alias: Option<String> = may_load(&custom_storage, sender.0.as_bytes())?;

        // Saves as addresses new custom alias
        save(&mut custom_storage, sender.0.as_bytes(), &alias.clone().unwrap())?;



        let mut alias_storage = PrefixedStorage::new(PREFIX_ALIAS_TO_ADDR, &mut deps.storage);

        // Tests to see if alias already has an address assigned
        let assigned_addr: Option<CanonicalAddr> = may_load(&alias_storage, alias.clone().unwrap().as_bytes())?;  
        if assigned_addr != None {
            return Err(StdError::generic_err(
                "This custom alias is unavailibe",
            ));
        }

        

        else {
            // Saves new custom key and remove old key if it exists
            save(&mut alias_storage, alias.clone().unwrap().as_bytes(), &sender_raw)?;


            //saves for output
            alias_string = alias.clone().unwrap();

            if old_alias != None {
                let old_alias_unwrapped = old_alias.unwrap();
                remove(&mut alias_storage, old_alias_unwrapped.as_bytes());
    
            }

        }


    }


    // If user wants a randomized alias
    else {
        let prng_seed: Vec<u8> = load(&mut deps.storage, PRNG_SEED_KEY)?;
        let mut alias_storage = PrefixedStorage::new(PREFIX_ALIAS_TO_ADDR, &mut deps.storage);

        let random_seed  = new_entropy(&env,prng_seed.as_ref(),prng_seed.as_ref());

        
        
        for byte in random_seed {
            write!(&mut alias_string, "{:x}", byte).unwrap();
        }
        



        //NOTE I TRIED USING RANDOM_SEED DIRECTLY AND SAVING IT

        save(&mut alias_storage, &random_seed, &sender_raw)?;


    }



    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("alias", &alias_string)
        ],
        data: Some(to_binary(&HandleAnswer::SetAlias { status: Success })?),
    })

}







/// Calls register_receive a snip20 token contract
/// and saves snip20 contract hash keyed to address
/// 
/// # Arguements
/// * `deps` - a mutable reference to Extern containing all the contract's external dependencies
/// * `env` - Env of contract's environment
/// * `snip20_addr` - address of the snip20 contract to be registered
/// * `snip20_hash` - contract callback hash of the snip20 contract
pub fn register_token<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    snip20_addr: HumanAddr,
    snip20_hash: String
) -> StdResult<HandleResponse> {


    
    if config.admin != sender_raw {
        return Err(StdError::generic_err(
            "This function is only usable by the Admin",
        ));
    }


    let mut snip_contract_storage = PrefixedStorage::new(PREFIX_TOKEN_CONTRACT_INFO, &mut deps.storage);
    
    save(&mut snip_contract_storage, snip20_addr.0.as_bytes(), &snip20_hash)?;


    Ok(HandleResponse {
        messages: vec![
            register_receive_msg(
                env.contract_code_hash,
                None,
                BLOCK_SIZE,
                snip20_hash,
                snip20_addr
            )?
        ],
        log: vec![],
        data: None,
    })
}





pub fn new_entropy(env: &Env, seed: &[u8], entropy: &[u8])-> [u8;32]{
    // 16 here represents the lengths in bytes of the block height and time.
    let entropy_len = 16 + env.message.sender.len() + entropy.len();
    let mut rng_entropy = Vec::with_capacity(entropy_len);
    rng_entropy.extend_from_slice(&env.block.height.to_be_bytes());
    rng_entropy.extend_from_slice(&env.block.time.to_be_bytes());
    rng_entropy.extend_from_slice(&env.message.sender.0.as_bytes());
    rng_entropy.extend_from_slice(entropy);

    let mut rng = Prng::new(seed, &rng_entropy);

    rng.rand_bytes()
}






// ADMIN COMMANDS -----------------------------------------------------------------------------------------------------------





pub fn change_fee<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    new_fee: Uint128,
) -> StdResult<HandleResponse> {
    let mut config: Config = load(&deps.storage, CONFIG_KEY)?;  
    let sender_raw = deps.api.canonical_address(&env.message.sender)?;

    if config.admin != sender_raw {
        return Err(StdError::generic_err(
            "This function is only usable by the Admin",
        ));
    }

    config.fee = new_fee.u128();

    save(&mut deps.storage, CONFIG_KEY, &config)?;


    Ok(HandleResponse::default())
}




pub fn change_admin<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    new_admin: HumanAddr
) -> StdResult<HandleResponse> {
    let mut config: Config = load(&deps.storage, CONFIG_KEY)?;
    let sender_raw = deps.api.canonical_address(&env.message.sender)?;

    if config.admin != sender_raw {
        return Err(StdError::generic_err(
            "This function is only usable by the Admin",
        ));
    }

    config.admin = deps.api.canonical_address(&new_admin)?;

    save(&mut deps.storage, CONFIG_KEY, &config)?;



    Ok(HandleResponse::default())
}






pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
    }
}

fn query_config<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<ConfigResponse> {
    let config: Config = load(&deps.storage, CONFIG_KEY)?;


    Ok(ConfigResponse { active: config.active, fee: Uint128(config.fee), decimals: config.fee_decimals })
}

