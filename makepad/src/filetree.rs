use render::*;
use widget::*;

#[derive(Clone)]
pub struct FileTree {
    pub view: ScrollView,
    pub drag_view: View,
    pub _drag_move: Option<FingerMoveEvent>,
    pub drag_bg: Quad,
    pub drag_bg_layout: Layout,
    pub node_bg: Quad,
    pub filler: Quad,
    pub tree_folder_color: Color,
    pub tree_file_color: Color,
    pub tree_text: Text,
    pub root_node: FileNode,
    pub animator: Animator,
    pub row_height: f32,
    pub font_size: f32,
    pub row_padding: Padding
}

#[derive(Clone, PartialEq)]
pub enum FileTreeEvent {
    None,
    DragMove {fe: FingerMoveEvent, paths: Vec<String>},
    DragCancel,
    DragEnd {fe: FingerUpEvent, paths: Vec<String>},
    DragOut,
    SelectFile {path: String},
    SelectFolder {path: String}
}

#[derive(Clone)]
pub enum NodeState {
    Open,
    Opening(f64),
    Closing(f64),
    Closed
}

#[derive(Clone)]
pub struct NodeDraw {
    animator: Animator,
    marked: u64
}

#[derive(Clone)]
pub enum FileNode {
    File {name: String, draw: Option<NodeDraw>},
    Folder {name: String, draw: Option<NodeDraw>, state: NodeState, folder: Vec<FileNode>}
}

impl FileNode {
    fn get_draw<'a>(&'a mut self) -> &'a mut Option<NodeDraw> {
        match self {
            FileNode::File {draw, ..} => draw,
            FileNode::Folder {draw, ..} => draw
        }
    }
    
    fn is_open(&self) -> bool {
        match self {
            FileNode::File {..} => false,
            FileNode::Folder {state, ..} => match state {
                NodeState::Opening(..) => true,
                NodeState::Open => true,
                _ => false
            }
        }
    }
    
    fn name(&self) -> String {
        match self {
            FileNode::File {name, ..} => name.clone(),
            FileNode::Folder {name, ..} => name.clone()
        }
    }
}

struct StackEntry<'a> {
    counter: usize,
    index: usize,
    len: usize,
    closing: bool,
    node: &'a mut FileNode
}

pub struct FileWalker<'a>
{
    stack: Vec<StackEntry<'a>>,
}

