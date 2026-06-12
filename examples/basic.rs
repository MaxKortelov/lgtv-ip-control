use lgtv_ip_control::LGTV;
use lgtv_ip_control::constants::types::Apps;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ip = "192.168.1.0";
    let mac_address = "3F:F6:83:8C:1C:E8";
    let key_code = Some("GHBFXMEZ");

    let mut tcp_connection = LGTV::connect_tcp(ip, mac_address, key_code).await?;

    tcp_connection.power_on(Some(10)).await?;

    // Make sure the TV is on before proceeding
    sleep(Duration::from_secs(5)).await;

    let app = tcp_connection.get_current_app().await?;
    let _result = tcp_connection.launch_app(Apps::Youtube).await;
    tcp_connection.power_off(None).await?;

    println!("{:?}", app);
    Ok(())
}
