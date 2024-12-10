# PAPERVIEW-RS

Paperview rewrite in rust with compositor support

![](screenshot.png)

<h2> Table of Contents </h2>

- [Quick Start](#quick-start)
    - [Installation](#installation)
    - [Single Monitor Use](#single-monitor-use)
    - [Multi Monitor Use](#multi-monitor-use)
    - [Running as a Background Process](#running-as-a-background-process)
    - [Custom Scenes](#creating-custom-scenes)
- [License](#license)


## Quick Start

### Installation

Clone the repository:
```bash
    git clone https://github.com/eatmynerds/paperview-rs.git
    cd paperview-rs
    cargo build --release
    mv target/release/paperview-rs .
```

## Single Monitor Use

```bash
    ./paperview-rs --path FOLDER 
```

## Multi Monitor Use

paperview-rs can handle any number of monitors. Each monitor will render wallpapers independently.

    ./paperview-rs --path FOLDER 



## Running as a Background Process

Run paperview-rs in the backgrond using `&`:

    ./paperview-rs --path FOLDER &

To terminate the background process:

    killall paperview-rs

## Creating Custom Scenes

Creating a custom BMP scene folder from a GIF requires imagemagick.
For example, to create a castle scene folder from a castle.gif:

    mkdir castle
    mv castle.gif castle
    cd castle
    convert -coalesce castle.gif out.bmp
    rm castle.gif

## Random Animated Wallpapers at Startup

Assuming a scenes folder containing a number of scene folders is present in the home folder,
run the following snippet as a background process within .xinitrc before running `startx`,
or simply execute it after X11 is running:

    while true
    do
        scene=$(ls -d ~/scenes/*/ | shuf -n 1)
        timeout 600 paperview-rs --path $scene 
    done



## License
Licensed under [MIT](./LICENSE).