// this flattens out recursion into an iterator. unfortunately needs unsafe. come on. thats not nice
impl<'a> FileWalker<'a> {
    pub fn new(root_node: &'a mut FileNode) -> FileWalker<'a> {
        return FileWalker {stack: vec![StackEntry {counter: 1, closing: false, index: 0, len: 0, node: root_node}]};
    }
    
    pub fn current_path(&self) -> String {
        // the current stack top returned as path
        let mut path = String::new();
        for i in 0..self.stack.len() {
            if i > 1 {
                path.push_str("/");
            }
            path.push_str(
                &self.stack[i].node.name()
            );
        };
        path
    }
    
    pub fn current_closing(&self) -> bool {
        if let Some(stack_top) = self.stack.last() {
            stack_top.closing
        }
        else {
            false
        }
    }
    
    pub fn walk(&mut self) -> Option<(usize, usize, usize, &mut FileNode)> {
        // lets get the current item on the stack
        let stack_len = self.stack.len();
        let push_or_pop = if let Some(stack_top) = self.stack.last_mut() {
            // return item 'count'
            match stack_top.node {
                FileNode::File {..} => {
                    stack_top.counter += 1;
                    if stack_top.counter == 1 {
                        return Some((stack_len - 1, stack_top.index, stack_top.len, unsafe {std::mem::transmute(&mut *stack_top.node)}));
                    }
                    None // pop stack
                },
                FileNode::Folder {folder, state, ..} => {
                    stack_top.counter += 1;
                    if stack_top.counter == 1 { // return self
                        return Some((stack_len - 1, stack_top.index, stack_top.len, unsafe {std::mem::transmute(&mut *stack_top.node)}));
                    }
                    else {
                        let child_index = stack_top.counter - 2;
                        let opened = if let NodeState::Closed = state {false} else {true};
                        let closing = if let NodeState::Closing(_) = state {true} else {stack_top.closing};
                        if opened && child_index < folder.len() { // child on stack
                            Some(StackEntry {counter: 0, closing: closing, index: child_index, len: folder.len(), node: unsafe {std::mem::transmute(&mut folder[child_index])}})
                        }
                        else {
                            None // pop stack
                        }
                    }
                }
            }
        }
        else {
            None
        };
        if let Some(item) = push_or_pop {
            self.stack.push(item);
            return self.walk();
        }
        else if self.stack.len() > 0 {
            self.stack.pop();
            return self.walk();
        }
        return None;
    }
}

impl FileTree {
    pub fn style(cx: &mut Cx) -> Self {
        Self {
            row_height: 20.,
            font_size: 8.0,
            row_padding: Padding {l: 5., t: 0., r: 0., b: 1.},
            root_node: FileNode::Folder {name: "".to_string(), state: NodeState::Open, draw: None, folder: vec![
                FileNode::File {name: "loading...".to_string(), draw: None},
            ]},
            node_bg: Quad::style(cx),
            drag_bg: Quad {
                color: cx.color("bg_marked"),
                shader: cx.add_shader(Self::def_drag_bg_shader(), "FileTree.drag_bg"),
                ..Quad::style(cx)
            },
            drag_bg_layout: Layout {
                padding: Padding {l: 5., t: 5., r: 5., b: 5.},
                width: Width::Compute,
                height: Height::Compute,
                ..Default::default()
            },
            filler: Quad {
                color: cx.color("icon_color"),
                z: 0.001,
                shader: cx.add_shader(Self::def_filler_shader(), "FileTree.filler"),
                ..Quad::style(cx)
            },
            tree_folder_color: cx.color("text_selected_focus"),
            tree_file_color: cx.color("text_deselected_focus"),
            tree_text: Text {z: 0.001, top_drop:1.3, ..Text::style(cx)},
            view: ScrollView {
                scroll_v: Some(ScrollBar {
                    smoothing: Some(0.25),
                    ..ScrollBar::style(cx)
                }),
                ..ScrollView::style(cx)
            },
            drag_view: View {
                is_overlay: true,
                ..View::style(cx)
            },
            animator: Animator::new(Anim::empty()),
            _drag_move: None,
        }
    }
    
    pub fn def_drag_bg_shader() -> ShaderGen {
        Quad::def_quad_shader().compose(shader_ast!({
            fn pixel() -> vec4 {
                df_viewport(pos * vec2(w, h));
                df_box(0., 0., w, h, 2.);
                return df_fill(color);
            }
        }))
    }
    
    pub fn def_filler_shader() -> ShaderGen {
        Quad::def_quad_shader().compose(shader_ast!({
            
            let line_vec: vec2<Instance>;
            let anim_pos: float<Instance>;
            
            fn pixel() -> vec4 {
                df_viewport(pos * vec2(w, h));
                if anim_pos<-0.5 {
                    df_move_to(0.5 * w, line_vec.x * h);
                    df_line_to(0.5 * w, line_vec.y * h);
                    return df_stroke(color, 1.);
                }
                else { // its a folder
                    df_box(0. * w, 0.35 * h, 0.87 * w, 0.39 * h, 0.75);
                    df_box(0. * w, 0.28 * h, 0.5 * w, 0.3 * h, 1.);
                    df_union();
                    // ok so.
                    return df_fill(color);
                }
            }
        }))
    }
    /*
    pub fn load_from_ron(&mut self, cx: &mut Cx, ron_data: &str) {
        
        #[derive(Deserialize, Debug)]
        struct RonFolder {
            name: String,
            open: bool,
            files: Vec<RonFile>,
            folders: Vec<RonFolder>
        }
        
        #[derive(Deserialize, Debug)]
        struct RonFile {
            name: String
        }
        
        fn recur_walk(node: RonFolder) -> FileNode {
            let mut out = Vec::new();
            for folder in node.folders {
                out.push(recur_walk(folder));
            }
            for file in node.files {
                out.push(FileNode::File {
                    name: file.name,
                    draw: None
                })
            };
            FileNode::Folder {
                name: node.name,
                state: if node.open {NodeState::Open} else {NodeState::Closed},
                draw: None,
                folder: out
            }
        }
        
        if let Ok(value) =  ron::de::from_str(ron_data) {
            self.root_node = recur_walk(value);
        }
        self.view.redraw_view_area(cx);
    }*/
    
