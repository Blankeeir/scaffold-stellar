#![allow(non_upper_case_globals)]
use crate::storage::Storage;
use crate::ContractArgs;
use crate::ContractClient;
use admin_sep::Administratable;
use soroban_sdk::{
    self, assert_with_error, contractimpl, contracttype, symbol_short, vec, Address, BytesN, Env,
    IntoVal, InvokeError, String, Symbol,
};

use crate::{
    error::Error,
    name::canonicalize,
    util::{hash_string, MAX_BUMP, REGISTRY},
    Contract,
};

use super::{IsDeployable, IsRedeployable};

#[contracttype]
pub struct DeployEventData {
    wasm_name: String,
    contract_name: String,
    version: String,
    deployer: Address,
    contract_id: Address,
}

impl Contract {
    fn get(env: &Env, contract_name: &String) -> Result<Address, Error> {
        Storage::new(env)
            .contract
            .get(contract_name)
            .ok_or(Error::NoSuchContractDeployed)
    }

    fn upgrade(
        env: &Env,
        name: &String,
        wasm_hash: &BytesN<32>,
        upgrade_fn: Option<Symbol>,
    ) -> Result<Address, Error> {
        let name = canonicalize(name)?;
        let contract_id = Self::get(env, &name)?;
        Storage::new(env)
            .contract
            .extend_ttl(&name, MAX_BUMP, MAX_BUMP);
        if let Ok(Ok(author)) = env.try_invoke_contract::<Address, Error>(
            &contract_id,
            &symbol_short!("admin"),
            vec![&env],
        ) {
            author.require_auth();
        }
        let fn_name = upgrade_fn.unwrap_or_else(|| symbol_short!("upgrade"));
        let val = wasm_hash.into_val(env);
        let _ = env
            .try_invoke_contract::<(), InvokeError>(&contract_id, &fn_name, vec![&env, val])
            .map_err(|_| Error::UpgradeInvokeFailed)?;
        Ok(contract_id)
    }
}

#[contractimpl]
impl IsDeployable for Contract {
    fn deploy(
        env: &Env,
        wasm_name: String,
        version: Option<String>,
        contract_name: String,
        admin: Address,
        init: Option<soroban_sdk::Vec<soroban_sdk::Val>>,
    ) -> Result<Address, Error> {
        let contract_name = canonicalize(&contract_name)?;
        let mut contract_map = Storage::new(env).contract;
        if contract_map.has(&contract_name) {
            return Err(Error::AlreadyDeployed);
        }
        let str = soroban_sdk::String::from_str(env, REGISTRY);
        if contract_name == str {
            assert_with_error!(env, Self::admin(env) == admin, Error::AdminOnly);
        }
        // signed by admin
        admin.require_auth();

        let hash = Self::get_hash_and_bump(env, &wasm_name, version.clone())?;
        let salt: BytesN<32> = hash_string(env, &contract_name).into();
        let contract_id = deploy_and_init(env, salt, hash, init);

        contract_map.set(&contract_name, &contract_id);

        let version = Self::get_version(env, &wasm_name, version)?;
        // Publish a deploy event
        let deploy_data = DeployEventData {
            wasm_name,
            contract_name,
            version,
            deployer: admin,
            contract_id: contract_id.clone(),
        };
        #[allow(deprecated)] // TODO
        env.events()
            .publish((symbol_short!("deploy"),), deploy_data);
        Ok(contract_id)
    }

    fn fetch_contract_id(env: &Env, contract_name: String) -> Result<Address, Error> {
        let contract_name = canonicalize(&contract_name)?;
        Self::get(env, &contract_name)
    }
}

fn deploy_and_init(
    env: &Env,
    salt: impl IntoVal<Env, BytesN<32>>,
    wasm_hash: BytesN<32>,
    args: Option<soroban_sdk::Vec<soroban_sdk::Val>>,
) -> Address {
    let deployer = env.deployer().with_current_contract(salt.into_val(env));
    if let Some(args) = args {
        deployer.deploy_v2(wasm_hash, args)
    } else {
        deployer.deploy_v2(wasm_hash, ())
    }
}

#[contractimpl]
impl IsRedeployable for Contract {
    fn dev_deploy(
        env: &Env,
        name: soroban_sdk::String,
        wasm: soroban_sdk::Bytes,
        upgrade_fn: Option<soroban_sdk::Symbol>,
    ) -> Result<soroban_sdk::Address, Error> {
        let wasm_hash = env.deployer().upload_contract_wasm(wasm);
        Self::upgrade(env, &name, &wasm_hash, upgrade_fn)
    }

    fn upgrade_contract(
        env: &Env,
        name: String,
        wasm_name: String,
        version: Option<String>,
        upgrade_fn: Option<Symbol>,
    ) -> Result<Address, Error> {
        let wasm_hash = Self::get_hash_and_bump(env, &wasm_name, version)?;
        Self::upgrade(env, &name, &wasm_hash, upgrade_fn)
    }
}
