/*
    This file is part of Tarangam.

    Tarangam is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Tarangam is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with Tarangam.  If not, see <https://www.gnu.org/licenses/>
*/

//! Feel free to see through codes. Application is not written to be used as a library for other app. :)

mod graph;

use gtk::prelude::*;

use rand::Rng;

use std::{collections::HashMap, sync::{Arc, Mutex}};
use std::rc::Rc;
use std::cell::RefCell;
use std::io::prelude::*;
use std::io::BufReader;


use graph::Graph;

/// Status of Serial reading
enum Status {
    JAGRIT, // Mode of Active
    SAYAN, // Mode of Sleeping
    AVRODTIH, // Mode of being stopped
    PARIVARTIT // Mode of being values modified
}

/// Configuration to read from Serial Port
pub struct Config {
    status: Status,
    bondrate: u32,
    port: String
}

impl Config {
    pub fn new() -> Config {
        Config {
            status: Status::AVRODTIH,
            bondrate: 9600,
            port: "".to_owned()
        }
    }
}

/// For communication between mpsc of graph and serial port
#[derive(Debug)]
enum MessageSerialThread {
    Msg(String, MessageSerialThreadMsgType),
    Points(Vec<(String, f64)>),
    Status(String)
}

#[derive(Debug)]
enum MessageSerialThreadMsgType {
    Point,
    Log
}

