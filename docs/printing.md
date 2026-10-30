# Printing

Programs that `uses Printer` print with `Writeln(Lst, ...)`. There's no
printer port any more, so where the output goes is up to you, through the
`PASCRUST_PRINTER` environment variable:

```sh
# The default: append everything printed to printer.txt, in the directory
# the program started in.
pascrust run report.pas

# Append to a file of your choice.
PASCRUST_PRINTER=/tmp/report.txt pascrust run report.pas

# Send it to a real printer (macOS and Linux), or to any other command.
PASCRUST_PRINTER='|lp' pascrust run report.pas
PASCRUST_PRINTER='|lp -d office' pascrust run report.pas
PASCRUST_PRINTER='|less' pascrust run report.pas
```

The quotes matter, because `|` means a pipe to the shell. A command is run
by `sh`, starts the first time the program prints, and gets the output on
its standard input. When the program ends, its input is closed and PascRust
waits for it to finish, so the whole job is sent. A program that never
prints creates no file and runs no command. Output goes through exactly as
written, form feeds (`Write(Lst, #12)`) included. A procedure that writes
to a `var f: Text` parameter can be handed `Lst` (or `Output`) to print a
report instead of showing it. Old programs often wrote
printer control codes for an Epson or IBM printer, and those reach your
printer unchanged too.
