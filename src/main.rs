use {
    structopt::StructOpt,
    std::{
        fs::File,
        io::{BufRead, BufReader, Write, BufWriter},
        process::{Command, Stdio},
        path::Path,
        env::current_dir,
        num::NonZeroUsize,
        path::PathBuf
    },
    rayon::prelude::*,
    regex::{Regex, Captures},
    rand::{SeedableRng, Rng}
};

fn main() {
    let opt = Job::from_args();
    if let Some(j) = opt.j
    {
        rayon::ThreadPoolBuilder::new().num_threads(j.get()).build_global().unwrap();
    }

    let file = match File::open(&opt.command_file)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Requested command file: {}", &opt.command_file);
            eprintln!("ERROR: {:#}", e);
            std::process::exit(2);
        }
    };

    let reader = BufReader::new(file);

    let mut commands: Vec<_> = reader.lines()
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
        .as_ref()
        .map(std::fs::canonicalize)
        .map(
            |r| 
            {
                match r {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("Requested execution path (either relative to calling dir or absolut): {:?}", opt.execution_path.as_ref().unwrap());
                        eprintln!("Execution path error: {:#}", e);
                        std::process::exit(1); 
                    }
                }
            }
        
        );

    
    let exec_path_is_empty = exec_path.as_ref().map(
        |p|
        {
            match p.read_dir()
            {
                Err(e) => {
                    eprintln!("Requested execution path, expanded to absolut path: {:?}", p);
                    eprintln!("Execution path error: {:#}", e);
                    std::process::exit(1); 
                },
                Ok(mut d) => d.next().is_none()
            }
        }
    );

    if let Some(is_empty) = exec_path_is_empty
    {
        if !opt.force && !is_empty && opt.move_back && opt.tmp_dir.is_none() {
            eprintln!("WARNING: execution directory is not empty before executing any command but requested to move all files in execution directory.\
            This could be dangerous. Thus MOVE of files will be SKIPPED. If this was not a mistake, the behavior can be changed by setting the --force flag.")
        }
    }
        
    let cwd = current_dir().unwrap();
    let cwd = cwd.to_str().unwrap();

    let re = Regex::new(r"§cwd§")
        .unwrap();

    commands
        .iter_mut()
        .for_each(
            |val|
            {
                let cow = re.replace_all(val, cwd);
                *val = cow.into_owned();
            }
        );
    
    let re = Regex::new(r"\$RANDOM")
        .unwrap();


    let mut rng = match opt.seed
    {
        None => rand_pcg::Pcg64::from_entropy(),
        Some(s) => rand_pcg::Pcg64::seed_from_u64(s)
    };

    let mut replacer: Box<dyn FnMut (&Captures) -> String> = if opt.u64{
        Box::new(
            |_: &Captures|
            {
                let num = rng.gen::<u64>();
                format!("{num}")
            }
        )
    } else {
        Box::new(
            |_: &Captures|
            {
                let num = rng.gen::<u32>();
                format!("{num}")
            }
        )
    };
    commands
        .iter_mut()
        .for_each(
            |val|
            {
                let cow = re.replace_all(
                    val, 
                    &mut replacer
                );
                *val = cow.into_owned();
            }
        );

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
                    
                    let mut path = PathBuf::from(p);
                    
                    if let Some(n) = &opt.tmp_dir
                    {
                        path.push(format!("{n}_{index}"));
                        println!("creating {:?}", path);
                        std::fs::create_dir(&path)
                            .expect("unable to create dir");
                    }
 
                    cmd.current_dir(&path);
                    Some(path)
                    
                } else {
                    None
                };

                
                let name = format!("{}_{index}", opt.logname);
                
                if opt.print {
                    cmd.stdout(std::process::Stdio::inherit());
                    cmd.stderr(std::process::Stdio::inherit());
                } else if opt.no_log {
                    cmd.stdout(Stdio::null());
                    cmd.stdout(Stdio::null());
                } else if opt.instant_log {
                    let out_name = format!("{name}.stdout");
                    let file_stdout = File::create(out_name).unwrap();
                    cmd.stdout(file_stdout);
                    let name = format!("{name}.stderr");
                    let file_stderr = File::create(name).unwrap();
                    cmd.stderr(file_stderr);
                }
                cmd.stdin(Stdio::null());

                let output = cmd.arg("-c")
                    .arg(command)
                    .output()
                    .expect("failed to execute process");

                
                if !opt.instant_log && !output.stdout.is_empty() && !opt.no_log && !opt.print
                {
                    let name = format!("{name}.stdout");
                    let file_stout = File::create(name)
                        .expect("unable to create file");
                    let mut buf = BufWriter::new(file_stout);
                    buf.write_all(&output.stdout).unwrap();
                }

                if !opt.instant_log && !output.stderr.is_empty() && !opt.no_log && !opt.print {
                    let name = format!("{name}.stderr");
                    let file_stderr = File::create(name)
                        .expect("unable to create file");
                    let mut buf = BufWriter::new(file_stderr);
                    buf.write_all(&output.stderr).unwrap();  
                }

                if opt.move_back && opt.tmp_dir.is_some() {
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
                
            }
        );

    if opt.tmp_dir.is_none() && opt.move_back
    {
        // if execution_path is None, exec_path_is_empty will also be None
        if let Some(is_empty) = exec_path_is_empty{
            if is_empty || opt.force{
                let ex_path = opt.execution_path.unwrap();
                if !move_files_and_subdir(&ex_path, cwd){
                    eprintln!("ERROR: Move failed :-/")
                }
            }
        }
    }
}