// Building and configuring GUI
pub fn build_ui(app: &gtk::Application, ui_file: &str) {
    let config = Arc::new(Mutex::new(Config::new()));
    let builder = gtk::Builder::from_file(ui_file);

    let win = builder.get_object::<gtk::ApplicationWindow>("win").expect("Resource file missing!");
    win.set_application(Some(app));
    let bar = builder.get_object::<gtk::Statusbar>("status_bar").expect("Resource file missing!");
    let log_area = builder.get_object::<gtk::TextView>("log_area").expect("Resource file missing!");

    let graph = Graph::new(
        builder.get_object::<gtk::DrawingArea>("draw_area").expect("Resource file missing!"),
        0.0, 100.0,
        0.0, 100.0,
        false,
        true,
        false,
        true,
        HashMap::new(),
        0.0
    );

    win.show_all();

    // exit_menu
    let exit_menu = builder.get_object::<gtk::MenuItem>("exit_menu").expect("Resource file missing!");

    let tmp_win = win.clone();
    exit_menu.connect_activate(move |_|{
        unsafe {
            tmp_win.destroy();
        }
    });

    // about_menu
    let about_menu = builder.get_object::<gtk::MenuItem>("about_menu").expect("Resource file missing!");
    let about_window = builder.get_object::<gtk::AboutDialog>("about_window").expect("Resource file missing!");
    about_window.set_transient_for(Some(&win));
    about_window.set_version(Some(env!("CARGO_PKG_VERSION")));
    about_window.connect_delete_event(|win,_| {
        win.hide();
        Inhibit(true)
    });

    let a_win = about_window.clone();
    about_menu.connect_activate(move |_|{
        a_win.show();
        a_win.present();
    });

    // save_log
    let save_menu = builder.get_object::<gtk::MenuItem>("save_menu").expect("Resource file missing!");
    
    let save_window = builder.get_object::<gtk::FileChooserDialog>("save_window").expect("Resource file missing!");
    save_window.set_transient_for(Some(&win));
    save_window.set_action(gtk::FileChooserAction::Save);
    
    save_window.connect_delete_event(|win,_| {
        win.hide();
        Inhibit(true)
    });

    save_window.add_button("_Save", gtk::ResponseType::Apply);
    save_window.add_button("_Cancel", gtk::ResponseType::Cancel);

    let tmp_log_area = log_area.clone();
    let tmp_bar =  bar.clone();
    save_window.connect_response(move |win, res| {
        match res {
            gtk::ResponseType::Cancel => win.hide(),
            gtk::ResponseType::Apply => {
                if let Some(path) = win.get_filename() {
                    if let Some(buf) = tmp_log_area.get_buffer() {
                        let text = buf.get_text(&buf.get_start_iter(), &buf.get_end_iter(), false).unwrap().to_string();

                        match std::fs::write(path, text) {
                            Ok(_) => { 
                                win.hide();
                            },
                            Err(_) => {
                                tmp_bar.push(1, "Failed to save!");
                            }
                        }
                    }
                }
            },
            _ => ()
        }
    });

    save_menu.connect_activate(move |_|{
        save_window.show();
    });

    // pankti
    let pankti = builder.get_object::<gtk::SpinButton>("pankti").expect("Resource file missing!");

    let tmp_graph = Rc::clone(&graph);
    pankti.connect_value_changed(move |btn| {
        let mut tmp_graph = tmp_graph.borrow_mut();
        tmp_graph.scale_x_size = btn.get_value();
        tmp_graph.redraw();
    });

    // stambh_1
    let stambh_1 = builder.get_object::<gtk::Entry>("stambh_1").expect("Resource file missing!");

    let tmp_graph = Rc::clone(&graph);
    stambh_1.connect_activate(move |entry| {
        let mut tmp_graph = tmp_graph.borrow_mut();
        let val = entry.get_text().parse::<f64>().unwrap_or(0.0);
        let purana_y_start = tmp_graph.scale_y_start;
        let y_size = tmp_graph.scale_y_size;
        tmp_graph.scale_y_start = val;
        tmp_graph.scale_y_size = y_size + (purana_y_start - val);
        tmp_graph.redraw();
    });

    // stambh_2
    let stambh_2 = builder.get_object::<gtk::Entry>("stambh_2").expect("Resource file missing!");

    let tmp_graph = Rc::clone(&graph);
    stambh_2.connect_activate(move |entry| {
        let mut tmp_graph = tmp_graph.borrow_mut();
        let val = entry.get_text().parse::<f64>().unwrap_or(0.0);
        let y_start = tmp_graph.scale_y_start;
        tmp_graph.scale_y_size = (val - y_start).abs();
        tmp_graph.redraw();
    });

    // nimna_stambh
    let nimna_stambh = builder.get_object::<gtk::CheckButton>("nimna_stambh").expect("Resource file missing!");

    let tmp_graph = Rc::clone(&graph);
    nimna_stambh.connect_clicked(move |btn| {
        tmp_graph.borrow_mut().auto_adjust_y = !btn.get_active();
        stambh_1.set_sensitive(btn.get_active());
        stambh_2.set_sensitive(btn.get_active());
        if btn.get_active() {
            stambh_1.emit_activate();
            stambh_2.emit_activate();
        }
    });

    // draw_patches
    let draw_patches = builder.get_object::<gtk::CheckButton>("draw_patches").expect("Resource file missing!");

    let tmp_graph = Rc::clone(&graph);
    draw_patches.connect_clicked(move |btn| {
        let mut tmp_graph = tmp_graph.borrow_mut();
        tmp_graph.draw_patch = btn.get_active();
        tmp_graph.redraw();
    });

    // draw_baarik_box
    let draw_baarik_box = builder.get_object::<gtk::CheckButton>("draw_baarik_box").expect("Resource file missing!");

    let tmp_graph = Rc::clone(&graph);
    draw_baarik_box.connect_clicked(move |btn| {
        let mut tmp_graph = tmp_graph.borrow_mut();
        tmp_graph.draw_baarik_box = btn.get_active();
        tmp_graph.redraw();
    });

    // draw_box
    let draw_box = builder.get_object::<gtk::CheckButton>("draw_box").expect("Resource file missing!");

    let tmp_graph = Rc::clone(&graph);
    draw_box.connect_clicked(move |btn| {
        draw_baarik_box.set_sensitive(btn.get_active());
        let mut tmp_graph = tmp_graph.borrow_mut();
        tmp_graph.draw_box = btn.get_active();
        tmp_graph.redraw();
    });

    // Bondrate
    let bondrate = builder.get_object::<gtk::ComboBoxText>("bondrate").expect("Resource file missing!");
    
    let tmp_bar =  bar.clone();
    let tmp_config = Arc::clone(&config);
    bondrate.connect_changed(move |cbx| {
        match tmp_config.lock() {
            Ok(mut config) => {
                config.bondrate = match cbx.get_active_text() {
                    Some(txt) => txt.to_string().parse::<u32>().unwrap_or(9600u32),
                    None => 9600
                };
            }, Err(_) => {
                tmp_bar.push(1, "Failed to change bondrate!");
            }
        }
    });

    // port
    let refresh_port = builder.get_object::<gtk::ToolButton>("refresh_port").expect("Resource file missing!");
    let port = builder.get_object::<gtk::ComboBoxText>("port").expect("Resource file missing!");

    let tmp_bar =  bar.clone();
    let tmp_port = port.clone();
    refresh_port.connect_clicked(move |_| {
        tmp_port.remove_all();
        match serialport::available_ports() {
            Ok(ports) => {
                if ports.len() == 0 { tmp_bar.push(1, "No port found!"); }
                for p in ports {
                    tmp_port.append_text(p.port_name.as_str());
                }
            }, Err(_) => {
                tmp_bar.push(1, "No port found!");
            }
        }
    });

    let tmp_bar =  bar.clone();
    let tmp_config = Arc::clone(&config);
    port.connect_changed(move |cbx| {
        match tmp_config.lock() {
            Ok(mut config) => {
                config.port = match cbx.get_active_text() {
                    Some(txt) => txt.to_string(),
                    None => "".to_owned()
                };
            }, Err(_) => {
                tmp_bar.push(1, "Failed to change port!");
            }
        }
    });

    // clear_graph
    let clear_graph = builder.get_object::<gtk::ToolButton>("clear_graph").expect("Resource file missing!");

    let tmp_graph = Rc::clone(&graph);
    clear_graph.connect_clicked(move |_ | {
        tmp_graph.borrow_mut().pankti_sankya = 0.0;
        tmp_graph.borrow_mut().lines.clear();
        tmp_graph.borrow_mut().redraw();
    });

    // jagrit_btn
    let jagrit_btn = builder.get_object::<gtk::ToolButton>("jagrit_btn").expect("Resource file missing!");

    let tmp_bar =  bar.clone();
    let tmp_config = Arc::clone(&config);
    let tmp_graph = Rc::clone(&graph);
    jagrit_btn.connect_clicked(move |_ | {
        match tmp_config.lock() {
            Ok(mut config) => {
                tmp_graph.borrow_mut().pankti_sankya = 0.0;
                tmp_graph.borrow_mut().lines.clear();
                tmp_graph.borrow_mut().redraw();
                tmp_bar.push(1, "Jagrit");
                config.status = Status::PARIVARTIT;
            }, Err(_) => {
                tmp_bar.push(1, "Failed to change port!");
            }
        }
    });

    // avrodith_btn
    let avrodith_btn = builder.get_object::<gtk::ToolButton>("avrodith_btn").expect("Resource file missing!");

    let tmp_bar =  bar.clone();
    let tmp_config = Arc::clone(&config);
    avrodith_btn.connect_clicked(move |_| {
        match tmp_config.lock() {
            Ok(mut config) => {
                tmp_bar.push(1, "Avrodhit");
                config.status = Status::AVRODTIH;
            }, Err(_) => {
                tmp_bar.push(1, "Failed to change port!");
            }
        }
    });

    // clear_log
    let clear_log = builder.get_object::<gtk::ToolButton>("clear_log").expect("Resource file missing!");

    let tmp_log_area = log_area.clone();
    clear_log.connect_clicked(move |_| {
        tmp_log_area.get_buffer().expect("Couldn't get window").set_text("");
    });

    // send_entry
    let send_entry = builder.get_object::<gtk::Entry>("send_entry").expect("Resource file missing!");

    let tmp_bar =  bar.clone();
    let tmp_config = Arc::clone(&config);
    send_entry.connect_activate(move |ent| {
        send_text(&tmp_config, ent, &tmp_bar);
    });

    // send_btn
    let send_btn = builder.get_object::<gtk::Button>("send_btn").expect("Resource file missing!");

    let tmp_bar =  bar.clone();
    let tmp_config = Arc::clone(&config);
    send_btn.connect_clicked(move |_| {
        send_text(&tmp_config, &send_entry, &tmp_bar);
    });

    /*
        Thread to manage Serial Port

        The program runs a thread to read and parse the output from serial port and
        send it through mpsc (rx, tx) to a recever. Where it is added to Graph 
        or Log is added to text area or any status is displayed in bar
    */
    let (sender, receiver) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

    let tmp_config = Arc::clone(&config);
    std::thread::spawn(move || {
        let mut bufread: Option<BufReader<Box<dyn  serialport::SerialPort>>> = None;
        let mut buf = String::new();
        loop {
            serial_thread_work(&tmp_config, &mut bufread, &sender, &mut buf);
        }
    });

    // Reciver for MessageSerialThread from the "Thread to manage Serial Port" and works accordingly
    let full_log = builder.get_object::<gtk::CheckButton>("full_log").expect("Resource file missing!");
    let graph_data = builder.get_object::<gtk::TextView>("graph_data").expect("Resource file missing!");
    let tmp_graph = Rc::clone(&graph);
    receiver.attach(None, move |msg| {
        match msg {
            MessageSerialThread::Msg(text, msg_type) => {
                receiver_for_msg(text, &msg_type, &full_log, &log_area);
            },
            MessageSerialThread::Points(points) => {
                receiver_for_points(points, &tmp_graph, &graph_data);
            }
            MessageSerialThread::Status(text) => {
                bar.push(1, &text);
            }
        }
        glib::Continue(true)
    });
}

