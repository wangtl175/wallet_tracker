use alloy::eips::BlockId;
use alloy::primitives::{address, Address, map::AddressHashSet, TxHash};
use anyhow::Result;
use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use futures_util::StreamExt;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use lettre::transport::smtp::authentication::Credentials;
use std::collections::HashSet;

const EMAIL_SENDER: &str = "email sender";
const SMTP_SERVER: &str = "smtp server";
const EMAIL_PASSWORD: &str = "password";
const RPC_URL: &str = "websocket url to Ethereum";


static mut SMART_MONEYS: Option<&AddressHashSet> = None;
static mut EMAIL_RECEIVERS: Option<&HashSet<&str>> = None;

async fn send_email(smart_money: Address, tx_hash: TxHash) {
    if unsafe { EMAIL_RECEIVERS.is_none() } {
        return;
    }

    let creds = Credentials::new(EMAIL_SENDER.to_string(), EMAIL_PASSWORD.to_string());
    let mailer: AsyncSmtpTransport<Tokio1Executor> = AsyncSmtpTransport::<Tokio1Executor>::relay(SMTP_SERVER).unwrap().credentials(creds).build();

    let email_receivers = unsafe { EMAIL_RECEIVERS.unwrap() };
    for email_receiver in email_receivers.iter() {
        let email = Message::builder()
            .from(EMAIL_SENDER.parse().unwrap())
            .to(email_receiver.parse().unwrap())
            .subject("Smart Money Transaction")
            .body(format!("smart money {} has a transaction {}", smart_money, tx_hash))
            .unwrap();

        if let Err(e) = mailer.send(email).await {
            println!("Could not send email: {e:?}");
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let ws = WsConnect::new(RPC_URL);
    let provider = ProviderBuilder::new().on_ws(ws).await?;
    println!("connect to {RPC_URL}");

    let mut smart_moneys = Box::new(AddressHashSet::default());
    smart_moneys.insert(address!("aCab087f7f0977c31d68E8BAe117069a90Dc6574"));

    unsafe {
        SMART_MONEYS = Some(Box::leak(smart_moneys));
    }

    let mut email_receivers = Box::new(HashSet::new());
    email_receivers.insert("email receiver");

    unsafe {
        EMAIL_RECEIVERS = Some(Box::leak(email_receivers));
    }

    let sub = provider.subscribe_blocks().await?;
    let mut stream = sub.into_stream();

    while let Some(block) = stream.next().await {
        if block.header.number % 100 == 0 {
            println!("Received block number: {}", block.header.number);
        }

        let provider = provider.clone();

        tokio::spawn(async move {
            let transactions = provider.get_block_receipts(BlockId::from(block.header.hash)).await;
            if let Ok(Some(transactions)) = transactions {
                for transaction in transactions {
                    let is_smart_money = unsafe {
                        if let Some(smart_moneys) = SMART_MONEYS {
                            smart_moneys.contains(&transaction.from)
                        } else {
                            false
                        }
                    };

                    if is_smart_money {
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

    let mut email_receivers = Box::new(HashSet::new());
    email_receivers.insert("receiver1@gmail.com");
    email_receivers.insert("receiver2@gmail.com");

    unsafe {
        EMAIL_RECEIVERS = Some(Box::leak(email_receivers));
    }

    send_email(smart_money, tx_hash).await;
}
