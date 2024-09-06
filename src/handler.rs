use std::collections::HashMap;
use std::fs::{self, File, DirEntry};
use std::io::{self, Read, Write};
use std::net::TcpStream;

use std::path::Path;


use crate::config::ServerConfig;

#[derive(Debug, Default)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

pub struct Response {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status_code: u16, body: Vec<u8>) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Length".to_string(), body.len().to_string());
        headers.insert("Connection".to_string(), "close".to_string());

        Response {
            status_code,
            headers,
            body,
        }
    }

    pub fn send(&self, stream: &mut TcpStream) -> io::Result<()> {
        let response_line = format!("HTTP/1.1 {} OK\r\n", self.status_code);
        let headers: String = self
            .headers
            .iter()
            .map(|(k, v)| format!("{}: {}\r\n", k, v))
            .collect();
        let response = format!("{}{}\r\n", response_line, headers);

        stream.write_all(response.as_bytes())?;
        stream.write_all(&self.body)?;
        stream.flush()
    }
}

pub fn handle_request(stream: &mut TcpStream, config: &ServerConfig,  server_index: usize) -> io::Result<()> {
    let (request, _buffer) = read_request(stream)?;
    // Vérifier si la méthode est autorisée
    if !is_method_allowed(&request.method, &config.route.default.methods) {
        let response = Response::new(405, handle_error(&config.error_pages.error_405, config, server_index));
        return response.send(stream);
    }

    handle_http_methods(stream, &request, config, server_index)?;
    Ok(())
}

pub fn read_request(stream: &mut TcpStream) -> io::Result<(Request, String)> {
    let mut buffer = String::new();
    let mut read_buf = [0; 2048];
    let mut headers = HashMap::new();
    let mut body = Vec::new();
    let mut content_length = 0;
    let mut method = String::new();
    let mut path = String::new();

    // Lire en utilisant directement TcpStream
    loop {
        let bytes_read = stream.read(&mut read_buf)?;

        if bytes_read == 0 {
            break; // Connexion fermée
        }

        // Ajouter les données lues au buffer
        buffer.push_str(&String::from_utf8_lossy(&read_buf[..bytes_read]));

        // Vérifier si l'en-tête complet a été lu
        if let Some(pos) = buffer.find("\r\n\r\n") {
            let header_part = &buffer[..pos];
            let body_part = &buffer[pos + 4..]; // 4 est la longueur de "\r\n\r\n"

            // Traiter les en-têtes
            let mut lines = header_part.lines();
            if let Some(first_line) = lines.next() {
                // Analyse de la ligne de demande (ex: "GET /path HTTP/1.1")
                let mut parts = first_line.split_whitespace();
                method = parts.next().unwrap_or("").to_string();
                path = parts.next().unwrap_or("").to_string();

                // Lire les en-têtes
                for line in lines {
                    if let Some((key, value)) = line.split_once(':') {
                        headers.insert(key.trim().to_string(), value.trim().to_string());
                    }
                }
            }

            // Vérifier si le corps est présent et obtenir la longueur
            if let Some(length) = headers.get("Content-Length") {
                content_length = length.parse().unwrap_or(0);
            }

            // Ajouter les parties du corps déjà lues
            body.extend_from_slice(body_part.as_bytes());

            // Lire le corps restant si nécessaire
            while body.len() < content_length {
                let mut buffer = vec![0; content_length - body.len()];
                let bytes_read = stream.read(&mut buffer)?;
                if bytes_read == 0 {
                    break; // Connexion fermée prématurément
                }
                body.extend_from_slice(&buffer[..bytes_read]);
            }

            break; // Sortir de la boucle après avoir lu le corps complet
        }
    }

    let request = Request {
        method: method.clone(),
        path,
        headers: headers.clone(),
        body: body.clone(),
    };

    Ok((request, buffer))
}

fn handle_http_methods(
    stream: &mut TcpStream,
    request: &Request,
    config: &ServerConfig,
    server_index: usize,
) -> io::Result<()> {
    if request.path.contains(".sh") || request.path.contains(".py") {
        handle_cgi_request(stream, &request, config, server_index)?;
    } else {
        match request.method.as_str() {
            "GET" => handle_get_request(stream, &request, config, server_index)?,
            "POST" => handle_post_request(stream, &request, config, server_index)?,
            "DELETE" => handle_delete_request(stream, &request, config, server_index)?,
            _ => {
                let response =
                    Response::new(405, handle_error(&config.error_pages.error_405, config, server_index));
                response.send(stream)?;
            }
        }
    }
    Ok(())
}