    pub fn clear_roots(&mut self, cx: &mut Cx, names: &Vec<String>) {
        self.root_node = FileNode::Folder {
            name: "".to_string(),
            draw: None,
            state: NodeState::Open,
            folder: names.iter().map( | v | FileNode::Folder {
                name: v.clone(),
                draw: None,
                state: NodeState::Open,
                folder: Vec::new()
            }).collect()
        };
        self.view.redraw_view_area(cx);
    }
    
    pub fn save_open_folders(&mut self) -> Vec<String> {
        let mut paths = Vec::new();
        fn recur_walk(node: &mut FileNode, base: &str, paths: &mut Vec<String>) {
            if node.is_open() {
                if let FileNode::Folder {folder, name, ..} = node {
                    let new_base = if name.len()>0 {
                        if base.len()>0 {format!("{}/{}", base, name)}else {name.to_string()}
                    }else {base.to_string()};
                    paths.push(new_base.clone());
                    for node in folder {
                        recur_walk(node, &new_base, paths)
                    }
                }
            }
        }
        recur_walk(&mut self.root_node, "", &mut paths);
        paths
    }
    
    pub fn load_open_folders(&mut self, cx: &mut Cx, paths: &Vec<String>) {
        
        fn recur_walk(node: &mut FileNode, base: &str, depth: usize, paths: &Vec<String>) {
            match node {
                FileNode::File {..} => (),
                FileNode::Folder {name, folder, state, ..} => {
                    let new_base = if name.len()>0 {
                        if base.len()>0 {format!("{}/{}", base, name)}else {name.to_string()}
                    }else {base.to_string()};
                    if depth == 0 || paths.iter().position( | v | *v == new_base).is_some() {
                        *state = NodeState::Open;
                        for node in folder {
                            recur_walk(node, &new_base, depth + 1, paths);
                        }
                    }
                    else {
                        *state = NodeState::Closed;
                    }
                }
            }
        }
        recur_walk(&mut self.root_node, "", 0, paths);
        self.view.redraw_view_area(cx);
    }
    
    pub fn get_default_anim(cx: &Cx, counter: usize, marked: bool) -> Anim {
        Anim::new(Play::Chain {duration: 0.01}, vec![
            Track::color(cx.id("bg.color"), Ease::Lin, vec![(1.0, if marked {cx.color("bg_marked")} else if counter & 1 == 0 {cx.color("bg_selected")}else {cx.color("bg_odd")})])
        ])
    }
    
    pub fn get_over_anim(cx: &Cx, counter: usize, marked: bool) -> Anim {
        let over_color = if marked {cx.color("bg_marked_over")} else if counter & 1 == 0 {cx.color("bg_selected_over")}else {cx.color("bg_odd_over")};
        Anim::new(Play::Cut {duration: 0.02}, vec![
            Track::color(cx.id("bg.color"), Ease::Lin, vec![
                (0., over_color),
                (1., over_color)
            ])
        ])
    }
    
    pub fn get_marked_paths(root: &mut FileNode) -> Vec<String> {
        let mut paths = Vec::new();
        let mut file_walker = FileWalker::new(root);
        // make a path set of all marked items
        while let Some((_depth, _index, _len, node)) = file_walker.walk() {
            let node_draw = if let Some(node_draw) = node.get_draw() {node_draw}else {continue};
            if node_draw.marked != 0 {
                paths.push(file_walker.current_path());
            }
        }
        paths
    }
    
