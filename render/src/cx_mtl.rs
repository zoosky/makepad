use std::mem;

//use cocoa::base::{id};
use cocoa::appkit::{NSView};
use cocoa::foundation::{NSAutoreleasePool, NSUInteger, NSRange};
use core_graphics::geometry::CGSize;
use objc::{msg_send, sel, sel_impl};
use objc::runtime::YES;
use metal::*;
use crate::cx_cocoa::*;
use crate::cx::*;

impl Cx {
    
    pub fn exec_draw_list(&mut self, draw_list_id: usize, dpi_factor: f32, device: &Device, encoder: &RenderCommandEncoderRef) {
        
        // tad ugly otherwise the borrow checker locks 'self' and we can't recur
        let draw_calls_len = self.draw_lists[draw_list_id].draw_calls_len;
        for draw_call_id in 0..draw_calls_len {
            let sub_list_id = self.draw_lists[draw_list_id].draw_calls[draw_call_id].sub_list_id;
            if sub_list_id != 0 {
                self.exec_draw_list(sub_list_id, dpi_factor, device, encoder);
            }
            else {
                let draw_list = &mut self.draw_lists[draw_list_id];
                draw_list.set_clipping_uniforms();
                draw_list.set_dpi_factor(dpi_factor);
                draw_list.platform.uni_dl.update_with_f32_data(device, &draw_list.uniforms);
                let draw_call = &mut draw_list.draw_calls[draw_call_id];
                let sh = &self.shaders[draw_call.shader_id];
                let shc = &self.compiled_shaders[draw_call.shader_id];
                
                if draw_call.instance_dirty {
                    draw_call.instance_dirty = false;
                    // update the instance buffer data
                    draw_call.platform.inst_vbuf.update_with_f32_data(device, &draw_call.instance);
                }
                if draw_call.uniforms_dirty {
                    draw_call.uniforms_dirty = false;
                    //draw_call.platform.uni_dr.update_with_f32_data(device, &draw_call.uniforms);
                }
                
                // lets verify our instance_offset is not disaligned
                let instances = (draw_call.instance.len() / shc.instance_slots) as u64;
                let pipeline_state = &shc.pipeline_state;
                encoder.set_render_pipeline_state(pipeline_state);
                if let Some(buf) = &shc.geom_vbuf.multi_buffer_read().buffer {encoder.set_vertex_buffer(0, Some(&buf), 0);}
                else {println!("Drawing error: geom_vbuf None")}
                
                if let Some(buf) = &draw_call.platform.inst_vbuf.multi_buffer_read().buffer {encoder.set_vertex_buffer(1, Some(&buf), 0);}
                else {println!("Drawing error: inst_vbuf None")}
                
                encoder.set_vertex_bytes(2, (self.uniforms.len() * 4) as u64, self.uniforms.as_ptr() as *const std::ffi::c_void);
                encoder.set_vertex_bytes(3, (draw_list.uniforms.len() * 4) as u64, draw_list.uniforms.as_ptr() as *const std::ffi::c_void);
                encoder.set_vertex_bytes(4, (draw_call.uniforms.len() * 4) as u64, draw_call.uniforms.as_ptr() as *const std::ffi::c_void);
                encoder.set_fragment_bytes(0, (self.uniforms.len() * 4) as u64, self.uniforms.as_ptr() as *const std::ffi::c_void);
                encoder.set_fragment_bytes(1, (draw_list.uniforms.len() * 4) as u64, draw_list.uniforms.as_ptr() as *const std::ffi::c_void);
                encoder.set_fragment_bytes(2, (draw_call.uniforms.len() * 4) as u64, draw_call.uniforms.as_ptr() as *const std::ffi::c_void);
                
                // lets set our textures
                for (i, texture_id) in draw_call.textures_2d.iter().enumerate() {
                    let tex = &mut self.textures_2d[*texture_id as usize];
                    if tex.dirty {
                        tex.upload_to_device(device);
                    }
                    if let Some(mtltex) = &tex.mtltexture {
                        encoder.set_fragment_texture(i as NSUInteger, Some(&mtltex));
                        encoder.set_vertex_texture(i as NSUInteger, Some(&mtltex));
                    }
                }
                
                if let Some(buf) = &shc.geom_ibuf.multi_buffer_read().buffer {
                    encoder.draw_indexed_primitives_instanced(
                        MTLPrimitiveType::Triangle,
                        sh.geometry_indices.len() as u64,
                        // Index Count
                        MTLIndexType::UInt32,
                        // indexType,
                        &buf,
                        // index buffer
                        0,
                        // index buffer offset
                        instances,
                        // instance count
                    )
                }
                else {println!("Drawing error: geom_ibuf None")}
            }
        }
    }
    
