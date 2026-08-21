import re

with open(r"d:\Satyam (CE00172125)\learn\remote-control\apps\desktop\src-tauri\src\main.rs", "r") as f:
    text = f.read()

# 1. Add new commands to invoke_handler
text = text.replace(
    "terminate_host_session",
    "terminate_host_session,\n            toggle_auto_accept,\n            get_auto_accept"
)

# 2. Add the commands implementation
commands_impl = """
#[tauri::command]
async fn toggle_auto_accept(enabled: bool, session: tauri::State<'_, AppSession>) -> Result<(), String> {
    *session.auto_accept.write().await = enabled;
    Ok(())
}

#[tauri::command]
async fn get_auto_accept(session: tauri::State<'_, AppSession>) -> Result<bool, String> {
    Ok(*session.auto_accept.read().await)
}
"""
text = text.replace(
    "#[tauri::command]\nasync fn approve_connection",
    commands_impl + "\n#[tauri::command]\nasync fn approve_connection"
)

# 3. Extract the inner logic of approve_connection
# Find the start of inner logic
start_str = "    let offer_data = session.pending_offer.write().await.take();\n    if let Some((session_id, sdp)) = offer_data {\n        let c = client.inner().as_ref().unwrap().clone();\n        \n        let (ice_tx"
inner_start_idx = text.find(start_str)

end_str = "    }\n    \n    Ok(())\n}\n\n#[tauri::command]\nasync fn send_input_event"
inner_end_idx = text.find(end_str)

if inner_start_idx != -1 and inner_end_idx != -1:
    before = text[:inner_start_idx]
    
    # We want to extract what's inside the if let Some
    body_extracted = text[inner_start_idx + len("    let offer_data = session.pending_offer.write().await.take();\n    if let Some((session_id, sdp)) = offer_data {\n        let c = client.inner().as_ref().unwrap().clone();\n") : inner_end_idx]
    
    # Trim the trailing spaces and closing brace
    body_extracted = body_extracted.rstrip()
    if body_extracted.endswith("}"):
        body_extracted = body_extracted[:-1].rstrip()
    
    new_approve = """    let offer_data = session.pending_offer.write().await.take();
    if let Some((session_id, sdp)) = offer_data {
        let c = client.inner().as_ref().unwrap().clone();
        process_offer_internal(session.inner(), c, session_id, sdp).await?;
    }
    
    Ok(())
}

async fn process_offer_internal(
    session: &AppSession,
    c: networking::signaling::SignalingClient,
    session_id: String,
    sdp: String
) -> Result<(), String> {
""" + body_extracted + """
    Ok(())
}

#[tauri::command]
async fn send_input_event"""
    
    text = before + new_approve + text[inner_end_idx + len(end_str):]


# 4. Modify the SdpOffer event logic to check auto_accept
old_sdp = """                                    networking::signaling::SignalingEvent::SdpOffer { session_id, sdp } => {
                                        info!("Host received SDP Offer for session: {}", session_id);
                                        let state = app_handle_clone.state::<AppSession>();
                                        *state.pending_offer.write().await = Some((session_id, sdp));
                                    }"""

new_sdp = """                                    networking::signaling::SignalingEvent::SdpOffer { session_id, sdp } => {
                                        info!("Host received SDP Offer for session: {}", session_id);
                                        let state = app_handle_clone.state::<AppSession>();
                                        let is_auto = *state.auto_accept.read().await;
                                        if is_auto {
                                            info!("Auto-accepting connection request...");
                                            let c = client_clone.clone();
                                            let _ = process_offer_internal(state.inner(), c, session_id, sdp).await;
                                        } else {
                                            *state.pending_offer.write().await = Some((session_id, sdp));
                                        }
                                    }"""

text = text.replace(old_sdp, new_sdp)

with open(r"d:\Satyam (CE00172125)\learn\remote-control\apps\desktop\src-tauri\src\main.rs", "w") as f:
    f.write(text)
print("Done patching main.rs")
