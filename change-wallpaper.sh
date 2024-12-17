#!/bin/sh

while true; do
    scene=$(ls -d $HOME/repos/paperview-rs/scenes/* | shuf -n 1)

    timeout 30 ./paperview-rs --bg "1920:1080:0:0:${scene}:60"
done

