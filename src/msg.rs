use cosmwasm_std::{Binary, HumanAddr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};




#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    /// Recipient of fees and able to make adjustments
    pub admin: HumanAddr,

    /// Percentage taken from tx
    pub fee: Uint128,

    /// Decimals in fee
    pub fee_decimals: u8,


    pub sscrt_addr: HumanAddr,
    pub sscrt_hash: String,


    /// Entropy for PRNG
    pub entropy: String
}


#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleReceiveMsg {
    ReceiveTokens {
        recipient: String,
    },
    SetAlias {
        alias: Option<String>
    }
}





#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    /// Receive Snip20 Payment
    Receive {
        sender: HumanAddr,
        from: HumanAddr,
        amount: Uint128,
        #[serde(default)]
        msg: Option<Binary>,
    },
    RegisterToken {
        snip20_addr: HumanAddr,
        snip20_hash: String
    },
    ChangeFee {
        new_fee: Uint128,
    },
    ChangeAdmin {
        new_admin: HumanAddr,
    }
}



#[derive(Serialize, Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {
    SetAlias {
        status: ResponseStatus, 
    },
}








#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    GetConfig {},
    CheckAlias {
        alias: String
    },
}

// Returns config settings
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub active: bool,
    pub fee: Uint128,
    pub decimals: u8
}

// Returns if alias exists
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AliasCheckResponse {
    pub does_exist: bool
}




#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Success,
    Failure,
}