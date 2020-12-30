# features
* generate bash file which call tmux directly so it works on windows
* use rust so you could download it without other dependency
* use yaml to manager

# config
```yml
name: session_name
restart_if_exists: true
windows:
    windows_name:
        panes:
            pane-name1:
                root: ./p1
                env:
                    a:b
                cmds:
                    - echo $a
            pane-name2:
                root: ./p2
                cmds:
                    - echo "p2"
```
# how to use
```
tmuxlayout ./xxx.yml
```
it will generate a file named as xxx.sh you just need to call this bash file.
# how to install
cargo install --git https://github.com/woodgear/tmuxlayout.git

# todo
* set border title
* add clippy
* add ci
* add support of mutli windows
* add support of mutli config format like json