// fn handle_get_request(
//     stream: &mut TcpStream,
//     request: &Request,
//     config: &ServerConfig,
// ) -> io::Result<()> {
//     // Construire le chemin du fichier demandé
//     //let mut requested_path_route = String::new();
//     let requested_path = if request.path == "/"
//         || request.path == format!("/{}/{}", config.server.root, config.server.default_file)
//     {
//         format!("{}/{}", config.server.root, config.server.default_file)
//     } else {
//         if request.path.trim_start_matches("/")
//             == config.route.default.root_directory.trim_start_matches("/")
//         {
//             format!("{}/{}", request.path, config.route.default.default)
//         } else {// =========================================================Il reste l'implementation des autre route====================================================
//             format!("{}", request.path)
//         }
//     };

//     println!("1{}", requested_path.trim_start_matches('/'));
//     //println!("2{}",requested_path_route);

//     // Vérifier si le fichier est un fichier CSS
//     let content_type = if requested_path.ends_with(".css") {
//         "text/css"
//     } else {
//         "text/html"
//     };

//     // Tenter de lire le fichier demandé
//     let content = match fs::read(&requested_path.trim_start_matches('/')) {
//         Ok(file_content) => file_content, // Si le fichier est trouvé, on le sert
//         Err(_) => {
//             // Si le fichier n'est pas trouvé, rediriger vers la page d'erreur 404 ou 500
//             let error_path = if request.path.contains("404") || !fs::read(&requested_path).is_ok() {
//                 format!("{}/{}", config.server.root, config.error_pages.error_404)
//             } else {
//                 format!("{}/{}", config.server.root, config.error_pages.error_500)
//             };
//             return Response::new(404, handle_error(&error_path, config)).send(stream);

//             //handle_error(&error_path, config)
//         }
//     };

//     let mut response = Response::new(200, content);
//     response
//         .headers
//         .insert("Content-Type".to_string(), content_type.to_string());
//     response.send(stream)
// }

// Ajoutez cette fonction utilitaire pour lister les fichiers d'un répertoire
fn list_directory_contents(path: &Path) -> io::Result<Vec<DirEntry>> {
    let mut entries = fs::read_dir(path)?
        .filter_map(|res| res.ok())
        .collect::<Vec<_>>();
    entries.sort_by_key(|dir| dir.path());
    Ok(entries)
}
fn handle_get_request(
    stream: &mut TcpStream,
    request: &Request,
    config: &ServerConfig,
    server_index: usize,
) -> io::Result<()> {
    // Exemple d'accès à un serveur spécifique, modifiez selon votre structure
    let current_server = &config.servers.get(0).expect("No server found"); // Remplacez `0` par la logique appropriée

    // Construire le chemin du fichier demandé
    let requested_path = if request.path == "/"
        || request.path == format!("/{}/{}", current_server.root, current_server.default_file)
    {
        format!("{}/{}", current_server.root, current_server.default_file)
    } else if request.path.trim_start_matches("/")
        == config.route.default.root_directory.trim_start_matches("/")
    {
        format!("{}/{}", request.path, config.route.default.default)
    } else {
        format!("{}", request.path)
    };

    println!("1{}", requested_path.trim_start_matches('/'));

    // Chemin absolu basé sur le répertoire racine du serveur
    let absolute_path = Path::new(requested_path.trim_start_matches('/'));

    // Vérifier si le chemin est un répertoire
    if absolute_path.is_dir() {
        let entries = list_directory_contents(absolute_path)?;
        
        let mut html_content = String::from("<html><body><h1>Index of ");
        html_content.push_str(&request.path);
        html_content.push_str("</h1><ul>");

        for entry in entries {
            let file_name = entry.file_name().into_string().unwrap_or_default();
            let file_path = format!("{}/{}", request.path.trim_end_matches('/'), file_name);
            html_content.push_str(&format!(
                "<li><a href=\"{}\">{}</a></li>",
                file_path, file_name
            ));
        }

        html_content.push_str("</ul></body></html>");
        
        let mut response = Response::new(200, html_content.into_bytes());
        response.headers.insert("Content-Type".to_string(), "text/html".to_string());
        return response.send(stream);
    }

    // Vérifier si le fichier est un fichier CSS
    let content_type = if requested_path.ends_with(".css") {
        "text/css"
    } else {
        "text/html"
    };

    // Tenter de lire le fichier demandé
    let content = match fs::read(&requested_path.trim_start_matches('/')) {
        Ok(file_content) => file_content,
        Err(_) => {
            // Gestion des erreurs basée sur le serveur actuel
            let error_path = if request.path.contains("404") || !fs::read(&requested_path).is_ok() {
                format!("{}/{}", current_server.root, config.error_pages.error_404)
            } else {
                format!("{}/{}", current_server.root, config.error_pages.error_500)
            };
            return Response::new(404, handle_error(&error_path, config, server_index)).send(stream);
        }
    };

    let mut response = Response::new(200, content);
    response.headers.insert("Content-Type".to_string(), content_type.to_string());
    response.send(stream)
}


