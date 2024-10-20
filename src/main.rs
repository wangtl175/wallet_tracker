use std::sync::Arc;
use alloy::eips::BlockId;
use alloy::primitives::{address, Address, map::AddressHashSet, TxHash};
use anyhow::Result;
use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use futures_util::StreamExt;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use lettre::transport::smtp::authentication::Credentials;
use tokio::sync::Mutex;

const EMAIL_RECEIVER: &str = "email receiver";
const EMAIL_SENDER: &str = "email sender";
const SMTP_SERVER: &str = "smtp server";
const EMAIL_PASSWORD: &str = "password";


async fn send_email(smart_money: Address, tx_hash: TxHash) {
    let email = Message::builder()
        .from(EMAIL_SENDER.parse().unwrap())
        .to(EMAIL_RECEIVER.parse().unwrap())
        .subject("Smart Money Transaction")
        .body(format!("smart money {} has a transaction {}", smart_money, tx_hash))
        .unwrap();

    let creds = Credentials::new(EMAIL_SENDER.to_string(), EMAIL_PASSWORD.to_string());

    let mailer: AsyncSmtpTransport<Tokio1Executor> = AsyncSmtpTransport::<Tokio1Executor>::relay(SMTP_SERVER).unwrap().credentials(creds).build();

    if let Err(e) = mailer.send(email).await {
        println!("Could not send email: {e:?}");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let rpc_url = "wss://eth-mainnet.g.alchemy.com/v2/LttRLGRc066mddQWggs0tttdq9FwmTZh";  // mainnet
    let ws = WsConnect::new(rpc_url);
    let provider = ProviderBuilder::new().on_ws(ws).await?;
    let mut smart_moneys = AddressHashSet::default();
    smart_moneys.insert(address!("aCab087f7f0977c31d68E8BAe117069a90Dc6574"));
    let smart_moneys = Arc::new(smart_moneys);

    println!("connect to {rpc_url}");

    let sub = provider.subscribe_blocks().await?;
    let mut stream = sub.into_stream();

    let provider = Arc::new(Mutex::new(provider));

    while let Some(block) = stream.next().await {
        if block.header.number % 100 == 0 {
            println!("Received block number: {}", block.header.number);
        }

        let provider = provider.clone();
        let smart_moneys = smart_moneys.clone();

        tokio::spawn(async move {
            let provider = provider.lock().await;

            let transactions = provider.get_block_receipts(BlockId::from(block.header.hash)).await;
            if let Ok(Some(transactions)) = transactions {
                for transaction in transactions {
                    if smart_moneys.contains(&transaction.from) {
                        println!("smart money {} has a transaction {}", transaction.from, transaction.transaction_hash);

                        send_email(transaction.from, transaction.transaction_hash).await;
                    }
                }
            }
        });
    }

    Ok(())
}

#[tokio::test]
async fn test_send_email() {
    let smart_money = address!("aCab087f7f0977c31d68E8BAe117069a90Dc6574");
    let tx_hash = [6u8; 32];
    let tx_hash = TxHash::from(&tx_hash);
    send_email(smart_money, tx_hash).await;
}
