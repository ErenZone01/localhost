use std::net::{TcpListener, TcpStream};
use std::os::unix::io::{AsRawFd, RawFd};
use std::collections::HashMap;
use libc::{epoll_create1, epoll_ctl, epoll_wait, epoll_event, EPOLLIN, EPOLL_CTL_ADD};
use crate::config::{ServerConfig};

mod handler;
mod config;

fn main() {
    // Charge la configuration depuis le fichier config.toml
    let config = ServerConfig::from_file("src/config.toml");

    // Crée une instance epoll
    let epoll_fd = unsafe { epoll_create1(0) };
    if epoll_fd == -1 {
        panic!("Échec de la création de l'instance epoll");
    }

    // Map pour stocker les descripteurs de fichier des listeners et leurs indices
    let mut listeners: HashMap<RawFd, (TcpListener, usize)> = HashMap::new();

    // Boucle sur chaque serveur défini dans la configuration
    for (index, server) in config.servers.iter().enumerate() {
        let address = format!("{}:{}", server.address, server.port);
        let listener = TcpListener::bind(&address).expect("Échec de la liaison à l'adresse");
        println!(
            "Serveur à l'écoute sur {} avec le nom d'hôte {}",
            address, server.hostname
        );

        let listener_fd = listener.as_raw_fd();
        listeners.insert(listener_fd, (listener, index)); // On stocke l'index avec le listener

        // Enregistre le listener avec epoll
        let mut event = epoll_event {
            events: EPOLLIN as u32,
            u64: listener_fd as u64,
        };
        let result = unsafe { epoll_ctl(epoll_fd, EPOLL_CTL_ADD, listener_fd, &mut event) };
        if result == -1 {
            panic!("Échec de l'ajout du listener à epoll");
        }
    }

    // Boucle d'événements
    let mut events = vec![epoll_event { events: 0, u64: 0 }; 1024];
    loop {
        let num_events =
            unsafe { epoll_wait(epoll_fd, events.as_mut_ptr(), events.len() as i32, -1) };
        if num_events == -1 {
            panic!("Échec de l'attente des événements epoll");
        }

        for i in 0..num_events as usize {
            let event = events[i];
            let fd = event.u64 as RawFd;

            // Vérifie si l'événement correspond à un des listeners enregistrés
            if let Some((listener, server_index)) = listeners.get(&fd) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        // Passez l'index du serveur à handle_connection
                        handle_connection(stream, &config, *server_index);
                    }
                    Err(e) => {
                        println!("Échec de la connexion : {}", e);
                    }
                }
            }
        }
    }
}


// Fonction de gestion de la connexion, appelant la logique de traitement des requêtes
fn handle_connection(mut stream: TcpStream, config: &ServerConfig, server_index: usize) {
    handler::handle_request(&mut stream, config, server_index)
        .unwrap_or_else(|e| println!("Échec de la gestion de la requête : {}", e));
}