// Controls the thread and read from serial port
fn serial_thread_work(
    config: &Arc<Mutex<Config>>, 
    bufread: &mut Option<BufReader<Box<dyn  serialport::SerialPort>>>, 
    sender: &glib::Sender<MessageSerialThread>, 
    buf: &mut String) {
    let mut do_sleep = false;
    match config.lock() {
        Ok(mut config) => {
            match config.status {
                Status::AVRODTIH => {
                    *bufread = None;
                    config.status = Status::SAYAN;
                },
                Status::JAGRIT => {
                    if let Some(read) = bufread {
                        if let Ok(_) = read.read_line(buf) {
                            for line in buf.lines() {
                                if line.len() == 0 {
                                    continue;
                                } else if line.starts_with("#") {
                                    let mut points: Vec<(String, f64)> = Vec::new();
                                    for (index, line) in line[1..].split(" ").enumerate() {
                                        let part = line.split("=");   
                                        let part = part.into_iter().collect::<Vec<&str>>();
                                        if part.len() == 1 {
                                            let num = match part[0].trim().parse::<f64>() {
                                                Ok(val) => val,
                                                Err(_) => {
                                                    continue;
                                                }
                                            };

                                            points.push((index.to_string(), num));
                                        } else if part.len() == 2 {
                                            points.push((part[0].trim().to_owned(), part[1].parse::<f64>().unwrap()));
                                        }
                                    }
                                    sender.send(MessageSerialThread::Points(points)).unwrap();
                                    sender.send(MessageSerialThread::Msg(line.to_owned(),  MessageSerialThreadMsgType::Point)).unwrap();
                                } else {
                                    sender.send(MessageSerialThread::Msg(line.to_owned(), MessageSerialThreadMsgType::Log)).unwrap();
                                }
                            }
                            buf.clear();
                        }
                    }
                },
                Status::PARIVARTIT => {
                    let p = match serialport::new(&config.port, config.bondrate).open() {
                        Ok(p) => p,
                        Err(_) => {
                            return;
                        }
                    };

                    *bufread = Some(BufReader::new(p));
                    config.status = Status::JAGRIT;
                },
                Status::SAYAN => {
                    do_sleep = true;
                }
            }
        }, Err(_) => {
            sender.send(MessageSerialThread::Status("Faild prepare for communication!".to_owned())).unwrap();
            return;
        }
    };
    
    // Hack for smooth performance
    if do_sleep {
        std::thread::sleep(std::time::Duration::from_millis(100));
    } else {
        std::thread::sleep(std::time::Duration::from_nanos(1));
    }
}