    pub fn repaint(
        &mut self,
        root_draw_list_id: usize,
        dpi_factor: f32,
        layer: &CoreAnimationLayer,
        device: &Device,
        command_queue: &CommandQueue
    ) {
        let pool = unsafe {NSAutoreleasePool::new(cocoa::base::nil)};
        //let command_buffer = command_queue.new_command_buffer();
        if let Some(drawable) = layer.next_drawable() {
            //self.prepare_frame();
            
            let render_pass_descriptor = RenderPassDescriptor::new();
            
            let color_attachment = render_pass_descriptor.color_attachments().object_at(0).unwrap();
            color_attachment.set_texture(Some(drawable.texture()));
            color_attachment.set_load_action(MTLLoadAction::Clear);
            color_attachment.set_clear_color(MTLClearColor::new(
                self.clear_color.r as f64,
                self.clear_color.g as f64,
                self.clear_color.b as f64,
                self.clear_color.a as f64
            ));
            color_attachment.set_store_action(MTLStoreAction::Store);
            
            let command_buffer = command_queue.new_command_buffer();
            
            render_pass_descriptor.color_attachments().object_at(0).unwrap().set_load_action(MTLLoadAction::Clear);
            
            let encoder = command_buffer.new_render_command_encoder(&render_pass_descriptor);
            //let encoder = parallel_encoder.render_command_encoder();
            
            //self.platform.uni_cx.update_with_f32_data(&device, &self.uniforms);
            
            // ok now we should call our render thing
            self.exec_draw_list(root_draw_list_id, dpi_factor, &device, encoder);
            /*
            match &self.debug_area{
                Area::All=>self.debug_draw_tree_recur(0, 0),
                Area::Instance(ia)=>self.debug_draw_tree_recur(ia.draw_list_id, 0),
                Area::DrawList(dl)=>self.debug_draw_tree_recur(dl.draw_list_id, 0),
                _=>()
            }*/
            
            encoder.end_encoding();
            //parallel_encoder.end_encoding();
            
            command_buffer.present_drawable(&drawable);
            command_buffer.commit();
            
            //if wait {
            //    command_buffer.wait_until_completed();
            // }
        }
        unsafe {
            msg_send![pool, release];
        }
    }
    
