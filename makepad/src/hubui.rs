use crate::hubmsg::*;
use std::sync::{mpsc, Arc, Mutex};

pub struct HubUI {
    pub hub_log:HubLog,
    pub thread: Option<std::thread::JoinHandle<()>>,
    pub route_send: HubRouteSend,
    pub htc_msgs_arc: Arc<Mutex<Vec<FromHubMsg>>>,
}


impl HubUI {
    
    pub fn start_hub_ui_direct<F>(hub_router:&mut HubRouter,event_handler: &'static F)->HubUI
    where F: Fn(&mut HubWorkspace, FromHubMsg) -> Result<(), HubWsError> + Clone + Send {
        // lets create a tx pair, and add a route
        let (tx_write, rx_write) = mpsc::channel::<FromHubMsg>();
        
        let htc_msgs_arc = Arc::new(Mutex::new(Vec::new()));
        let route_send = hub_router.connect_direct(HubRouteType::UI, tx_write);
        
        let thread = {
            let htc_msgs_arc = Arc::clone(&htc_msgs_arc);
            let route_send = route_send.clone();
            let signal = signal.clone();
            std::thread::spawn(move || {
                // lets transmit a BuildServer ack
                route_send.send(ToHubMsg {
                    to: HubMsgTo::All,
                    msg: HubMsg::ConnectUI
                });
                
                // this is the main messageloop, on rx
                while let Ok(htc) = rx_write.recv() {
                    let mut do_signal = false;
                    if let Ok(mut htc_msgs) = htc_msgs_arc.lock(){
                        if htc_msgs.len() == 0{
                            do_signal = true;
                        }
                        htc_msgs.push(htc);
                    } 
                    if do_signal{
                        event_handler();
                    }
                }
            })
        };

        HubUI{
            signal: signal,
            thread: Some(thread),
            htc_msgs_arc: htc_msgs_arc,
            hub_log: HubLog::None,
            route_send: route_send
        }
    }
    
    pub fn start_hub_ui_networked(cx:&mut Cx, key: &[u8], hub_log:HubLog)->HubUI{

        let key = key.to_vec();

        let route_send = HubRouteSend::Networked{
            uid_alloc: Arc::new(Mutex::new(0)),
            tx_write_arc:  Arc::new(Mutex::new(None)),
            own_addr_arc:  Arc::new(Mutex::new(None))
        };

        let htc_msgs_arc = Arc::new(Mutex::new(Vec::new()));
        let signal = cx.new_signal();
        
        // lets start a thread that stays connected
        let thread = {
            let htc_msgs_arc = Arc::clone(&htc_msgs_arc);
            let hub_log = hub_log.clone();
            let signal = signal.clone();
            
            std::thread::spawn(move || {
                loop {
                    
                    hub_log.log("HubUI waiting for hub announcement..");
                    
                    // lets wait for a server announce
                    let address = HubClient::wait_for_announce(&key).expect("cannot wait for announce");
                    
                    hub_log.msg("HubUI got announce, connecting to ", &address);
                    
                    // ok now connect to that address
                    let hub_client = HubClient::connect_to_server(&key, address, hub_log.clone()).expect("cannot connect to hub");
                    
                    hub_log.msg("HubUI connected to ", &hub_client.server_addr);
                    
                    let route_send = hub_client.get_route_send();
                    
                    // lets transmit a BuildServer ack
                    route_send.send(ToHubMsg {
                        to: HubMsgTo::All,
                        msg: HubMsg::ConnectUI
                    });
                    
                    // this is the main messageloop, on rx
                    while let Ok(htc) = hub_client.rx_read.as_ref().unwrap().recv() {
                        let restart_connection = if let HubMsg::ConnectionError(_e) = &htc.msg{
                            true
                        }
                        else{
                            false
                        };
                        let mut do_signal = false;
                        if let Ok(mut htc_msgs) = htc_msgs_arc.lock(){
                            if htc_msgs.len() == 0{
                                do_signal = true;
                            }
                            htc_msgs.push(htc);
                        } 
                        if do_signal{
                            Cx::post_signal(signal, 0);
                        }
                        if restart_connection {
                            break
                        }
                    }
                }
            })
        };

        HubUI{
            signal: signal,
            hub_log:hub_log.clone(),
            thread: Some(thread),
            htc_msgs_arc: htc_msgs_arc,
            route_send: route_send
        }
    }
    
    pub fn process_signal(&mut self, se:&SignalEvent)->Option<Vec<FromHubMsg>>{
         if self.signal.is_signal(se) {
            if let Ok(mut htc_msgs) = self.htc_msgs_arc.lock() {
                let mut msgs = Vec::new();
                std::mem::swap(&mut msgs, &mut htc_msgs);
                return Some(msgs);
            }
        }
        None
    }
}