use chrono::prelude::*;
use home::home_dir;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::sync::mpsc::SyncSender;

use std::error::Error;
use std::fs::File;
use csv::ReaderBuilder;
use std::io::BufReader;
use ndarray::Array1;

pub mod api;
pub mod config;
pub mod spectrum;
pub mod spectrum_handler;
pub mod svg_utils;

#[derive(Serialize, Deserialize, Debug)]
pub struct Log {
    msg: String,
    log_type: LogType,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum LogType {
    Info,
    Warning,
    Error,
}

fn get_log_path() -> PathBuf {
    if let Some(home) = home_dir() {
        let path = home.join(".config").join("rosav");
        if create_dir_all(path.clone()).is_ok() {
            return path.join("rosav.log");
        }
    }

    Path::new("rosav.log").to_path_buf() // If anything fails, create at the app's location
}

pub fn setup_fern_logger() -> Result<(), fern::InitError> {
    let log_path = get_log_path();

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[#{} at {} on {}] {}",
                record.level(),
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.target(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(fern::log_file(log_path)?)
        .apply()?;

    Ok(())
}

pub fn new_log(msg: String, log_type: LogType) -> Log {
    Log { msg, log_type }
}

pub fn log_info(tx: &SyncSender<Log>, msg: String) {
    info!("{}", msg);
    println!("#Info: {}", msg);
    match tx.send(new_log(msg, LogType::Info)) {
        Ok(_) => (),
        Err(error) => println!(
            "#Exteme error: Error when trying to send info log! ({})",
            error
        ),
    }
}

pub fn log_war(tx: &SyncSender<Log>, msg: String) {
    warn!("{}", msg);
    println!("#Warning: {}", msg);
    match tx.send(new_log(msg, LogType::Warning)) {
        Ok(_) => (),
        Err(error) => println!(
            "#Exteme error: Error when trying to send warning log! ({})",
            error
        ),
    }
}

pub fn log_error(tx: &SyncSender<Log>, msg: String) {
    error!("{}", msg);
    println!("#Error: {}", msg);
    match tx.send(new_log(msg, LogType::Error)) {
        Ok(_) => (),
        Err(error) => println!(
            "#Exteme error: Error when trying to send error log! ({})",
            error
        ),
    }
}

// CSV files


/// Aproxima um espectro de LPFG por uma lorentziana em Rust
///
/// # Parâmetros
///
/// * `x` - Wavelength for simulation
/// * `a` - Attenuation intensity
/// * `x0` - Resonant wavelength
/// * `w` - FWHM
/// * `bias` - Insertion loss
///
/// # Retorna
///
/// * `spectrum` - LPFG array
pub fn transmission_spectra(x: Array1<f64>, a: f64, x0: f64, w: f64, bias: f64) -> Array1<f64> {
    let factor = w / (2.0 * (a / 3.0 - 1.0).abs().sqrt());
    x.mapv(|xi| -a * (1.0 + ((xi - x0) / factor).powi(2)).powf(-1.0) - bias)
}

pub fn my_gauss(x: Array1<f64>, a: f64, x0: f64, w: f64, bias: f64) -> Array1<f64> {
    let s = 2.0 * (4.0 * (a / 3.01).ln().abs()).sqrt();
    let s = w / s;
    let arg = x.mapv(|xi| -((xi - x0).powi(2) / (2.0 * s.powi(2))));
    arg.mapv(|arg_val| -a * arg_val.exp() - bias)
}

pub fn transmission_spectra_2(x: Array1<f64>, a: f64, x0: f64, w: f64, bias: f64, fcn: f64) -> Array1<f64> {
    if fcn < 0.5 {
        transmission_spectra(x, a, x0, w, bias)
    } else {
        let ts = transmission_spectra(x.clone(), a / 2.0, x0, w / 2.0, bias);
        let mg = my_gauss(x, a / 2.0, x0, w / 2.0, bias);
        ts + mg + bias
    }
}

// #[derive(Debug, Deserialize)]
// pub struct Record {
//     field1: String,
//     field2: String,
//     field3: i32,
// }

#[derive(Debug, Deserialize)]
pub struct Record {
    a: f64,
    x0: f64,
    w: f64,
    bias: f64,
}

pub fn read_csv(file_path: &str) -> Result<Vec<Record>, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);
    
    let mut records = Vec::new();
    for result in rdr.deserialize() {
        let record: Record = result?;
        records.push(record);
    }
    Ok(records)
}

pub fn read_txt(file_path: &str) -> Result<Vec<(f64, f64)>, Box<dyn Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut rdr = ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(false)
        .from_reader(reader);

    let mut records = Vec::new();
    for result in rdr.records() {
        let record = result?;
        let value1: f64 = record[0].parse()?;
        let value2: f64 = record[1].parse()?;
        records.push((value1, value2));
    }
    Ok(records)
}

pub fn show_data_txt(file_path: &str) {
    match read_txt(file_path) {
        Ok(records) => {
            for (i, (val1, val2)) in records.iter().enumerate() {
                println!("{}: {}, {}", i + 1, val1, val2);
            }
        }
        Err(err) => {
            eprintln!("Error reading file: {}", err);
        }
    }
}

pub fn show_strat(file_path: &str) {
    match read_csv(file_path) {
        Ok(records) => {
            for record in records {
                println!("{:?}", record);
            }
        }
        Err(err) => {
            eprintln!("Error reading CSV file: {}", err);
        }
    }
}