    pub fn event_loop<F>(&mut self, mut event_handler: F)
    where F: FnMut(&mut Cx, &mut Event),
    {
        self.feature = "mtl".to_string();
        
        let mut cocoa_app = CocoaApp::new();
        
        cocoa_app.init();
        
        let device = Device::system_default();
        
        let command_queue = device.new_command_queue();
        
        let mut render_windows: Vec<CocoaRenderWindow> = Vec::new();
        
        self.mtl_compile_all_shaders(&device);
        
        self.load_binary_deps_from_file();
        
        self.call_event_handler(&mut event_handler, &mut Event::Construct);
        
        self.redraw_child_area(Area::All);
        
        cocoa_app.event_loop( | cocoa_app, events | {
            let mut paint_dirty = false;
            for mut event in events {
                
                match &mut event {
                    Event::FingerHover(_) => {
                        self.hover_mouse_cursor = None;
                    },
                    Event::FingerUp(_) => {
                        self.down_mouse_cursor = None;
                    },
                    Event::WindowCloseRequested(_cr) => {
                    },
                    Event::FingerDown(fe) => {
                        // lets set the finger tap count
                        fe.tap_count = self.process_tap_count(fe.digit, fe.abs, fe.time);
                    },
                    Event::KeyDown(ke) => {
                        self.process_key_down(ke.clone());
                        if ke.key_code == KeyCode::PrintScreen {
                            if ke.modifiers.control {
                                self.panic_redraw = true;
                            }
                            else {
                                self.panic_now = true;
                            }
                        }
                    },
                    Event::KeyUp(ke) => {
                        self.process_key_up(&ke);
                    },
                    Event::AppFocusLost => {
                        self.call_all_keys_up(&mut event_handler);
                    },
                    _ => ()
                };
                
                match &event {
                    Event::WindowGeomChange(re) => { // do this here because mac
                        for render_window in &mut render_windows {
                            if render_window.window_id == re.window_id {
                                render_window.window_geom = re.new_geom.clone();
                                self.windows[re.window_id].window_geom = re.new_geom.clone();
                                // redraw just this windows root draw list
                                self.redraw_window_id(re.window_id);
                                break;
                            }
                        }
                        // ok lets not redraw all, just this window
                        self.call_event_handler(&mut event_handler, &mut event);
                    },
                    Event::Paint => {
                        
                        if self.playing_anim_areas.len() != 0 {
                            let time = cocoa_app.time_now();
                            // keeps the error as low as possible
                            self.call_animation_event(&mut event_handler, time);
                        }
                        
                        if self.frame_callbacks.len() != 0 {
                            let time = cocoa_app.time_now();
                            // keeps the error as low as possible
                            self.call_frame_event(&mut event_handler, time);
                        }
                        
                        self.call_signals_before_draw(&mut event_handler);
                        
                        // call redraw event
                        if self.redraw_child_areas.len()>0 || self.redraw_parent_areas.len()>0 {
                            //let time_start = cocoa_window.time_now();
                            self.call_draw_event(&mut event_handler);
                            // let time_end = cocoa_window.time_now();
                            // println!("Redraw took: {}", (time_end - time_start));
                        }
                        
                        self.process_desktop_file_read_requests(&mut event_handler);
                        
                        self.call_signals_after_draw(&mut event_handler);
                        
                        // construct or destruct windows
                        for (index, window) in self.windows.iter_mut().enumerate() {
                            
                            window.window_state = match &window.window_state {
                                CxWindowState::Create {inner_size, position, title} => {
                                    // lets create a platformwindow
                                    let render_window = CocoaRenderWindow::new(index, &device, cocoa_app, *inner_size, *position, &title);
                                    window.window_geom = render_window.window_geom.clone();
                                    render_windows.push(render_window);
                                    for render_window in &mut render_windows {
                                        render_window.cocoa_window.update_ptrs();
                                    }
                                    CxWindowState::Created
                                },
                                CxWindowState::Destroy => {
                                    CxWindowState::Destroyed
                                },
                                CxWindowState::Created => CxWindowState::Created,
                                CxWindowState::Destroyed => CxWindowState::Destroyed
                            }
                        }
                        
                        // set a cursor
                        if !self.down_mouse_cursor.is_none() {
                            cocoa_app.set_mouse_cursor(self.down_mouse_cursor.as_ref().unwrap().clone())
                        }
                        else if !self.hover_mouse_cursor.is_none() {
                            cocoa_app.set_mouse_cursor(self.hover_mouse_cursor.as_ref().unwrap().clone())
                        }
                        else {
                            cocoa_app.set_mouse_cursor(MouseCursor::Default)
                        }
                        
                        if let Some(set_ime_position) = self.platform.set_ime_position {
                            self.platform.set_ime_position = None;
                            for render_window in &mut render_windows {
                                render_window.cocoa_window.set_ime_spot(set_ime_position);
                            }
                        }
                        
                        while self.platform.start_timer.len()>0 {
                            let (timer_id, interval, repeats) = self.platform.start_timer.pop().unwrap();
                            cocoa_app.start_timer(timer_id, interval, repeats);
                        }
                        
                        while self.platform.stop_timer.len()>0 {
                            let timer_id = self.platform.stop_timer.pop().unwrap();
                            cocoa_app.stop_timer(timer_id);
                        }
                        
                        // repaint al windows we need to repaint
                        let mut windows_need_repaint = 0;
                        for render_window in &mut render_windows {
                            if self.windows[render_window.window_id].paint_dirty {
                                windows_need_repaint += 1;
                            }
                        }
                        
                        if windows_need_repaint > 0 {
                            for render_window in &mut render_windows {
                                if self.windows[render_window.window_id].paint_dirty {
                                    windows_need_repaint -= 1;
                                    // only vsync the last window. needs fixing properly with a thread. unfortunately
                                    render_window.set_vsync_enable(windows_need_repaint == 0);
                                    render_window.set_buffer_count(
                                        if render_window.window_geom.is_fullscreen {3}else {2}
                                    );
                                    
                                    self.set_projection_matrix(&render_window.window_geom);
                                    if let Some(root_draw_list_id) = self.windows[render_window.window_id].root_draw_list_id {
                                        self.repaint(
                                            root_draw_list_id,
                                            render_window.window_geom.dpi_factor,
                                            &render_window.core_animation_layer,
                                            &device,
                                            &command_queue
                                        );
                                    }
                                    // do this after the draw to create jump-free window resizing
                                    if render_window.resize_core_animation_layer() {
                                        self.windows[render_window.window_id].paint_dirty = true;
                                        paint_dirty = true;
                                    }
                                    else {
                                        self.windows[render_window.window_id].paint_dirty = false;
                                    }
                                }
                            }
                        }
                    },
                    Event::None => {
                    },
                    _ => {
                        self.call_event_handler(&mut event_handler, &mut event);
                    }
                }
                match &event {
                    Event::FingerUp(fe) => { // decapture automatically
                        self.captured_fingers[fe.digit] = Area::Empty;
                    },
                    Event::WindowClosed(_wc) => {
                        cocoa_app.terminate_event_loop();
                    },
                    _ => {}
                }
            }
            
            if self.playing_anim_areas.len() == 0 && self.redraw_parent_areas.len() == 0 && self.redraw_child_areas.len() == 0 && self.frame_callbacks.len() == 0 && !paint_dirty {
                CocoaLoopState::Block
            }
            else {
                CocoaLoopState::Poll
            }
        });
    }
    
