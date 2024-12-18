while true; do
    scene=$(ls -d /home/user/repos/paperview-rs/scenes/* | shuf -n 1)

    timeout 600 ./paperview-rs --bg "1366:768:0:0:${scene}:60"
done

