use anyhow::Result;
use std::path::Path;
use xbl_auth::{xbox_auth, AuthOption, Client, XSTSToken};
#[tokio::test]
async fn main() -> Result<()> {
    let client = Client::new();
    let xsts = xbox_auth(
        client.clone(),
        AuthOption {
            user_name: "uenomut384@gmail.com",
            cache_path: Path::new("./auth"),
        },
    )
    .await?;
    println!("{:?}", xsts);
    let XSTSToken {
        user_hash, xsts, ..
    } = xsts.get()?;
    let v = client
        .get("https://peoplehub.xboxlive.com/users/me/people/social/")
        .header("Authorization", format!("XBL3.0 x={};{}", user_hash, xsts))
        .header("x-xbl-contract-version", "5")
        .header("Accept-Language", "en")
        .send()
        .await?;
    println!(
        "{:?},{:?}",
        v.status(),
        v.json::<serde_json::Value>().await?
    );
    Ok(())
}
//https://peoplehub.xboxlive.com/users/me/people/social/decoration/broadcast,multiplayersummary,preferredcolor,socialManager,presenceDetail
