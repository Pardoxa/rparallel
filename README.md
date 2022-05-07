# Run comands in parallel

Very small program.
Used to run commands from a file in parallel.
Use at your own risk.


# help output 
```
rparallel 0.1.0
Created by Yannick Feld

Used to run commands that are stored in a script in parallel. The order of the commands is not guaranteed. Commands are
executed in shell (sh) not bash.

Commands are set to ignore standard input as there is no way to give them input.

Note: all occurences of §cwd§ will be replaced by the directory this program was called in! Also $RANDOM will be
replaced with a randomly drawn u32 (unsigned 32bit integer)

USAGE:
    rparallel [FLAGS] [OPTIONS] <command-file>

FLAGS:
    -f, --force          
            Force the move of files even if the execution path was not empty before executing any command

    -h, --help           
            Prints help information

    -i, --instant-log    
            This way the output of the commands is written to the corresponding log files instantly (usual behavior:
            storing output in RAM until command finishes and flushing afterwards). This will result in empty logfiles
            for commands that do not output anything, as the programm has no way of knowing that in advance
    -m, --move-back      
            move all the files from execution_path to calling directory after all commands finish. CAUTION: This will
            move all files and subdirectorys of the execution directory!
            
            If option tmp_dir is used, it will only move the newly create sub directories, in which the commands were
            executed
    -n, --no-log         
            Ignore stdout and stderr of commands, don't create log files. The --print flag will ignore this option

        --print          
            Print output to stdout and stderr instead of creating a logfile for each command
            
            Note: print writes output instantly, without buffering, similar to the behavior one can get for the logfiles
            with the --instant-log flag
        --u64            
            Changes behavior of $RANDOM to be exchanged for an u64 instead, i.e., an 64 bit unsigned integer

    -V, --version        
            Prints version information


OPTIONS:
    -e, --execution-path <execution-path>    
            where should the command be executed? Default: Current directroy

    -j <j>                                   
            Number of commands that are run in parallel. If not given the program will try to figure out the appropriate
            ammount itself
    -l, --logname <logname>                  
            Name of the logfiles created for each command. If the flag --print is not set each command will print a
            logfile called {logname}_{command_index}.stdout and .stderr These will be created whenever a command
            finishes, if said command did output anything to stdout (stderr) [default: log]
    -s, --seed <seed>                        
            Seed for the random number generator used to replace $RANDOM. If nothing is given, the seed will be
            generated from entropy
    -t, --tmp-dir <tmp-dir>                  
            Stem name of temporary directories that are created in the execution directory. All commands will be run in
            their respective temporary directory instead. Every command gets a unique directory, as the stem name is
            appended with the execution index that corresponds to the line number in the command file (though blank
            lines or lines starting with # are not counted)
            
            The option move_back will now move the temporary directory of a command each time the corresponding command
            finishes. This is useful if the commands you want to execute may produce output that would interfere with
            one another, e.g., by having the same names

ARGS:
    <command-file>    
            (path to) file of which every line is to be executed in the shell (sh)
```