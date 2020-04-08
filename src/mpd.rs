use std::net::{TcpListener, TcpStream};
use std::thread;
use std::io::{Write, BufReader, BufRead};
use std::collections::HashMap;
use crate::mpd::mpd_commands::*;
use rspotify::spotify::client::Spotify;
use regex::Regex;

mod mpd_commands;

pub(crate) struct MpdServer<'a> {
    host: &'a str,
    spotify: Spotify
}

impl MpdServer<'static> {
    pub fn new(host: &'static str, spotify: Spotify) -> Self {
        Self {
            host,
            spotify
        }
    }

    pub fn run(&mut self) {
        let listener = TcpListener::bind(self.host.to_owned()).unwrap();
        println!("Server listening on {}", self.host);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!("New connection: {}", stream.peer_addr().unwrap());
                    let spotify = self.spotify.clone();
                    thread::spawn(move|| {
                        let mut mpd_handler = MpdRequestHandler::new(spotify);
                        mpd_handler.handle_client(stream)
                    });
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        }

        // close the socket server
        drop(listener);
    }
}

struct MpdRequestHandler {
    commands: HashMap<&'static str, Box<dyn MpdCommand + 'static>>,
    spotify: Spotify
}

impl MpdRequestHandler {
    pub fn new(spotify: Spotify) -> Self{
        Self {
            spotify,
            commands: HashMap::new()
        }
    }

    fn commands(&self) -> HashMap<&'static str, Box<dyn MpdCommand>> {
        let mut commands: HashMap<&'static str, Box<dyn MpdCommand>> = HashMap::new();
        commands.insert("status", Box::new(StatusCommand{}));
        commands.insert("stats", Box::new(StatsCommand{}));
        commands.insert("listplaylists", Box::new(ListPlaylistsCommand{ spotify: self.spotify.clone() }));
        commands.insert("listplaylistinfo", Box::new(ListPlaylistInfoCommand{ spotify: self.spotify.clone() }));

        commands
    }

    fn handle_client(&mut self, mut stream: TcpStream) {
        self.commands = self.commands();
        let welcome = b"OK MPD 0.21.11\n";
        stream.write(welcome).expect("Unable to send OK msg");

        loop {
            let mut reader = BufReader::new(&stream);
            let mut response = String::new();
            let mut command_list = vec![];
            reader.read_line(&mut response).expect("could not read");
            if response.trim() == "command_list_begin" {
                while response.trim() != "command_list_end" {
                    response = String::new();
                    reader.read_line(&mut response).expect("could not read");
                    if response.trim() != "command_list_end" {
                        command_list.push(response.trim().to_owned());
                    }
                }
            } else if response.len() > 0 && response.trim() != "idle" {
                command_list.push(response.trim().to_owned());
            }

            if command_list.len() > 0 {
                self.execute_command(&mut stream, command_list);
            }
        }
    }

    fn execute_command(&self, stream: &mut TcpStream, command_list: Vec<String>) {
        let mut output = vec![];
        for command in command_list {
            println!("Server received {:?}", command);
            output.extend(self.do_command(command));
        }
        output.push("OK\n".to_owned());
        stream.write(output.join("\n").as_bytes()).unwrap();
    }

    fn do_command (&self, command: String) -> Vec<String> {
        lazy_static! {
            static ref RE: Regex = Regex::new("\"([^\"]*)\"").unwrap();
        }

        for (name, mpd_command) in &self.commands {
            if command.starts_with(name) {
                let args = RE.captures(&command);
                return mpd_command.execute(args)
            }
        }

        let mut output = vec![];

        //if command.starts_with("add") && command.starts_with("play") || command == "command_list_begin" || command == "clear" || command.starts_with("plchanges") || command == "noidle" || command == "channels" || command == "playlistinfo" || command == "currentsong" || command.starts_with("replay_gain_mode") {
        //    stream.write(b"OK\n");
        //}
        if command.starts_with("lsinfo") {
            //stream.write(b"ACK [5@0] {lsinfo} Unsupported URI scheme");
        }
        if command == "urlhandlers" {
            output.push("handler: spotify:");
        }
        if command == "outputs" {
            output.push("outputsoutputid: 0");
            output.push("outputname: default detected output");
            output.push("plugin: alsa");
            output.push("outputenabled: 1");
            output.push("attribute: allowed_formats=");
            output.push("attribute: dop=0");
        }
        if command == "decoders" {
            output.push("plugin: mad");
            output.push("suffix: mp3");
            output.push("suffix: mp2");
            output.push("mime_type: audio/mpeg");
            output.push("plugin: mpcdec");
            output.push("suffix: mpc");
        }
        if command == "tagtypes" {
            output.push("tagtype: Artist");
            output.push("tagtype: ArtistSort");
            output.push("tagtype: Album");
            output.push("tagtype: AlbumSort");
            output.push("tagtype: AlbumArtist");
            output.push("tagtype: AlbumArtistSort");
            output.push("tagtype: Title");
            output.push("tagtype: Name");
            output.push("tagtype: Genre");
            output.push("tagtype: Date");
        }
        if command == "commands" {
            output.push("command: play");
            output.push("command: stop");
            output.push("command: pause");
            output.push("command: status");
            output.push("command: stats");
            output.push("command: decoders");
        }

        output.iter().map(|x| x.to_string()).collect::<Vec<String>>().into()
    }
}