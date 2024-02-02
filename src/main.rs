//Example: how to access Uniswap V3 pair price on Rust using ethers.rs crate

use anyhow::Result;
use ethers::prelude::*;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::cast::ToPrimitive;
use std::env;
use std::sync::Arc;

//from docs: "Generates type-safe bindings to an Ethereum smart contract from its ABI."
//you can get the signatures of the functions for abigen from Uniswap documentation
//or from the contract source code, e.g. on Etherscan
//https://docs.uniswap.org/contracts/v3/reference/core/interfaces/pool/IUniswapV3PoolState
abigen!(
    IUniswapV3PoolState,
    "[function slot0() external view returns (uint160 sqrtPriceX96, int24 tick, uint16 observationIndex, uint16 observationCardinality, uint16 observationCardinalityNext, uint8 feeProtocol, bool unlocked)]"
);

//https://docs.uniswap.org/contracts/v3/reference/core/interfaces/IUniswapV3Factory
abigen!(IUniswapV3Factory,
    "[function getPool(address tokenA, address tokenB, uint24 fee) external view returns (address pool)]"
);

#[tokio::main]
async fn main() -> Result<()> {
    //set this environment variable to your Alchemy API key before running the example
    let alchemy_api_key = env::var("ALCHEMY_API_KEY")?;

    //WARNING: this is mainnet (though we are not transfering any funds here, just checking the price)
    //note: another provider except Alchemy can also be used
    let rpc_url = format!("https://eth-mainnet.g.alchemy.com/v2/{}", alchemy_api_key);
    let provider = Arc::new(Provider::try_from(rpc_url)?);

    //this is the address of UniswapV3Factory contract of Uniswap V3, can be found here:
    //https://docs.uniswap.org/contracts/v3/reference/deployments
    let factory_address = "0x1F98431c8aD98523631AE4a59f267346ea31F984".parse::<Address>()?;

    //this is the address of USDC on the mainnet
    let token0_address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse::<Address>()?;

    //this is the address of WETH (Wrapped Ether) on the mainnet
    let token1_address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse::<Address>()?;

    //from docs: "Returns the pool address for a given pair of tokens and a fee, or address 0 if it does not exist"
    //note: the pools in Uniswap V3 are distinguished not only by the pair, but also by the commission fee
    //in this case the fee is 30bps or 0.3%
    let pool_address = IUniswapV3Factory::new(factory_address, provider.clone())
        .get_pool(token0_address, token1_address, 3000)
        .await?;
    println!("result: {:?}", pool_address);

    //and slot_0 is the endpoint for the data we came for
    let slot0 = IUniswapV3PoolState::new(pool_address, provider)
        .slot_0()
        .await?;
    println!("slot0: {:?}", slot0);

    //32 because we need 32 bytes to encode U256
    let mut little_endian = [0; 32];
    slot0.0.to_little_endian(&mut little_endian);

    //the data we got earlier is sqrtPrice multiplied by 2^96, so we need to do some calculations
    let sqrt_price = BigRational::from_integer(BigInt::from_bytes_le(
        num_bigint::Sign::Plus,
        &little_endian,
    )) / (BigRational::new(2.into(), 1.into()).pow(96));
    println!("sqrt_price: {:?}", sqrt_price);

    //we want the price in USDC, so we divide by the sqrt price...
    //...and also we should take the digits in the contracts of USDC and WETH into account,
    //USDC has 6 digits, WETH has 18 digits (data available on Etherscan or from the token contract method)
    let price = BigRational::new(10.into(), 1.into()).pow(18 - 6) / sqrt_price.pow(2);

    //note: the displayed price is approximate
    println!("price: {:?}", price.to_f32().unwrap());

    Ok(())
}