    pub fn handle_file_tree(&mut self, cx: &mut Cx, event: &mut Event) -> FileTreeEvent {
       
        // alright. someone clicking on the tree items.
        let mut file_walker = FileWalker::new(&mut self.root_node);
        let mut counter = 0;
        self.view.handle_scroll_bars(cx, event);
        // todo, optimize this so events are not passed through 'all' of our tree elements
        // but filtered out somewhat based on a bounding rect
        let mut unmark_nodes = false;
        let mut drag_nodes = false;
        let mut drag_end: Option<FingerUpEvent> = None;
        let mut select_node = 0;
        while let Some((_depth, _index, _len, node)) = file_walker.walk() {
            // alright we haz a node. so now what.
            let is_filenode = if let FileNode::File {..} = node {true} else {false};
            
            let node_draw = if let Some(node_draw) = node.get_draw() {node_draw}else {continue};
            
            match event.hits(cx, node_draw.animator.area, HitOpt::default()) {
                Event::Animate(ae) => {
                    node_draw.animator.write_area(cx, node_draw.animator.area, "bg.", ae.time);
                },
                Event::AnimEnded(_)=>{
                    node_draw.animator.end();
                },
                Event::FingerDown(_fe) => {
                    // mark ourselves, unmark others
                    if is_filenode {
                        select_node = 1;
                    }
                    else {
                        select_node = 2;
                    }
                    node_draw.marked = cx.event_id;
                    
                    unmark_nodes = true;
                    node_draw.animator.play_anim(cx, Self::get_over_anim(cx, counter, node_draw.marked != 0));
                    
                    if let FileNode::Folder {state, ..} = node {
                        *state = match state {
                            NodeState::Opening(fac) => {
                                NodeState::Closing(1.0 - *fac)
                            },
                            NodeState::Closing(fac) => {
                                NodeState::Opening(1.0 - *fac)
                            },
                            NodeState::Open => {
                                NodeState::Closing(1.0)
                            },
                            NodeState::Closed => {
                                NodeState::Opening(1.0)
                            }
                        };
                        // start the redraw loop
                        self.view.redraw_view_area(cx);
                    }
                },
                Event::FingerUp(fe) => {
                    if !self._drag_move.is_none() {
                        drag_end = Some(fe);
                        // we now have to do the drop....
                        self.drag_view.redraw_view_area(cx);
                        //self._drag_move = None;
                    }
                },
                Event::FingerMove(fe) => {
                    cx.set_down_mouse_cursor(MouseCursor::Hand);
                    if self._drag_move.is_none() {
                        if fe.move_distance() > 10. {
                            self._drag_move = Some(fe);
                            self.view.redraw_view_area(cx);
                            self.drag_view.redraw_view_area(cx);
                        }
                    }
                    else {
                        self._drag_move = Some(fe);
                        self.view.redraw_view_area(cx);
                        self.drag_view.redraw_view_area(cx);
                    }
                    drag_nodes = true;
                },
                Event::FingerHover(fe) => {
                    cx.set_hover_mouse_cursor(MouseCursor::Hand);
                    match fe.hover_state {
                        HoverState::In => {
                            node_draw.animator.play_anim(cx, Self::get_over_anim(cx, counter, node_draw.marked != 0));
                        },
                        HoverState::Out => {
                            node_draw.animator.play_anim(cx, Self::get_default_anim(cx, counter, node_draw.marked != 0));
                        },
                        _ => ()
                    }
                },
                _ => ()
            }
            counter += 1;
        }
        
        //unmark non selected nodes and also set even/odd animations to make sure its rendered properly
        if unmark_nodes {
            let mut file_walker = FileWalker::new(&mut self.root_node);
            let mut counter = 0;
            while let Some((_depth, _index, _len, node)) = file_walker.walk() {
                if let Some(node_draw) = node.get_draw() {
                    if node_draw.marked != cx.event_id || node_draw.marked == 0 {
                        node_draw.marked = 0;
                        node_draw.animator.play_anim(cx, Self::get_default_anim(cx, counter, false));
                    }
                }
                if !file_walker.current_closing() {
                    counter += 1;
                }
            }
        }
        if let Some(fe) = drag_end {
            self._drag_move = None;
            let paths = Self::get_marked_paths(&mut self.root_node);
            if !self.view.get_view_area(cx).get_rect(cx, true).contains(fe.abs.x, fe.abs.y) {
                return FileTreeEvent::DragEnd {
                    fe: fe.clone(),
                    paths: paths
                };
            }
        }
        if drag_nodes {
            if let Some(fe) = &self._drag_move {
                // lets check if we are over our own filetree
                // ifso, we need to support moving files with directories
                let paths = Self::get_marked_paths(&mut self.root_node);
                if !self.view.get_view_area(cx).get_rect(cx, true).contains(fe.abs.x, fe.abs.y) {
                    return FileTreeEvent::DragMove {
                        fe: fe.clone(),
                        paths: paths
                    };
                }
                else {
                    return FileTreeEvent::DragCancel;
                }
            }
        };
        if select_node != 0 {
            let mut file_walker = FileWalker::new(&mut self.root_node);
            while let Some((_depth, _index, _len, node)) = file_walker.walk() {
                let node_draw = if let Some(node_draw) = node.get_draw() {node_draw}else {continue};
                if node_draw.marked != 0 {
                    if select_node == 1 {
                        return FileTreeEvent::SelectFile {
                            path: file_walker.current_path()
                        };
                    }
                    else {
                        return FileTreeEvent::SelectFolder {
                            path: file_walker.current_path()
                        };
                    }
                }
            }
        }
        FileTreeEvent::None
    }
    
