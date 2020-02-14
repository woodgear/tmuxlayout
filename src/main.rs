use context_attribute::context;
use failure::ResultExt;
use log::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use sugar::btreemap;

#[context(fn)]
fn init_log(config: &str) -> Result<(), failure::Error> {
    use log4rs::{
        config::Config,
        file::{Deserializers, RawConfig},
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
    cmds: Vec<String>,
}

fn do_parse(layout: &TmuxLayout, out: &mut Vec<String>) {
    out.push("#!/bin/sh".to_string());
    do_preare(layout, out);
    for (name, windows) in layout.windows.iter() {
        if !layout.root.is_empty() && windows.root.is_empty() {
            let mut windows = windows.clone();
            windows.root = layout.root.clone();
            do_window(name, &windows, out);
        } else {
            do_window(name, windows, out);
        }
    }
    out.push("tmux -2 attach-session -d".to_string());
}

fn do_preare(layout: &TmuxLayout, out: &mut Vec<String>) {
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

    for (_name, pane) in windows.panes.iter() {
        if !windows.root.is_empty() && pane.root.is_empty() {
            let mut pane = pane.clone();
            pane.root = windows.root.clone();
            do_pane(index, &pane, out);
        } else {
            do_pane(index, pane, out);
        }
        index = index + 1;
    }
}

fn do_pane(index: u32, pane: &TmuxPane, out: &mut Vec<String>) {
    out.push(format!("tmux selectp -t {}", index));
    if !pane.root.is_empty() {
        out.push(format!("tmux send-keys 'cd {}' 'C-m'", pane.root));
    }

    for cmd in pane.cmds.iter() {
        out.push(format!("tmux send-keys '{}' 'C-m'", cmd));
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

fn parse(yml: &str) -> Result<Vec<String>, failure::Error> {
    let layout: TmuxLayout = serde_yaml::from_str(yml)?;
    let mut out = vec![];
    let _ret = do_parse(&layout, &mut out);
    return Ok(out);
}
use std::fs;
use structopt::StructOpt;
#[derive(StructOpt, Debug)]
#[structopt(name = "tmuxlayout")]
struct Config {
    yml_path: String,
}

use std::path::Path;
fn app() -> Result<(), failure::Error> {
    init_log(std::include_str!("./log.yaml"))?;
    let config = Config::from_args();
    let path = Path::new(&config.yml_path).canonicalize()?;
    let file_name = path.file_stem().unwrap().to_string_lossy().to_string();
    let bash_path = path.parent().unwrap().join(format!("{}.sh", file_name));
    let yml = fs::read_to_string(&path)?;
    info!("yaml path is {:?} sh path is {:?}", path, bash_path);
    let cmds = parse(&yml)?;
    let bash = cmds.join("\n");
    fs::write(bash_path, bash)?;
    Ok(())
}

fn main() {
    if let Err(e) = app() {
        println!("{:?}", e);
        std::process::exit(-1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_parse(yaml: &str, bash: &str) {
        let bash: Vec<String> = bash.lines().map(|s| s.to_string()).collect();
        let ret_bash = parse(yaml).unwrap();
        for i in 0..ret_bash.len() {
            assert_eq!(bash[i], ret_bash[i]);
        }
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
                                cmds:vec![r#"echo "a1""#.to_string(),r#"echo "b1""#.to_string()]
                            },
                            "panel-2".to_string() => TmuxPane{
                                root:"".to_string(),
                                cmds:vec![r#"echo "a2""#.to_string(),r#"echo "b2""#.to_string()]
                            },
                            "panel-3".to_string() => TmuxPane{
                                root:"./3".to_string(),
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
                root: ./1              
                cmds:
                    - vim
           panel-2:                
                cmds:
                    - vim
           panel-3:                
                cmds:
                    - vim
           panel-4:                
                cmds:
                    - vim

"#;
        let expect_bash = r#"#!/bin/sh
echo "start"
tmux kill-session -t sample
tmux new-session -d -s sample
tmux splitw -h
tmux select-layout tiled
tmux splitw -h
tmux splitw -h
tmux select-layout tiled
tmux select-layout tiled
tmux selectp -t 0
tmux send-keys 'cd ./1' 'C-m'
tmux send-keys 'vim' 'C-m'
tmux selectp -t 1
tmux send-keys 'cd ./w1' 'C-m'
tmux send-keys 'vim' 'C-m'
tmux selectp -t 2
tmux send-keys 'cd ./w1' 'C-m'
tmux send-keys 'vim' 'C-m'
tmux selectp -t 3
tmux send-keys 'cd ./w1' 'C-m'
tmux send-keys 'vim' 'C-m'
tmux -2 attach-session -d"#;

        let _ret = parse(example_config).unwrap();
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
        // assert_preare_panel_tiled(1,r#"tmux select-layout tiled"#);

        // assert_preare_panel_tiled(2,r#"tmux splitw -h
        // tmux select-layout tiled
        // "#);
    }
}
