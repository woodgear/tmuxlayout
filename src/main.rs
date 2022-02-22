use clap::Parser;
use context_attribute::context;
use failure::ResultExt;
use filepath::FilePath;
use log::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::process::Command;
use sugars::hmap;
use tempfile::tempfile;

#[context(fn)]
fn init_log(config: &str) -> Result<(), failure::Error> {
    use log4rs::{
        config::{Config, Deserializers, RawConfig},
        Logger,
    };

    use serde_yaml;
    let log4rs_config: RawConfig = serde_yaml::from_str(config)?;

    let (appenders, _) = log4rs_config.appenders_lossy(&Deserializers::default());

    let (config, _) = Config::builder()
        .appenders(appenders)
        .loggers(log4rs_config.loggers())
        .build_lossy(log4rs_config.root());

    let log4rs_logger = Logger::new(config);

    let logger = Box::new(log4rs_logger);
    log::set_max_level(log::LevelFilter::Info);
    log::set_boxed_logger(logger)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TmuxLayout {
    name: String,
    #[serde(default)]
    root: String,
    #[serde(default)]
    restart_if_exists: bool,
    #[serde(default)]
    on_start: Vec<String>,
    windows: BTreeMap<String, TmuxWindow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TmuxWindow {
    #[serde(default)]
    root: String,
    panes: BTreeMap<String, TmuxPane>,
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
struct TmuxPane {
    #[serde(default)]
    root: String,
    #[serde(default)]
    env: HashMap<String, String>,
    #[serde(default)]
    cmds: Vec<String>,
}

fn do_parse(layout: &TmuxLayout, out: &mut Vec<String>) {
    out.push("#!/bin/sh".to_string());
    do_prepare(layout, out);
    let mut current_windows_index = 0;
    for (name, windows) in layout.windows.iter() {
        current_windows_index += 1;
        let is_first_window = current_windows_index == 1;

        if is_first_window {
            out.push(format!(r#"tmux rename-window '{}'"#, name));
        } else {
            out.push(format!("tmux new-window -n '{}'", name));
        }

        if !layout.root.is_empty() && windows.root.is_empty() {
            let mut windows = windows.clone();
            windows.root = layout.root.clone();
            do_window(name, &windows, out);
        } else {
            do_window(name, windows, out);
        }
    }
    // out.push("tmux -2 attach-session -d".to_string());
    if layout.windows.len() > 1 {
        out.push("tmux select-window -t 0".to_string());
    }
}

fn do_prepare(layout: &TmuxLayout, out: &mut Vec<String>) {
    for c in layout.on_start.iter() {
        out.push(format!("{}", c));
    }

    if layout.restart_if_exists {
        out.push(format!("tmux kill-session -t {}", layout.name));
    }

    out.push(format!("tmux new-session -d -s {}", layout.name));
}

fn do_window(_name: &str, windows: &TmuxWindow, out: &mut Vec<String>) {
    do_preare_panel_tiled(windows.panes.len(), out);
    let mut index = 0;

    for (name, pane) in windows.panes.iter() {
        if !windows.root.is_empty() && pane.root.is_empty() {
            let mut pane = pane.clone();
            pane.root = windows.root.clone();
            do_pane(name, index, &pane, out);
        } else {
            do_pane(name, index, pane, out);
        }
        index = index + 1;
    }
}

fn send_keys(cmd: &str) -> String {
    return format!(
        r#"cmd=$(cat <<EOF
{}
EOF
); tmux send-keys "$cmd" 'C-m'
    "#,
        cmd
    );
}

fn do_pane(name: &str, index: u32, pane: &TmuxPane, out: &mut Vec<String>) {
    out.push(format!("tmux selectp -t {}", index));
    if !pane.root.is_empty() {
        out.push(format!("tmux send-keys 'cd {}' 'C-m'", pane.root));
    }
    out.push(format!("tmux select-pane -T '{}'", name));
    for (key, val) in &pane.env {
        out.push(format!("tmux send-keys 'export {}={}' 'C-m'", key, val));
    }

    for cmd in pane.cmds.iter() {
        out.push(send_keys(cmd));
    }
}

fn do_preare_panel_tiled(pane_count: usize, out: &mut Vec<String>) {
    if pane_count > 1 {
        for i in 0..pane_count - 1 {
            out.push("tmux splitw -h".to_string());
            if i % 2 == 0 {
                // make sure pan is not to small
                out.push("tmux select-layout tiled".to_string());
            }
        }
    }
    out.push("tmux select-layout tiled".to_string());
}

use std::fs;
use structopt::StructOpt;
#[derive(StructOpt, Debug)]
#[structopt(name = "tmuxlayout")]
struct Config {
    #[structopt(short = "y")]
    yml_path: String,
    #[structopt(short = "o")]
    out: bool,
    #[structopt(short = "r")]
    run: bool,
}

use std::path::Path;
fn app() -> Result<(), failure::Error> {
    init_log(std::include_str!("./log.yaml"))?;
    let config = Config::from_args();
    let path = Path::new(&config.yml_path).canonicalize()?;
    let file_name = path.file_stem().unwrap().to_string_lossy().to_string();
    let mut bash_path = path.parent().unwrap().join(format!("{}.sh", file_name));
    let yml = fs::read_to_string(&path)?;

    let layout: TmuxLayout = serde_yaml::from_str(&yml)?;
    let mut cmds = vec![];
    do_parse(&layout, &mut cmds);

    let bash = cmds.join("\n");
    if !config.out {
        bash_path = tempfile()?.path()?
    }

    fs::write(bash_path.clone(), bash)?;
    Command::new("chmod")
        .args([
            "a+x".to_string(),
            bash_path
                .clone()
                .into_os_string()
                .to_string_lossy()
                .to_string(),
        ])
        .spawn()?
        .wait();
    if config.run {
        Command::new(bash_path).spawn()?.wait();
        Command::new("tmux")
            .args(["attach", "-dt", &layout.name])
            .spawn()?
            .wait();
    }

    Ok(())
}

fn main() {
    if let Err(e) = app() {
        error!("{:?}", e);
        std::process::exit(-1);
    }
}

#[cfg(test)]
mod tests {
    use sugar::btreemap;

    use super::*;

    fn assert_parse(yaml: &str, bash: &str) {
        let ret_bash = parse(yaml).unwrap();
        println!("{}", ret_bash.join("\n"));
        assert_eq!(bash, ret_bash.join("\n"));
        // let bash: Vec<String> = bash.lines().map(|s| s.to_string()).collect();

        // let ret_bash = parse(yaml).unwrap();
        // for i in 0..ret_bash.len() {
        //     assert_eq!(bash[i], ret_bash[i]);
        // }
    }
    fn parse(yml: &str) -> Result<Vec<String>, failure::Error> {
        let layout: TmuxLayout = serde_yaml::from_str(yml)?;
        let mut out = vec![];
        let _ret = do_parse(&layout, &mut out);
        return Ok(out);
    }

    #[test]
    fn test_layout() {
        fn assert_yaml(yaml: &str, layout: TmuxLayout) {
            let ret_layout: TmuxLayout = serde_yaml::from_str(yaml).unwrap();
            assert_eq!(ret_layout, layout);
        }

        let example_config = r#"name: sample
root: ~/
restart_if_exists: true
windows:
    window-1:
        panes:
            panel-1:
                env: 
                    ENVA: b
                    ENVB: 10
                root: ./1
                cmds:
                    - echo "a1"
                    - echo "b1"
            panel-2:
                cmds:
                    - echo "a2"
                    - echo "b2"        
            panel-3:
                root: ./3

"#;
        assert_yaml(
            example_config,
            TmuxLayout {
                name: "sample".to_string(),
                root: "~/".to_string(),
                restart_if_exists: true,
                on_start: vec![],
                windows: btreemap! {
                    "window-1".to_owned() => TmuxWindow {
                        root:"".to_string(),
                        panes:btreemap!{
                            "panel-1".to_string() => TmuxPane{
                                root:"./1".to_string(),
                                env:hmap!{"ENVA".to_string()=>"b".to_string(), "ENVB".to_string()=> "10".to_string()},
                                cmds:vec![r#"echo "a1""#.to_string(),r#"echo "b1""#.to_string()]
                            },
                            "panel-2".to_string() => TmuxPane{
                                root:"".to_string(),
                                env:Default::default(),
                                cmds:vec![r#"echo "a2""#.to_string(),r#"echo "b2""#.to_string()]
                            },
                            "panel-3".to_string() => TmuxPane{
                                root:"./3".to_string(),
                                env:Default::default(),
                                cmds:vec![]
                            }


                        }
                    },

                },
            },
        )
    }

    #[test]
    fn test_parse() {
        let example_config = r#"name: sample
root: ~/
restart_if_exists: true
on_start:
        - echo "start"
windows:
    window-1:
        root: ./w1
        panes:                        
           panel-1:  
                env:
                    a: b
                root: ./1              
                cmds:
                    - vim
           panel-2:                
                cmds:
                    - vim

"#;
        let expect_bash = r#"#!/bin/sh
echo "start"
tmux kill-session -t sample
tmux new-session -d -s sample
tmux rename-window 'window-1'
tmux splitw -h
tmux select-layout tiled
tmux select-layout tiled
tmux selectp -t 0
tmux send-keys 'cd ./1' 'C-m'
tmux select-pane -T 'panel-1'
tmux send-keys 'export a=b' 'C-m'
cmd=$(cat <<EOF
vim
EOF
); tmux send-keys "$cmd" 'C-m'
    
tmux selectp -t 1
tmux send-keys 'cd ./w1' 'C-m'
tmux select-pane -T 'panel-2'
cmd=$(cat <<EOF
vim
EOF
); tmux send-keys "$cmd" 'C-m'
    "#;

        assert_parse(example_config, expect_bash);
    }

    fn assert_preare_panel_tiled(count: usize, bash: &str) {
        let mut out = vec![];
        do_preare_panel_tiled(count, &mut out);
        let bash: Vec<String> = bash.lines().map(|s| s.to_string()).collect();
        for i in 0..out.len() {
            assert_eq!(out[i], bash[i]);
        }
    }
    #[test]
    fn test_do_preare_panel_tiled() {
        assert_preare_panel_tiled(
            4,
            r#"tmux splitw -h
tmux select-layout tiled
tmux splitw -h
tmux splitw -h
tmux select-layout tiled
tmux select-layout tiled"#,
        );
    }
    #[test]
    fn test_send_keys() {
        let cmd = r#"tmux-send-key-to-pane "eyes" C-c  '  sudo bpftrace -v  ./actions/http-handle-event.trace' C-m"#;
        let ret = send_keys(cmd);
        println!("{}", ret);
        assert_eq!(
            ret,
            r#"cmd=$(cat <<EOF
tmux-send-key-to-pane "eyes" C-c  '  sudo bpftrace -v  ./actions/http-handle-event.trace' C-m
EOF
); tmux send-keys "$cmd" 'C-m'
    "#
        )
    }
}