    pub fn show_text_ime(&mut self, x: f32, y: f32) {
        self.platform.set_ime_position = Some(Vec2 {x: x, y: y});
    }
    
    pub fn hide_text_ime(&mut self) {
    }
    
    pub fn set_window_outer_size(&mut self, size: Vec2) {
        self.platform.set_window_outer_size = Some(size);
    }
    
    pub fn set_window_position(&mut self, pos: Vec2) {
        self.platform.set_window_position = Some(pos);
    }
    
    pub fn start_timer(&mut self, interval: f64, repeats: bool) -> Timer {
        self.timer_id += 1;
        self.platform.start_timer.push((self.timer_id, interval, repeats));
        Timer {timer_id: self.timer_id}
    }
    
    pub fn stop_timer(&mut self, timer: &mut Timer) {
        if timer.timer_id != 0 {
            self.platform.stop_timer.push(timer.timer_id);
            timer.timer_id = 0;
        }
    }
    
    pub fn send_signal(signal: Signal, value: u64) {
        CocoaApp::post_signal(signal.signal_id, value);
    }
    
    //pub fn send_custom_event_before_draw(&mut self, id:u64, message:u64){
    //   self.custom_before_draw.push((id, message));
    //}
}

#[derive(Clone, Default)]
pub struct CxPlatform {
    pub uni_cx: MetalBuffer,
    pub post_id: u64,
    pub set_window_position: Option<Vec2>,
    pub set_window_outer_size: Option<Vec2>,
    pub set_ime_position: Option<Vec2>,
    pub start_timer: Vec<(u64, f64, bool)>,
    pub stop_timer: Vec<(u64)>,
    pub text_clipboard_response: Option<String>,
    pub desktop: CxDesktop,
}