pub fn check_dir_errors(param: &Job, index: usize, exec_path: &Option<PathBuf>) -> bool
{
    if let Some(p) = &exec_path
    {
        if !p.is_dir()
        {
            return false;
        }
        if param.move_back{
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

fn move_files_and_subdir(src: &str, dst: &str) -> bool
{
    let cmd = format!("mv {src}/* {dst}");

    println!("{}", cmd);
    let move_cmd = Command::new("sh")
        .arg("-c")
        .arg(&cmd)
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
/// Used to run commands that are stored in a script in parallel.
/// The order of the commands is not guaranteed.
/// Commands are executed in shell (sh) not bash.
/// 
/// Commands are set to ignore standard input as there is no way to give them input.
/// 
/// Note: all occurences of §cwd§ will be replaced by the directory this program was called in!
/// Also $RANDOM will be replaced with a randomly drawn u32 (unsigned 32bit integer)
/// 
pub struct Job{
    /// Number of commands that are run in parallel.
    /// If not given the program will try to figure out the appropriate 
    /// ammount itself
    #[structopt(short)]
    pub j: Option<NonZeroUsize>,

    /// (path to) file of which every line is to be executed in the shell (sh)
    pub command_file: String,

    /// where should the command be executed?
    /// Default: Current directroy
    #[structopt(short, long)]
    pub execution_path: Option<String>,

    /// Stem name of temporary directories that are created in the execution directory.
    /// All commands will be run in their respective temporary directory instead.
    /// Every command gets a unique directory, as the stem name is appended with the execution index
    /// that corresponds to the line number in the command file (though blank lines or lines starting with # are not counted)
    /// 
    /// The option move_back will now move the temporary directory of a command each time the 
    /// corresponding command finishes. This is useful if the commands you want to execute 
    /// may produce output that would interfere with one another, e.g., by having the same names 
    #[structopt(short, long)]
    pub tmp_dir: Option<String>,

    /// move all the files from execution_path to calling directory after all commands finish.
    /// CAUTION: This will move all files and subdirectorys of the execution directory! 
    /// 
    /// If option tmp_dir is used, it will only move the newly create sub directories, in which 
    /// the commands were executed
    #[structopt(short, long)]
    pub move_back: bool,

    /// Ignore stdout and stderr of commands, don't create log files.
    /// The --print flag will ignore this option
    #[structopt(short, long)]
    pub no_log: bool,

    /// Seed for the random number generator used to replace $RANDOM.
    /// If nothing is given, the seed will be generated from entropy
    #[structopt(short, long)]
    pub seed: Option<u64>,

    /// Print output to stdout and stderr instead of creating a logfile 
    /// for each command
    /// 
    /// Note: print writes output instantly, without buffering,
    /// similar to the behavior one can get for the logfiles with the 
    /// --instant-log flag
    #[structopt(long)]
    pub print: bool,

    /// Name of the logfiles created for each command.
    /// If the flag --print is not set each command will print 
    /// a logfile called {logname}_{command_index}.stdout and .stderr
    /// These will be created whenever a command finishes, if 
    /// said command did output anything to stdout (stderr)
    #[structopt(long, short, default_value = "log")]
    pub logname: String,

    /// Changes behavior of $RANDOM to be exchanged for an u64 instead,
    /// i.e., an 64 bit unsigned integer
    #[structopt(long)]
    pub u64: bool,

    /// Force the move of files even if the execution path was not 
    /// empty before executing any command
    #[structopt(long, short)]
    pub force: bool,

    /// This way the output of the commands is written to the corresponding log files instantly 
    /// (usual behavior: storing output in RAM until command finishes and flushing afterwards).
    /// This will result in empty logfiles for commands that do not output anything,
    /// as the programm has no way of knowing that in advance
    #[structopt(long, short)]
    pub instant_log: bool
}