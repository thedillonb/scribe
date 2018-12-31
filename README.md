# Scribe
Takes its stdin and writes it to a file while managing file rotation after a given size.
This application is ment to be used in conjunction with another application which is actively writing to its stdout.

The following will pipe its stdout and stderr into scribe and have scribe write that input to a log file with a maximum size of 5120 bytes in each file and a maximum of 5 files.

```bash
./prog_that_outputs_a_lot 2>&1 | scribe /var/log/prog.log --max-file-size 5120 --max-rotations 5
```

## Why
There are many other applications that serve the same purpose. The primary goal of this project was to learn Rust.
The application just happened to be useful and straightforward. 