#[derive(Clone)]
pub struct CocoaRenderWindow {
    pub window_id: usize,
    pub window_geom: WindowGeom,
    pub cal_size: Vec2,
    pub core_animation_layer: CoreAnimationLayer,
    pub cocoa_window: CocoaWindow
}

impl CocoaRenderWindow {
    fn new(window_id: usize, device: &Device, cocoa_app: &mut CocoaApp, inner_size: Vec2, position: Option<Vec2>, title: &str) -> CocoaRenderWindow {
        
        let core_animation_layer = CoreAnimationLayer::new();
        
        let mut cocoa_window = CocoaWindow::new(cocoa_app, window_id);
        
        cocoa_window.init(title, inner_size, position);
        
        core_animation_layer.set_device(&device);
        core_animation_layer.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
        core_animation_layer.set_presents_with_transaction(false);
        
        unsafe {
            //msg_send![layer, displaySyncEnabled:false];
            let count: u64 = 2;
            msg_send![core_animation_layer, setMaximumDrawableCount: count];
            msg_send![core_animation_layer, setDisplaySyncEnabled: true];
        }
        
        unsafe {
            let view = cocoa_window.view;
            view.setWantsBestResolutionOpenGLSurface_(YES);
            view.setWantsLayer(YES);
            view.setLayer(mem::transmute(core_animation_layer.as_ref()));
        }
        
        CocoaRenderWindow {
            window_id,
            cal_size: Vec2::zero(),
            core_animation_layer,
            window_geom: cocoa_window.get_window_geom(),
            cocoa_window,
        }
    }
    
    fn set_vsync_enable(&mut self, enable: bool) {
        unsafe {
            msg_send![self.core_animation_layer, setDisplaySyncEnabled: enable];
        }
    }
    
    fn set_buffer_count(&mut self, count: u64) {
        unsafe {
            msg_send![self.core_animation_layer, setMaximumDrawableCount: count];
        }
    }
    
    fn resize_core_animation_layer(&mut self) -> bool {
        let cal_size = Vec2 {
            x: self.window_geom.inner_size.x * self.window_geom.dpi_factor,
            y: self.window_geom.inner_size.y * self.window_geom.dpi_factor
        };
        if self.cal_size != cal_size {
            self.cal_size = cal_size;
            self.core_animation_layer.set_drawable_size(CGSize::new(cal_size.x as f64, cal_size.y as f64));
            true
        }
        else {
            false
        }
    }
}

#[derive(Clone, Default)]
pub struct DrawListPlatform {
    pub uni_dl: MetalBuffer
}

#[derive(Default, Clone, Debug)]
pub struct DrawCallPlatform {
    pub uni_dr: MetalBuffer,
    pub inst_vbuf: MetalBuffer
}

#[derive(Default, Clone, Debug)]
pub struct MultiMetalBuffer {
    pub buffer: Option<metal::Buffer>,
    pub size: usize,
    pub used: usize
}

#[derive(Default, Clone, Debug)]
pub struct MetalBuffer {
    pub last_written: usize,
    pub multi1: MultiMetalBuffer,
    pub multi2: MultiMetalBuffer,
    pub multi3: MultiMetalBuffer,
    pub multi4: MultiMetalBuffer,
    pub multi5: MultiMetalBuffer,
    pub multi6: MultiMetalBuffer,
}

impl MetalBuffer {
    pub fn multi_buffer_read(&self) -> &MultiMetalBuffer {
        match self.last_written {
            0 => &self.multi1,
            1 => &self.multi2,
            2 => &self.multi3,
            3 => &self.multi4,
            4 => &self.multi5,
            _ => &self.multi6,
        }
    }
    