    pub fn draw_file_tree(&mut self, cx: &mut Cx) {
        if self.view.begin_view(cx, Layout::default()).is_err() {return}
        
        let mut file_walker = FileWalker::new(&mut self.root_node);
        
        // lets draw the filetree
        let mut counter = 0;
        let mut scale_stack = Vec::new();
        let mut last_stack = Vec::new();
        scale_stack.push(1.0f64);
        
        while let Some((depth, index, len, node)) = file_walker.walk() {
            
            let is_first = index == 0;
            let is_last = index == len - 1;
            
            while depth < scale_stack.len() {
                scale_stack.pop();
                last_stack.pop();
            }
            let scale = scale_stack[depth - 1];
            
            // lets store the bg area in the tree
            let node_draw = node.get_draw();
            if node_draw.is_none() {
                *node_draw = Some(NodeDraw {
                    animator: Animator::new(Self::get_default_anim(cx, counter, false)),
                    marked: 0
                })
            }
            let node_draw = node_draw.as_mut().unwrap();
            
            // if we are NOT animating, we need to get change a default color.
            
            self.node_bg.color = node_draw.animator.last_color(cx.id("bg.color"));
            
            let inst = self.node_bg.begin_quad(cx, &Layout {
                width: Width::Fill,
                height: Height::Fix(self.row_height * scale as f32),
                align: Align::left_center(),
                padding: self.row_padding,
                ..Default::default()
            });
            node_draw.animator.update_area_refs(cx, inst.clone().into_area());
            let is_marked = node_draw.marked != 0;
            
            for i in 0..(depth - 1) {
                let quad_margin = Margin {l: 1., t: 0., r: 4., b: 0.};
                if i == depth - 2 { // our own thread.
                    let area = self.filler.draw_quad_walk(
                        cx,
                        Width::Fix(10.),
                        Height::Fill,
                        quad_margin
                    );
                    if is_last {
                        if is_first {
                            //line_vec
                            area.push_vec2(cx, Vec2 {x: 0.3, y: 0.7})
                        }
                        else {
                            //line_vec
                            area.push_vec2(cx, Vec2 {x: -0.2, y: 0.7})
                        }
                    }
                    else if is_first {
                        //line_vec
                        area.push_vec2(cx, Vec2 {x: -0.3, y: 1.2})
                    }
                    else {
                        //line_vec
                        area.push_vec2(cx, Vec2 {x: -0.2, y: 1.2});
                    }
                    //anim_pos
                    area.push_float(cx, -1.);
                }
                else {
                    let here_last = if last_stack.len()>1 {last_stack[i + 1]} else {false};
                    if here_last {
                        cx.walk_turtle(Width::Fix(10.), Height::Fill, quad_margin, None);
                    }
                    else {
                        let area = self.filler.draw_quad_walk(cx, Width::Fix(10.), Height::Fill, quad_margin);
                        //line_vec
                        area.push_vec2(cx, Vec2 {x: -0.2, y: 1.2});
                        //anim_pos
                        area.push_float(cx, -1.);
                    }
                }
            };
            self.filler.z = depth as f32 + 1.0;
            self.tree_text.z = depth as f32 + 1.0;
            self.tree_text.font_size = self.font_size;
            self.tree_text.font_scale = scale as f32;
            match node {
                FileNode::Folder {name, state, ..} => {
                    // draw the folder icon
                    let inst = self.filler.draw_quad_walk(cx, Width::Fix(14.), Height::Fill, Margin {l: 0., t: 0., r: 2., b: 0.});
                    inst.push_vec2(cx, Vec2::zero());
                    inst.push_float(cx, 1.);
                    // move the turtle down a bit
                    //cx.move_turtle(0., 3.5);
                    cx.turtle_align_y();
                    //cx.realign_turtle(Align::left_center(), false);
                    self.tree_text.color = self.tree_folder_color;
                    let wleft = cx.get_width_left() - 10.;
                    self.tree_text.wrapping = Wrapping::Ellipsis(wleft);
                    self.tree_text.draw_text(cx, name);
                    
                    let (new_scale, new_state) = match state {
                        NodeState::Opening(fac) => {
                            self.view.redraw_view_area(cx);
                            if *fac < 0.001 {
                                (1.0, NodeState::Open)
                            }
                            else {
                                (1.0 - *fac, NodeState::Opening(*fac * 0.7))
                            }
                        },
                        NodeState::Closing(fac) => {
                            self.view.redraw_view_area(cx);
                            if *fac < 0.001 {
                                (0.0, NodeState::Closed)
                            }
                            else {
                                (*fac, NodeState::Closing(*fac * 0.7))
                            }
                        },
                        NodeState::Open => {
                            (1.0, NodeState::Open)
                        },
                        NodeState::Closed => {
                            (1.0, NodeState::Closed)
                        }
                    };
                    *state = new_state;
                    last_stack.push(is_last);
                    scale_stack.push(scale * new_scale);
                },
                FileNode::File {name, ..} => {
                    //cx.move_turtle(0., 3.5);
                    cx.turtle_align_y();
                    let wleft = cx.get_width_left() - 10.;
                    self.tree_text.wrapping = Wrapping::Ellipsis(wleft);
                    //cx.realign_turtle(Align::left_center(), false);
                    self.tree_text.color = if is_marked {
                        self.tree_folder_color
                    }
                    else {
                        self.tree_file_color
                    };
                    self.tree_text.draw_text(cx, name);
                }
            }
            
            self.node_bg.end_quad(cx, &inst);
            
            cx.turtle_new_line();
            // if any of the parents is closing, don't count alternating lines
            if !file_walker.current_closing() {
                counter += 1;
            }
        }
        
        // draw filler nodes
        let view_total = cx.get_turtle_bounds();
        let rect_now = cx.get_turtle_rect();
        let bg_even = cx.color("bg_selected");
        let bg_odd = cx.color("bg_odd");
        let mut y = view_total.y;
        while y < rect_now.h {
            self.node_bg.color = if counter & 1 == 0 {bg_even}else {bg_odd};
            self.node_bg.draw_quad_walk(
                cx,
                Width::Fill,
                Height::Fix((rect_now.h - y).min(self.row_height)),
                Margin::zero()
            );
            cx.turtle_new_line();
            y += self.row_height;
            counter += 1;
        }
        
        // draw the drag item overlay layer if need be
        if let Some(mv) = &self._drag_move {
            if let Ok(()) = self.drag_view.begin_view(cx, Layout {
                abs_origin: Some(Vec2 {x: mv.abs.x + 5., y: mv.abs.y + 5.}),
                ..Default::default()
            }) {
                let mut file_walker = FileWalker::new(&mut self.root_node);
                while let Some((_depth, _index, _len, node)) = file_walker.walk() {
                    let node_draw = if let Some(node_draw) = node.get_draw() {node_draw}else {continue};
                    if node_draw.marked != 0 {
                        let inst = self.drag_bg.begin_quad(cx, &self.drag_bg_layout);
                        self.tree_text.color = self.tree_folder_color;
                        self.tree_text.draw_text(cx, match node {
                            FileNode::Folder {name, ..} => {name},
                            FileNode::File {name, ..} => {name}
                        });
                        self.drag_bg.end_quad(cx, &inst);
                    }
                }
                self.drag_view.end_view(cx);
            }
        }
        self.view.end_view(cx);
    }
    
}

