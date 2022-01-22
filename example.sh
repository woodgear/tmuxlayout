#!/bin/sh
tmux kill-session -t sample
tmux new-session -d -s sample
tmux rename-window 'window-1'
tmux splitw -h
tmux select-layout tiled
tmux splitw -h
tmux select-layout tiled
tmux selectp -t 0
tmux send-keys 'cd ~/' 'C-m'
tmux select-pane -T 'panel-1'
tmux send-keys 'export ENVA=b' 'C-m'
tmux send-keys 'export ENVB=10' 'C-m'
tmux send-keys 'echo "a1"' 'C-m'
tmux send-keys 'echo "b1"' 'C-m'
tmux selectp -t 1
tmux send-keys 'cd ~/' 'C-m'
tmux select-pane -T 'panel-2'
tmux send-keys 'echo "a2"' 'C-m'
tmux send-keys 'echo "b2"' 'C-m'
tmux selectp -t 2
tmux send-keys 'cd ~/' 'C-m'
tmux select-pane -T 'panel-3'
tmux send-keys 'echo "a3"' 'C-m'
tmux -2 attach-session -d