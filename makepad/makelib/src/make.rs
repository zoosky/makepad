use makehub::*;
use crate::process::*;
use serde::{Deserialize};
use std::sync::{Arc, Mutex};
use serde_json::{Result};

pub struct Make {
    pub hub_client: HubClient,
    pub processes: Arc<Mutex<Vec<MakeProcess>>>
}

pub struct MakeProcess{
    uid:HubUid,
    _process:Process,
    _thread: Option<std::thread::JoinHandle<()>>
}

impl Make {
    pub fn proc<F>(mut event_handler: F)
    where F: FnMut(&mut Make, HubToClientMsg) {
        let key = [1u8, 2u8, 3u8, 4u8];
        
        // lets wait for a server announce
        let address = HubClient::wait_for_announce(&key).expect("cannot wait for announce");
        
        // ok now connect to that address
        let hub_client = HubClient::connect_to_hub(&key, address).expect("cannot connect to hub");
        
        let mut make = Make {
            hub_client: hub_client,
            processes: Arc::new(Mutex::new(Vec::<MakeProcess>::new()))
        };
        
        // lets transmit a BuildServer ack
        make.hub_client.tx_write.send(ClientToHubMsg {
            to: HubMsgTo::All,
            msg: HubMsg::ConnectBuild
        }).expect("Cannot send login");
        
        // this is the main messageloop, on rx
        while let Ok(htc) = make.hub_client.rx_read.recv() {
            // we just call the thing.
            event_handler(&mut make, htc);
        }
    }
    
    pub fn default(&mut self, htc:HubToClientMsg){
        match htc.msg{
            HubMsg::CargoCheck{uid, target, ..} => {
                self.cargo(uid,&["check", "-p", &target]);
            },
            HubMsg::ListWorkspace{uid}=>{
                self.list_workspace(uid, "./");
            },
            _=>()
        }
    }
    
    pub fn cargo(&mut self, uid:HubUid, args: &[&str]){
        // lets start a thread
        let mut extargs = args.to_vec();
        extargs.push("--message-format=json");
        let mut process = Process::start("cargo", &extargs, "./").expect("Cannot start process");
        
        // we now need to start a subprocess and parse the cargo output.
        let tx_write = self.hub_client.tx_write.clone();

        let rx_line = process.rx_line.take().unwrap();

        let processes = Arc::clone(&self.processes);
        
        let thread = std::thread::spawn(move || {
            while let Ok(line) = rx_line.recv(){
                if let Some(line) = line{
                    
                    // lets parse the line
                    let parsed: Result<RustcCompilerMessage> = serde_json::from_str(&line);
                    match parsed {
                        Err(err) => println!("Json Parse Error {:?} {}", err, line),
                        Ok(rcm) => {
                            // here we convert the parsed message
                            
                        }
                    }
                    
                    tx_write.send(ClientToHubMsg{
                        to: HubMsgTo::UI,
                        msg: HubMsg::CargoMsg{
                            uid:uid,
                            msg:CargoMsg::Warning{msg:"hi".to_string()}
                        }
                    }).expect("tx_write fail");
                }
                else{ // process terminated
                    break;
                }
            }

            // process ends as well
            tx_write.send(ClientToHubMsg{
                to: HubMsgTo::UI,
                msg: HubMsg::CargoDone{
                    uid:uid,
                }
            }).expect("tx_write fail");
            
            // remove process from process list
            if let Ok(mut processes) = processes.lock() {
                if let Some(index) = processes.iter().position(|p| p.uid == uid){
                    processes.remove(index);
                }
            };
        });
        
        if let Ok(mut processes) = self.processes.lock() {
            processes.push(MakeProcess {
                _process:process,
                uid:uid,
                _thread: Some(thread)
            });
            // wait for the thread to terminate?
            
        };
    }
    
    pub fn cargo_has_targets(&mut self, uid: HubUid, tgt: &[&'static str]){
        self.hub_client.tx_write.send(ClientToHubMsg {
            to: HubMsgTo::UI,
            msg: HubMsg::CargoHasTargets {
                uid: uid,
                targets: tgt.iter().map( | v | v.to_string()).collect()
            }
        }).expect("cannot send message");
    }
    
    pub fn list_workspace(&mut self, uid:HubUid, path:&str){

    }
}

// rust compiler output json structs
#[derive(Clone, Deserialize, Default)]
pub struct RustcTarget {
    kind: Vec<String>,
    crate_types: Vec<String>,
    name: String,
    src_path: String,
    edition: String
}

#[derive(Clone, Deserialize, Default)]
pub struct RustcText {
    text: String,
    highlight_start: u32,
    highlight_end: u32
}

#[derive(Clone, Deserialize, Default)]
pub struct RustcSpan {
    file_name: String,
    byte_start: u32,
    byte_end: u32,
    line_start: u32,
    line_end: u32,
    column_start: u32,
    column_end: u32,
    is_primary: bool,
    text: Vec<RustcText>,
    label: Option<String>,
    suggested_replacement: Option<String>,
    sugggested_applicability: Option<String>,
    expansion: Option<Box<RustcExpansion>>,
    level: Option<String>
}

#[derive(Clone, Deserialize, Default)]
pub struct RustcExpansion {
    span: Option<RustcSpan>,
    macro_decl_name: String,
    def_site_span: Option<RustcSpan>
}

#[derive(Clone, Deserialize, Default)]
pub struct RustcCode {
    code: String,
    explanation: Option<String>
}

#[derive(Clone, Deserialize, Default)]
pub struct RustcMessage {
    message: String,
    code: Option<RustcCode>,
    level: String,
    spans: Vec<RustcSpan>,
    children: Vec<RustcMessage>,
    rendered: Option<String>
}

#[derive(Clone, Deserialize, Default)]
pub struct RustcProfile {
    opt_level: String,
    debuginfo: Option<u32>,
    debug_assertions: bool,
    overflow_checks: bool,
    test: bool
}

#[derive(Clone, Deserialize, Default)]
pub struct RustcCompilerMessage {
    reason: String,
    package_id: String,
    target: Option<RustcTarget>,
    message: Option<RustcMessage>,
    profile: Option<RustcProfile>,
    features: Option<Vec<String>>,
    filenames: Option<Vec<String>>,
    executable: Option<String>,
    fresh: Option<bool>
}

/*#[derive(Clone, Deserialize, Default)]
pub struct RustcCompilerArtifact {
    reason: String,
    package_id: String,
    target: RustcTarget,
    profile: RustcProfile,
    features: Vec<String>,
    filenames: Vec<String>,
    executable: Option<String>,
    fresh: bool
}*/
