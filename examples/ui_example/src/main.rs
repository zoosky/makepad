/// This is Makepad, a work-in-progress livecoding IDE for 2D Design.
// This application is nearly 100% Wasm running on webGL. NO HTML AT ALL.
// The vision is to build a livecoding / design hybrid program,
// here procedural design and code are fused in one environment.
// If you have missed 'learnable programming' please check this out:
// http://worrydream.com/LearnableProgramming/
// Makepad aims to fulfill (some) of these ideas using a completely
// from-scratch renderstack built on the GPU and Rust/wasm.
// It will be like an IDE meets a vector designtool, and had offspring.
// Direct manipulation of the vectors modifies the code, the code modifies the vectors.
// And the result can be lasercut, embroidered or drawn using a plotter.
// This means our first goal is 2D path CNC with booleans (hence the CAD),
// and not dropshadowy-gradients.

// Find the repo and more explanation at github.com/makepad/makepad.
// We are developing the UI kit and code-editor as MIT, but the full stack
// will be a cloud/native app product in a few months.

// However to get to this amazing mixed-mode code editing-designtool,
// we first have to build an actually nice code editor (what you are looking at)
// And a vector stack with booleans (in progress)
// Up next will be full multiplatform support and more visual UI.
// All of the code is written in Rust, and it compiles to native and Wasm
// Its built on a novel immediate-mode UI architecture
// The styling is done with shaders written in Rust, transpiled to metal/glsl

// for now enjoy how smooth a full GPU editor can scroll (try select scrolling the edge)
// Also the tree fold-animates and the docking panel system works.
// Multi cursor/grid cursor also works with ctrl+click / ctrl+shift+click
// press alt or escape for animated codefolding outline view!

use widget::*;

struct App {
    window: Window,
    pass: Pass,
    color_texture: Texture,
    text: Text,
    blit: Blit,
    //trapezoid_text: TrapezoidText,
    main_view: View<NoScroll>,
}

main_app!(App);

impl App {
    pub fn style(cx: &mut Cx) -> Self {
        Self {
            window: Window::style(cx),
            pass: Pass::default(),
            color_texture: Texture::default(),
            text: Text {
                font_size: 8.0,
                font: cx.load_font_path("resources/Inconsolata-Regular.ttf"),
                ..Text::style(cx)
            },
            blit: Blit {
                ..Blit::style(cx)
            },
            //trapezoid_text: TrapezoidText::style(cx),
            main_view: View::style(cx),
        }
    }
    
    fn handle_app(&mut self, _cx: &mut Cx, event: &mut Event) {
        match event {
            Event::Construct => {
            },
            _ => ()
        }
    }
    
    fn draw_app(&mut self, cx: &mut Cx) {
        self.window.begin_window(cx);
        self.pass.begin_pass(cx);
        self.pass.add_color_texture(cx, &mut self.color_texture, ClearColor::ClearWith(color256(0, 0, 0)));
        
        let _ = self.main_view.begin_view(cx, Layout::default());
        cx.move_turtle(50., 50.);
        self.text.font_size = 9.0;
        for _ in 0..7{
            self.text.draw_text(cx, "- num -");
        }
        self.blit.draw_blit_abs(cx, &Texture {texture_id: Some(cx.fonts_atlas.texture_id)}, Rect {x: 100., y: 100., w: 700., h: 400.});
        //self.trapezoid_text.draw_character(cx, 100.,100., 0.5, 'X', &self.text.font);
        //self.trapezoid_text.draw_character(cx, 100.,300., 0.2, 'O', &self.text.font);
        
        self.main_view.end_view(cx);
        self.pass.end_pass(cx);
        self.window.end_window(cx);
    }
}