fn handle_post_request(
    stream: &mut TcpStream,
    request: &Request,
    config: &ServerConfig,
    server_index: usize,
) -> io::Result<()> {
    let upload_dir = "src/uploads";

    if let Some(content_type) = request.headers.get("Content-Type") {
        if content_type.starts_with("multipart/form-data") {
            if let Some(boundary) = content_type.split("boundary=").nth(1) {
                let boundary = format!("--{}", boundary);
                let boundary_bytes = boundary.as_bytes();
                let mut parts = Vec::new();
                let mut start = 0;

                while let Some(pos) = request.body[start..]
                    .windows(boundary_bytes.len())
                    .position(|w| w == boundary_bytes)
                {
                    let end = start + pos;
                    if end > start {
                        parts.push(&request.body[start..end]);
                    }
                    start = end + boundary_bytes.len();
                }

                if start < request.body.len() {
                    parts.push(&request.body[start..]);
                }

                for part in parts {
                    // Remove leading \r\n
                    let part = if part.starts_with(b"\r\n") {
                        &part[2..]
                    } else {
                        part
                    };

                    // Split headers from content
                    let mut sections = part.split(|b| b == &b'\n');
                    if let Some(headers) = sections.next() {
                        //let headers_str = String::from_utf8_lossy(headers);
                        if let Some(filename) = extract_filename_from_disposition(headers) {
                            let file_path = format!("{}/{}", upload_dir, filename);

                            // Create directory if it doesn't exist
                            let dir = std::path::Path::new(&file_path).parent().unwrap();
                            if !dir.exists() {
                                std::fs::create_dir_all(dir)?;
                            }

                            //  let mut file = File::create(&file_path)?;

                            let mut file = File::create(&file_path)?;

                            // Localiser la position de début et de fin du contenu
                            if let Some(content_start) =
                                part.windows(4).position(|w| w == b"\r\n\r\n")
                            {
                                //println!("part: {:?}", String::from_utf8_lossy(part));
                                // La position de fin du contenu est après le premier \r\n\r\n
                                let content_start = content_start + 4; // Ajouter 4 pour le décalage après \r\n\r\n

                                // Trouver la position de fin de la partie (le premier \r\n après le début du contenu)
                                let content_end = part[content_start..]
                                    .windows(2)
                                    .position(|w| w == b"\r\n")
                                    .map(|pos| pos + content_start); // Ajuster la position globale

                                if let Some(_end) = content_end {
                                    // Extraire le contenu du fichier
                                    let content = &part[content_start..];
                                    file.write_all(content)?;
                                    //  println!("content: {:?}", String::from_utf8_lossy(content));
                                } else {
                                    // Si aucune fin n'est trouvée, écrire le reste du contenu
                                    let content = &part[content_start..];
                                    //println!("content: {:?}", String::from_utf8_lossy(content));

                                    file.write_all(content)?;
                                }
                            }
                            let response =
                                Response::new(200, b"File uploaded successfully".to_vec());
                            return response.send(stream);
                        }
                    }
                }
            }
        }
    }

    let response = Response::new(400, handle_error(&config.error_pages.error_400, config, server_index));
    response.send(stream)
}

// Fonction utilitaire pour extraire le nom du fichier du Content-Disposition
fn extract_filename_from_disposition(disposition: &[u8]) -> Option<String> {
    let disposition_str = String::from_utf8_lossy(disposition);
    if let Some(filename_part) = disposition_str
        .split(';')
        .find(|part| part.trim().starts_with("filename="))
    {
        let filename = filename_part
            .trim()
            .split('=')
            .nth(1)?
            .trim_matches('"')
            .to_string();
        Some(filename)
    } else {
        None
    }
}

