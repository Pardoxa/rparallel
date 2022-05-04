use structopt::StructOpt;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::{
    fs::File,
    io::{BufRead, BufReader, Write, BufWriter},
    process::Command,
    path::Path
};
use rayon::prelude::*;

fn main() {
    let opt = Job::from_args();
    if let Some(j) = opt.j
    {
        rayon::ThreadPoolBuilder::new().num_threads(j.get()).build_global().unwrap();
    }

    let file = File::open(&opt.path)
        .expect("unable to open file");
    let reader = BufReader::new(file);

    let commands: Vec<_> = reader.lines()
        .filter_map(
            |l|
            l.ok()
        ).filter(
            |s|
            {
                !(s.starts_with('#') || s.is_empty())
            }
        ).collect();
    
    let exec_path = opt.execution_path
        .clone()
        .map(std::fs::canonicalize)
        .map(|r| r.unwrap());
        

    let mut error = false;
    for index in 0..commands.len()
    {
        if !check_dir_errors(&opt, index, &exec_path)
        {
            error = true;
        }
    }
    if error {
        println!("Dir Errors! Abbort");
        std::process::exit(1);
    }

    commands.par_iter()
        .enumerate()
        .for_each(
            |(index, command)|
            {
                let mut cmd = Command::new("sh");
                

                
                let dir = if let Some(p) = &exec_path
                {
                    if opt.copy_back{
                        let mut path = PathBuf::from(p);
                        
                        let dir_name = if let Some(n) = &opt.tmp_dir
                        {
                            format!("{n}_{index}")
                        } else {
                            format!("{index}")
                        };
                        path.push(dir_name);
                        println!("{:?}", path);
                        std::fs::create_dir(&path)
                            .expect("unable to create dir");
                        cmd.current_dir(&path);
                        Some(path)
                    } else {
                        cmd.current_dir(p);
                        None
                    }
                } else {
                    None
                };

                let output = cmd.arg("-c")
                    .arg(command)
                    .output()
                    .expect("failed to execute process");

                let name = format!("log_{index}");
                if !output.stdout.is_empty()
                {
                    let name = format!("{name}.stdout");
                    let file = File::create(name)
                        .expect("unable to create file");
                    let mut buf = BufWriter::new(file);
                    buf.write_all(&output.stdout).unwrap();
                }

                if !output.stderr.is_empty(){
                    let name = format!("{name}.stdedd");
                    let file = File::create(name)
                        .expect("unable to create file");
                    let mut buf = BufWriter::new(file);
                    buf.write_all(&output.stderr).unwrap();  
                }

                if let Some(d) = dir {
                    let current = std::env::current_dir()
                        .expect("current dir invalid");

                    if !move_dir(&d, &current)
                    {
                        let last = d.file_name().unwrap().to_str().unwrap();
                        for i in 0..10 {
                            eprintln!("try_fallback {i}");
                            let mut n = current.clone();
                            n.push(format!("{last}_{i}"));
                            if move_dir(&d, n) 
                            {
                                eprintln!("Success");
                                break;
                            }
                        }
                    }
                }
            }
        );
}

pub fn check_dir_errors(param: &Job, index: usize, exec_path: &Option<PathBuf>) -> bool
{
    if let Some(p) = &exec_path
    {
        if !p.is_dir()
        {
            return false;
        }
        if param.copy_back{
            let mut path = PathBuf::from(p);
            
            let dir_name = if let Some(n) = &param.tmp_dir
            {
                format!("{n}_{index}")
            } else {
                format!("{index}")
            };
            path.push(dir_name);
            let valid = !path.exists();
            if !valid {
                eprintln!("Error: {:?} exists", path);
            }
            return valid
        } 
    }
    true
}

fn move_dir<P1, P2>(src: P1, dst: P2) -> bool
where P1: AsRef<Path>,
    P2: AsRef<Path>
{
    let d = src.as_ref().to_str().unwrap();

    let c = dst.as_ref().to_str().unwrap();
    let move_cmd = Command::new("mv")
        .args(&[d, c])
        .spawn()
        .unwrap()
        .wait();

    
    match move_cmd {
        Ok(status) => {
            status.success()
        },
        Err(e) => 
        {
            eprintln!("error in move: {e}");
            false
        }
    }
}

#[derive(Debug, StructOpt, Clone)]
/// Created by Yannick Feld
/// 
/// Used to run commands that are stored in a script in parallel
pub struct Job{
    /// How many threads to use?
    #[structopt(short)]
    pub j: Option<NonZeroUsize>,

    /// file of which every line is to be executed
    #[structopt(short, long)]
    pub path: String,

    /// where should the command be executed?
    #[structopt(short, long)]
    pub execution_path: Option<String>,

    /// How should temporary directorys be called?
    /// Will be appended with line number
    #[structopt(short, long)]
    pub tmp_dir: Option<String>,

    /// Copy all the files from execution_path
    #[structopt(short, long)]
    pub copy_back: bool
}