// Receives MessageSerialThread from Serial Port managing thread adds message to text area
fn receiver_for_msg(text: String, msg_type: &MessageSerialThreadMsgType, full_log: &gtk::CheckButton, log_area: &gtk::TextView) {
    if !full_log.get_active(){
        if let MessageSerialThreadMsgType::Point = msg_type {
            return;
        }
    }
    let buf = log_area.get_buffer()
        .expect("Couldn't get log_area");
    buf.insert(&mut buf.get_end_iter(), &format!("{}\n",text));
    log_area.scroll_to_iter(&mut buf.get_end_iter(), 0.4, true, 0.0, 0.0);
    log_area.queue_draw();
}

// Receives MessageSerialThread from Serial Port managing thread and add points to draw on graph
fn receiver_for_points(points: Vec<(String, f64)>, graph: &Rc<RefCell<Graph>>, graph_data: &gtk::TextView) {
    for (line, point) in points {
        let mut gp = graph.borrow_mut();
                
        let sankhya = gp.pankti_sankya;
        match gp.lines.get_mut(&line) {
            Some(val) => {
                val.points.push((sankhya, point));
            } None => {
                let v = vec![(sankhya, point)];
                let mut rng = rand::thread_rng();
                gp.lines.insert(line, graph::Line::new(rng.gen_range(0.0..1.0), 0.0, rng.gen_range(0.0..1.0), v));
                let buf = graph_data.get_buffer().expect("Couldn't get graph_data");
                buf.set_text("");
                gp.lines.iter().for_each(|(key, line)| {
                    buf.insert(&mut buf.get_end_iter(), "##");
                    
                    let tag = gtk::TextTag::new(None);
                    let rgba = gdk::RGBA {
                        red: line.color.0,
                        green: line.color.1,
                        blue: line.color.2,
                        alpha: 1.0
                    };
                    tag.set_property_background_rgba(Some(&rgba));
                    tag.set_property_foreground_rgba(Some(&rgba));
                    buf.get_tag_table().unwrap().add(&tag);
                    buf.apply_tag(&tag, &buf.get_iter_at_offset(buf.get_end_iter().get_offset() - 2), &buf.get_end_iter());
                    buf.insert(&mut buf.get_end_iter(), &format!(" {}, ", key));
                });
                graph_data.queue_draw();
            }
        }
        gp.redraw();
    }
    graph.borrow_mut().pankti_sankya += 1.0;
}

// Sends text through Serial Post to device
fn send_text(config: &Arc<Mutex<Config>>, entry: &gtk::Entry, bar: &gtk::Statusbar) {
    match config.lock() {
        Ok(config) => {
            if let Status::JAGRIT = config.status {
                let mut p = match serialport::new(&config.port, config.bondrate).open() {
                    Ok(p) => p,
                    Err(_) => {
                        bar.push(1, "Failed to change port!");
                        return;
                    }
                };
    
                unsafe {
                    p.write_all(entry.get_text().to_string().as_bytes_mut()).unwrap();
                }
                entry.set_text("");
            }
        }, Err(_) => {
            bar.push(1, "Failed to change port!");
        }
    }
}
