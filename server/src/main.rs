use platformer_shared::{
    bevy::{self, app::ScheduleRunnerSettings, prelude::*},
    crystalorb_bevy_networking_turbulence::{
        bevy_networking_turbulence::{
            self, MessageChannelMode, MessageChannelSettings, NetworkResource,
            ReliableChannelSettings,
        },
        CommandChannelSettings, CrystalOrbServerPlugin,
    },
    game::GameWorld,
    SERVER_PORT,
};
use std::{net::SocketAddr, time::Duration};

fn main() {
    println!("Server starting");
    App::build()
        // You can optionally override some message channel settings
        // There is `CommandChannelSettings`, `SnapshotChannelSettings`, and `ClockSyncChannelSettings`
        // Make sure you apply the same settings for both client and server.
        .insert_resource(CommandChannelSettings(MessageChannelSettings {
            channel: 0,
            channel_mode: MessageChannelMode::Compressed {
                reliability_settings: ReliableChannelSettings {
                    bandwidth: 4096,
                    recv_window_size: 1024,
                    send_window_size: 1024,
                    burst_bandwidth: 1024,
                    init_send: 512,
                    wakeup_time: Duration::from_millis(100),
                    initial_rtt: Duration::from_millis(200),
                    max_rtt: Duration::from_secs(2),
                    rtt_update_factor: 0.1,
                    rtt_resend_factor: 1.5,
                },
                max_chunk_len: 1024,
            },
            message_buffer_size: 64,
            packet_buffer_size: 64,
        }))
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .add_plugins(MinimalPlugins)
        .add_plugin(CrystalOrbServerPlugin::<GameWorld>::new(
            platformer_shared::crystal_orb_config(),
        ))
        .add_plugin(bevy::log::LogPlugin)
        .add_startup_system(server_setup.system())
        .add_system(handle_events.system())
        .run();
}

fn server_setup(mut net: ResMut<NetworkResource>) {
    let ip_address =
        bevy_networking_turbulence::find_my_ip_address().expect("can't find ip address");
    let socket_address = SocketAddr::new(ip_address, SERVER_PORT);
    info!("Starting server on address {}", socket_address);
    net.listen(socket_address, None, None);
}

fn handle_events(
    mut event_reader: EventReader<bevy_networking_turbulence::NetworkEvent>,
    net: Res<NetworkResource>,
) {
    for event in event_reader.iter() {
        debug!("Got event: {:?}", event);
        match event {
            bevy_networking_turbulence::NetworkEvent::Connected(handle) => {
                let connection = net.connections.get(handle).unwrap();
                info!(
                    "Client connected: {:?} {}",
                    connection.remote_address(),
                    handle,
                );
            }
            bevy_networking_turbulence::NetworkEvent::Disconnected(handle) => {
                info!("Client disconnected: {:?}", handle);
            }
            bevy_networking_turbulence::NetworkEvent::Packet(_, _) => {}
            bevy_networking_turbulence::NetworkEvent::Error(handle, error) => {
                error!("Got error on handle {}: {:?}", handle, error);
            }
        }
    }
}
