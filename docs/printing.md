# Printing

Programs that `uses Printer` print with `Writeln(Lst, ...)`. There's no
printer port any more, so where the output goes is up to you, through the
`RUSCAL_PRINTER` environment variable:

```sh
# The default: append everything printed to printer.txt, in the directory
# the program started in.
ruscal run report.pas

# Append to a file of your choice.
RUSCAL_PRINTER=/tmp/report.txt ruscal run report.pas

# Send it to a real printer (macOS and Linux), or to any other command.
RUSCAL_PRINTER='|lp' ruscal run report.pas
RUSCAL_PRINTER='|lp -d office' ruscal run report.pas
RUSCAL_PRINTER='|less' ruscal run report.pas
```

The quotes matter, because `|` means a pipe to the shell. A command is run
by `sh`, starts the first time the program prints, and gets the output on
its standard input. When the program ends, its input is closed and Ruscal
waits for it to finish, so the whole job is sent. A program that never
prints creates no file and runs no command. Output goes through exactly as
written, form feeds (`Write(Lst, #12)`) included. A procedure that writes
to a `var f: Text` parameter can be handed `Lst` (or `Output`) to print a
report instead of showing it. Old programs often wrote
printer control codes for an Epson or IBM printer, and those reach your
printer unchanged too.
