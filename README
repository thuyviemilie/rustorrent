# README

## RUSTORRENT PROJECT

The goal of this project is to create a program allowing to download and upload
torrents, following the BitTorrent protocol. It's a Rust-based project.

## How to compile it

`cargo build --release`

## How to run it

Usage: `cargo run -- [options] --torrent FILE`

Where:
- `--torrent` or `-t` precedes the FILE path to the .torrent file
- [options]:
-- `--pretty-print-file` or `-p` to pretty print file(s) in JSON format;
-- `--dump-peers` or `-d` to display peers ip and port returned by the tracker;
-- `--verbose` or `-v` to display all the network communications with the
peers.

Please be patient, it can take a while sometimes.

## How does it work

Mainly following the topic written by Jules Aubert U ACU 2018, feel free to
read his PDF of the Rustorrent project.
Or you can generate a detailed documentation out of code with the following
command: `cargo doc --open`

## Current state

Among the given files, our Bittorent client only works with
kali-linux-2023.3-installer-netinst-amd64.iso.torrent file since the client treats
only HTTPS protocol (UDP protocol has not been considered yet).
For now, the client can download pieces individually: he communicates with
a tracker to get a list of peer and download their pieces. Howewer, sometimes
obsolete peers block our client if we try to download them all. For
demonstration purpose, we have commented the `download::all` function l.303 in
main.rs.
The current state of our project permits to download a unique piece by specifying
its number. For instance, for kali-linux we ask for the piece number 0
(l.206, src/main.rs). Feel free to change it to see that we are able to
download any existing pieces... but not always at the first try! ^.^'

## Eventual errors

Being working with old torrent files, some peers does not seem to be active anymore.
Therefore, there is a good chance that you will encounter some of these following errors:

```bash
thread 'main' panicked at src/peers.rs:28:72:
called `Result::unwrap()` on an `Err` value: Os { code: 111, kind: ConnectionRefused, message: "Connection refused" }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

```bash
thread 'main' panicked at src/peers.rs:28:72:
called `Result::unwrap()` on an `Err` value: Os { code: 110, kind: TimedOut, message: "Connection timed out" }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

```bash
thread 'main' panicked at src/peers.rs:28:72:
called `Result::unwrap()` on an `Err` value: Os { code: 113, kind: HostUnreachable, message: "No route to host" }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```



                                                                         ~-----~
                                                                      /===--  ---~~~
                                                                /==~- --   -    ---~~~
                                                             /=----         ~~_  --(  '
                                                          /=----               \__~
     '                                                ~-~~      ~~~~        ~~~--\~'
     \\                                             /~--    ~~~--   -~     (     '
      `\                                           / ~~------~     ~~\   (
      \ '                                         /~/             ~~~ \ \(
      ``~\                                        |`_          ~~-~     )\
       '-~                                       |` ` _       ~~         '
       \ -~\                                      \\    \ _ _/
       `` ~~=\                                     ||   _ :(
        \  ~~=\__                                _//   ( `|'
        ``    , ~\--~=\                         / '    (   '
         \`    } ~ ~~ -~=\   _~_               // :_  / '
         |    ,          _~-'   '~~__-_--_/\--/     \ (
          \  ,_--_     _/              \/   \/-~ .   \
           )/      /\ / /\   ,~,                    \_  "~_
           ,      { ( _ )'} ~ - \_    ~\              "\   ~
                  /'' ''  )~ \~_ ~\   )->          _,       "
                 (\  _/)''}  /\~_ ~  /~(          /          }
                <``  >;,,/   {{\~__ {{{ '        ,   ,       ;
               {o_o }_/         '~__  _          "  :       ,"
               {/"\_)             \~__ ~\_      '  {        /~\
               ,/!                 '~__ _-~     :  '      ,"  ~
              (''`                  /,'~___~    | /     ,"  \ ~'
             '/, )                 (-)  '~____~";     ,"     , }
           /,')                    / \         /  ,~-"       '~'
       (  ''/                     / ( '       /  /          '~'
    ~ ~  ,, /) ,                 (/( \)      ( -)          /~'
  (  ~~ )`  ~}                   '  \)'     _/ /           ~'
 { |) /`,--.(  }'                    '     (  /          /~'
(` ~ ( c|~~| `}   )                        '/:\         ,'
 ~ )/``) )) '|),                          (/ | \)                 -smaug
  (` (-~(( `~`'  )                        ' (/ '
   `~'    )'`')                              '
     ` ``
      ___     ___
     .i .-'   `-. i.
   .'   `/     \'  _`.
   |,-../,o   o,\.' `|
(| | R / '_\ /_' \ S | |)
 \\\ U(_.'.'"`.`._)T ///
  \\`._(..:   :..)_.'//
   \`.__\ .:-:. /__.'/
    `-i-->.___.<--i-'
    .'.-'/.=^=.\`-.`.
   /.'  //     \\  `.\
  ||   ||       ||   ||
  \)   ||       ||  (/
       \)       (/