fn handle_delete_request(
    stream: &mut TcpStream,
    request: &Request,
    config: &ServerConfig,
    server_index: usize,
) -> io::Result<()> {
    // Construire le chemin du fichier à supprimer
    let file_to_delete = format!("{}", request.path);
    let cleaned_path = file_to_delete.trim_start_matches('/');

    //println!("{}",cleaned_path);
    // Vérifier que le fichier existe
    if !fs::metadata(&cleaned_path).is_ok() {
        // Si le fichier n'existe pas, renvoyer une réponse d'erreur 404
        let response = Response::new(404, handle_error(&config.error_pages.error_404, config, server_index));
        return response.send(stream);
    }

    // Tenter de supprimer le fichier
    match fs::remove_file(&cleaned_path) {
        Ok(_) => {
            // Si la suppression réussit, renvoyer une réponse 200 OK
            let response = Response::new(200, b"File deleted successfully".to_vec());
            response.send(stream)
        }
        Err(e) => {
            eprintln!("Error deleting file: {:?}", e); // Log de l'erreur
                                                       // Si une erreur se produit lors de la suppression, renvoyer une réponse d'erreur 500
            let response = Response::new(500, handle_error(&config.error_pages.error_500, config, server_index));
            response.send(stream)
        }
    }
}
fn handle_error(path: &str, config: &ServerConfig, server_index: usize) -> Vec<u8> {
    // Accéder au serveur actuel via un index ou une autre méthode appropriée
    let current_server = &config.servers.get(server_index).expect("No server found");

    // Construire le chemin complet de l'erreur en utilisant la racine du serveur actuel
    let full_path = format!("{}/{}", current_server.root, path);

    fs::read(path).unwrap_or_else(|_| {
        // Choisir la page d'erreur appropriée selon le code d'erreur dans le chemin
        if path.contains("404") {
            match fs::read(&full_path) {
                Ok(error_content) => error_content,
                Err(_) => b"Error 404: File Not Found".to_vec(),
            }
        } else if path.contains("405") {
            match fs::read(&full_path) {
                Ok(error_content) => error_content,
                Err(_) => b"Error 405: File Not Found".to_vec(),
            }
        } else if path.contains("400") {
            match fs::read(&full_path) {
                Ok(error_content) => error_content,
                Err(_) => b"Error 400: File Not Found".to_vec(),
            }
        } else {
            match fs::read(&full_path) {
                Ok(error_content) => error_content,
                Err(_) => b"Error 500: File Not Found".to_vec(),
            }
        }
    })
}

fn is_method_allowed(method: &str, allowed_methods: &[String]) -> bool {
    allowed_methods
        .iter()
        .any(|m| m.eq_ignore_ascii_case(method))
}

fn handle_cgi_request(
    stream: &mut TcpStream,
    request: &Request,
    config: &ServerConfig,
    server_index: usize,
) -> io::Result<()> {
    // Construire le chemin complet du script CGI
    let script_path = format!(".{}", request.path);

    // Déterminer le type de script à exécuter (Python, Shell, etc.)
    let command = if script_path.ends_with(".py") {
        "python3"
    } else if script_path.ends_with(".sh") {
        "sh"
    } else {
        // Si le type de fichier n'est pas supporté, renvoyer une erreur 400
        let response = Response::new(400, handle_error(&config.error_pages.error_400, config, server_index));
        return response.send(stream);
    };

    // Exécuter le script CGI avec le bon interpréteur
    let output = std::process::Command::new(command)
        .arg(&script_path)
        .output(); // Capturer la sortie

    match output {
        Ok(output) => {
            // Vérifier le statut de sortie du script
            if output.status.success() {
                // Envoyer la sortie du script comme réponse HTTP
                let response = Response::new(200, output.stdout);
                response.send(stream)
            } else {
                // En cas d'erreur, renvoyer une erreur 500 avec le stderr
                let response = Response::new(500, output.stderr);
                response.send(stream)
            }
        }
        Err(e) => {
            // En cas d'erreur d'exécution, renvoyer une erreur 500
            println!("Erreur lors de l'exécution du script CGI : {}", e);
            let response = Response::new(500, handle_error(&config.error_pages.error_500, config, server_index));
            response.send(stream)
        }
    }
}