    pub fn multi_buffer_write(&mut self) -> &mut MultiMetalBuffer {
        self.last_written = (self.last_written + 1) % 3;
        match self.last_written {
            0 => &mut self.multi1,
            1 => &mut self.multi2,
            2 => &mut self.multi3,
            3 => &mut self.multi4,
            4 => &mut self.multi5,
            _ => &mut self.multi6,
        }
    }
    
    pub fn update_with_f32_data(&mut self, device: &Device, data: &Vec<f32>) {
        let elem = self.multi_buffer_write();
        if elem.size < data.len() {
            elem.buffer = None;
        }
        if let None = &elem.buffer {
            elem.buffer = Some(
                device.new_buffer(
                    (data.len() * std::mem::size_of::<f32>()) as u64,
                    MTLResourceOptions::CPUCacheModeDefaultCache
                )
            );
            elem.size = data.len()
        }
        if let Some(buffer) = &elem.buffer {
            let p = buffer.contents();
            
            unsafe {
                std::ptr::copy(data.as_ptr(), p as *mut f32, data.len());
            }
            buffer.did_modify_range(NSRange::new(0 as u64, (data.len() * std::mem::size_of::<f32>()) as u64));
        }
        elem.used = data.len()
    }
    
    pub fn update_with_u32_data(&mut self, device: &Device, data: &Vec<u32>) {
        let elem = self.multi_buffer_write();
        if elem.size < data.len() {
            elem.buffer = None;
        }
        if let None = &elem.buffer {
            elem.buffer = Some(
                device.new_buffer(
                    (data.len() * std::mem::size_of::<u32>()) as u64,
                    MTLResourceOptions::CPUCacheModeDefaultCache
                )
            );
            elem.size = data.len()
        }
        if let Some(buffer) = &elem.buffer {
            let p = buffer.contents();
            
            unsafe {
                std::ptr::copy(data.as_ptr(), p as *mut u32, data.len());
            }
            buffer.did_modify_range(NSRange::new(0 as u64, (data.len() * std::mem::size_of::<u32>()) as u64));
        }
        elem.used = data.len()
    }
}


#[derive(Default, Clone)]
pub struct Texture2D {
    pub texture_id: usize,
    pub dirty: bool,
    pub image: Vec<u32>,
    pub width: usize,
    pub height: usize,
    pub mtltexture: Option<metal::Texture>
}

impl Texture2D {
    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.image.resize((width * height) as usize, 0);
        self.dirty = true;
    }
    
    pub fn upload_to_device(&mut self, device: &Device) {
        let desc = TextureDescriptor::new();
        desc.set_texture_type(MTLTextureType::D2);
        desc.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
        desc.set_width(self.width as u64);
        desc.set_height(self.height as u64);
        desc.set_storage_mode(MTLStorageMode::Managed);
        //desc.set_mipmap_level_count(1);
        //desc.set_depth(1);
        //desc.set_sample_count(4);
        let tex = device.new_texture(&desc);
        
        let region = MTLRegion {
            origin: MTLOrigin {x: 0, y: 0, z: 0},
            size: MTLSize {width: self.width as u64, height: self.height as u64, depth: 1}
        };
        tex.replace_region(region, 0, (self.width * mem::size_of::<u32>()) as u64, self.image.as_ptr() as *const std::ffi::c_void);
        
        //image_buf.did_modify_range(NSRange::new(0 as u64, (self.image.len() * mem::size_of::<u32>()) as u64));
        
        self.mtltexture = Some(tex);
        self.dirty = false;
        
    }
}

use closefds::*;
use std::process::{Command, Child, Stdio};
use std::os::unix::process::{CommandExt};

pub fn spawn_process_command(cmd: &str, args: &[&str], current_dir: &str) -> Result<Child, std::io::Error> {
    unsafe {Command::new(cmd) .args(args) .pre_exec(close_fds_on_exec(vec![0, 1, 2]).unwrap()) .stdout(Stdio::piped()) .stderr(Stdio::piped()) .current_dir(current_dir) .spawn()}
}
