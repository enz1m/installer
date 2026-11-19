//! Bounded calls to the live medium's network helper. Demo never calls it.
use enzui_services::process_transport::{ProcessRequest, StdinPolicy};
use serde_json::Value;
#[derive(Debug, Clone, Default)]
pub(crate) struct Device {
    pub(crate) device: String,
    pub(crate) kind: String,
    pub(crate) state: String,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct AccessPoint {
    pub(crate) ssid: String,
    pub(crate) bssid: String,
    pub(crate) device: String,
    pub(crate) signal: u64,
    pub(crate) security: String,
    pub(crate) active: bool,
    pub(crate) supported: bool,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct State {
    pub(crate) devices: Vec<Device>,
    pub(crate) wifi: Vec<AccessPoint>,
    pub(crate) radio: bool,
}
impl State {
    pub(crate) fn connected(&self) -> bool {
        self.devices.iter().any(|d| d.state == "connected")
    }
}
fn string(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or_default().into()
}
fn parse(v: Value) -> Result<State, String> {
    if let Some(e) = v["error"].as_str() {
        return Err(e.into());
    }
    if !v["devices"].is_array() || !v["wifi"].is_array() {
        return Err("Network service returned an invalid response.".into());
    }
    Ok(State {
        radio: v["radio"].as_bool().unwrap_or(false),
        devices: v["devices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| Device {
                device: string(d, "device"),
                kind: string(d, "kind"),
                state: string(d, "state"),
            })
            .collect(),
        wifi: v["wifi"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| AccessPoint {
                ssid: string(a, "ssid"),
                bssid: string(a, "bssid"),
                device: string(a, "device"),
                signal: a["signal"].as_u64().unwrap_or(0),
                security: string(a, "security"),
                active: a["active"].as_bool().unwrap_or(false),
                supported: a["supported"].as_bool().unwrap_or(false),
            })
            .collect(),
    })
}
pub(crate) async fn request(demo: bool, request: Value) -> Result<State, String> {
    if demo {
        tokio::time::sleep(std::time::Duration::from_millis(350)).await;
        let connected = matches!(request["action"].as_str(), Some("wifi" | "wired"));
        // Captures of the failure notice: a demo connection attempt fails.
        if connected && std::env::var_os("ENZIMOS_INSTALLER_DEMO_NETWORK_ERROR").is_some() {
            return Err("Network service is unavailable. You can continue offline.".into());
        }
        return parse(
            serde_json::json!({"radio":request["on"].as_bool().unwrap_or(true),"devices":[{"device":"wlan0","kind":"wifi","state":if connected {"connected"} else {"disconnected"},"connection":if connected {"Home Wi-Fi"} else {""}}],"wifi":[{"ssid":"Home Wi-Fi","bssid":"02:00:00:00:00:01","device":"wlan0","signal":92,"security":"WPA2","active":connected,"supported":true},{"ssid":"Guest network","bssid":"02:00:00:00:00:02","device":"wlan0","signal":65,"security":"--","active":false,"supported":true}]}),
        );
    }
    let mut request = request;
    let input = serde_json::to_vec(&request).map_err(|_| "Invalid network request.".to_owned())?;
    if let Some(Value::String(password)) = request.get_mut("password") {
        use zeroize::Zeroize;
        password.zeroize();
    }
    let request = ProcessRequest::new(
        "/usr/bin/sudo",
        StdinPolicy::Bytes(input),
        512 * 1024,
        16 * 1024,
        std::time::Duration::from_secs(20),
    )
    .args(["-n", "/usr/lib/enzimos-installer/ui-network.sh"])
    .inherit_environment();
    let output = crate::probe::run_async(request).await?;
    parse(
        serde_json::from_slice(&output.stdout)
            .map_err(|_| "Network service is unavailable. You can continue offline.".to_owned())?,
    )
}
