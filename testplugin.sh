#!/bin/sh
tmux new-session -d -s test-net
tmux splitw -h
tmux select-layout tiled
tmux splitw -h
tmux splitw -h
tmux select-layout tiled
tmux select-layout tiled
tmux selectp -t 0
tmux send-keys 'cd "E:\sm\work\edr\client"' 'C-m'
tmux send-keys 'echo "p1"' 'C-m'
tmux selectp -t 1
tmux send-keys 'cd "E:\sm\pv\s-config"' 'C-m'
tmux send-keys 'simple-http-server.exe ./ -p 12345' 'C-m'
tmux selectp -t 2
tmux send-keys 'cd "E:\sm\work\edr\edr-mock-server\testplugins"' 'C-m'
tmux send-keys 'echo "p1"' 'C-m'
tmux selectp -t 3
tmux send-keys 'cd "E:\sm\work\edr\edr-mock-server\testplugins"' 'C-m'
tmux send-keys 'tail -f Hello.txt' 'C-m'
tmux new-window
tmux splitw -h
tmux select-layout tiled
tmux splitw -h
tmux splitw -h
tmux select-layout tiled
tmux select-layout tiled
tmux selectp -t 0
tmux send-keys 'cd "E:\sm\work\edr\client"' 'C-m'
tmux send-keys 'echo "p1"' 'C-m'
tmux selectp -t 1
tmux send-keys 'cd "E:\sm\pv\s-config"' 'C-m'
tmux send-keys 'simple-http-server.exe ./ -p 12345' 'C-m'
tmux selectp -t 2
tmux send-keys 'cd "E:\sm\work\edr\edr-mock-server\testplugins"' 'C-m'
tmux send-keys 'echo "p1"' 'C-m'
tmux selectp -t 3
tmux send-keys 'cd "E:\sm\work\edr\edr-mock-server\testplugins"' 'C-m'
tmux send-keys 'tail -f Hello.txt' 'C-m'
tmux -2 attach-session -d
tmux select-window -t 0