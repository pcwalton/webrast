/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use atlas::Atlas;
use batch::Batch;

use gleam::gl::{self, GLenum, GLint, GLuint};
use std::cell::RefCell;
use std::mem;
use std::rc::Rc;

static VERTEX_SHADER: &'static str = "
    attribute vec3 aVertexPosition;
    attribute vec4 aVertexColor;
    attribute vec2 aBufferGamma;
    attribute vec2 aTextureCoord;

    varying vec4 vVertexColor;
    varying vec2 vBufferGamma;
    varying vec2 vTextureCoord;

    void main() {
        vVertexColor = aVertexColor;
        vBufferGamma = aBufferGamma;
        vTextureCoord = aTextureCoord;
        gl_Position = vec4(aVertexPosition, 1.0);
    }
";

static FRAGMENT_SHADER: &'static str = "
    #ifdef GL_ES
        precision mediump float;
    #endif

    uniform sampler2D uTexture;

    varying vec4 vVertexColor;
    varying vec2 vBufferGamma;
    varying vec2 vTextureCoord;

    void main() {
        vec4 lTextureColor = texture2D(uTexture, vTextureCoord);
        float lAlpha = smoothstep(vBufferGamma[0] - vBufferGamma[1],
                                  vBufferGamma[0] + vBufferGamma[1],
                                  lTextureColor.a);
        vec4 lColor = vec4(lTextureColor.rgb, lAlpha) + vVertexColor;
        if (lColor.ga == vec2(0.0, 0.0))
            discard;
        gl_FragColor = lColor;
    }
";

struct DrawBuffers {
    vertex_position_buffer: GLuint,
    vertex_color_buffer: GLuint,
    buffer_gamma_buffer: GLuint,
    texture_coord_buffer: GLuint,
}

impl DrawBuffers {
    fn new() -> DrawBuffers {
        let buffers = gl::gen_buffers(4);
        DrawBuffers {
            vertex_position_buffer: buffers[0],
            vertex_color_buffer: buffers[1],
            buffer_gamma_buffer: buffers[2],
            texture_coord_buffer: buffers[3],
        }
    }
}

pub struct DrawContext {
    program: Program,
    buffers: DrawBuffers,
    pub atlas: Rc<RefCell<Atlas>>,
}

struct Program {
    program: GLuint,
    vertex_position_attribute: GLuint,
    vertex_color_attribute: GLuint,
    buffer_gamma_attribute: GLuint,
    texture_coord_attribute: GLuint,
    texture_uniform: GLuint,
}

fn compile_shader(source_string: &str, shader_type: GLenum) -> GLuint {
    let shader = gl::create_shader(shader_type);
    gl::shader_source(shader, &[ source_string.as_bytes() ]);
    gl::compile_shader(shader);
    if gl::get_shader_iv(shader, gl::COMPILE_STATUS) == 0 {
        panic!("failed to compile shader: {}", gl::get_shader_info_log(shader))
    }
    shader
}

fn create_program() -> GLuint {
    let vertex_shader = compile_shader(VERTEX_SHADER, gl::VERTEX_SHADER);
    let fragment_shader = compile_shader(FRAGMENT_SHADER, gl::FRAGMENT_SHADER);
    let program = gl::create_program();
    gl::attach_shader(program, vertex_shader);
    gl::attach_shader(program, fragment_shader);
    gl::link_program(program);
    if gl::get_program_iv(program, gl::LINK_STATUS) == 0 {
        panic!("failed to compile shader program: {}", gl::get_program_info_log(program))
    }
    program
}

impl Program {
    fn new() -> Program {
        let program = create_program();
        let vertex_position_attribute = gl::get_attrib_location(program, "aVertexPosition");
        let vertex_color_attribute = gl::get_attrib_location(program, "aVertexColor");
        let buffer_gamma_attribute = gl::get_attrib_location(program, "aBufferGamma");
        let texture_coord_attribute = gl::get_attrib_location(program, "aTextureCoord");
        let texture_uniform = gl::get_uniform_location(program, "uTexture");
        gl::enable_vertex_attrib_array(vertex_position_attribute as GLuint);
        gl::enable_vertex_attrib_array(vertex_color_attribute as GLuint);
        gl::enable_vertex_attrib_array(buffer_gamma_attribute as GLuint);
        gl::enable_vertex_attrib_array(texture_coord_attribute as GLuint);
        Program {
            program: program,
            vertex_position_attribute: vertex_position_attribute as GLuint,
            vertex_color_attribute: vertex_color_attribute as GLuint,
            buffer_gamma_attribute: buffer_gamma_attribute as GLuint,
            texture_coord_attribute: texture_coord_attribute as GLuint,
            texture_uniform: texture_uniform as GLuint,
        }
    }
}

impl DrawContext {
    pub fn new(atlas: Rc<RefCell<Atlas>>) -> DrawContext {
        DrawContext {
            program: Program::new(),
            buffers: DrawBuffers::new(),
            atlas: atlas,
        }
    }

    pub fn init_gl_state(&mut self) {
        gl::use_program(self.program.program);
        gl::enable(gl::TEXTURE_2D);
        gl::enable(gl::BLEND);
        gl::enable(gl::STENCIL_TEST);
        gl::enable(gl::DEPTH_TEST);
        gl::blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::stencil_mask(1);
        gl::stencil_func_separate(gl::FRONT, gl::GREATER, 1, 1);
        gl::stencil_func_separate(gl::BACK, gl::ALWAYS, 1, 1);
        gl::stencil_op_separate(gl::FRONT, gl::KEEP, gl::KEEP, gl::KEEP);
        gl::stencil_op_separate(gl::BACK, gl::KEEP, gl::ZERO, gl::REPLACE);
    }

    pub fn clear(&mut self) {
        gl::depth_mask(true);
        gl::clear_depth(0.5);
        gl::clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);
        gl::depth_mask(false);
    }

    pub fn draw_batch(&mut self, batch: &Batch) {
        gl::active_texture(gl::TEXTURE0);
        gl::bind_texture(gl::TEXTURE_2D, self.atlas.borrow().texture);
        gl::uniform_1i(self.program.texture_uniform as GLint, 0);

        self.buffer_data_for_batch(batch);

        let elements_u8 = unsafe {
            mem::transmute::<&[_],&[u8]>(&batch.elements[..])
        };
        gl::draw_elements(gl::TRIANGLES,
                          batch.elements.len() as i32,
                          gl::UNSIGNED_INT,
                          Some(elements_u8));
    }

    fn buffer_data_for_batch(&mut self, batch: &Batch) {
        gl::bind_buffer(gl::ARRAY_BUFFER, self.buffers.vertex_position_buffer);
        gl::buffer_data(gl::ARRAY_BUFFER, &batch.vertices[..], gl::DYNAMIC_DRAW);
        gl::vertex_attrib_pointer_f32(self.program.vertex_position_attribute, 3, false, 0, 0);
        debug!("drawing vertices: {:?}", &batch.vertices[..]);

        gl::bind_buffer(gl::ARRAY_BUFFER, self.buffers.vertex_color_buffer);
        gl::buffer_data(gl::ARRAY_BUFFER, &batch.colors[..], gl::DYNAMIC_DRAW);
        gl::vertex_attrib_pointer_u8(self.program.vertex_color_attribute, 4, false, 0, 0);
        debug!("... colors: {:?}", &batch.colors[..]);

        gl::bind_buffer(gl::ARRAY_BUFFER, self.buffers.buffer_gamma_buffer);
        gl::buffer_data(gl::ARRAY_BUFFER, &batch.buffer_gamma[..], gl::DYNAMIC_DRAW);
        gl::vertex_attrib_pointer_f32(self.program.buffer_gamma_attribute, 2, false, 0, 0);
        debug!("... buffer gamma: {:?}", &batch.buffer_gamma[..]);

        gl::bind_buffer(gl::ARRAY_BUFFER, self.buffers.texture_coord_buffer);
        gl::buffer_data(gl::ARRAY_BUFFER, &batch.texture_coords[..], gl::DYNAMIC_DRAW);
        gl::vertex_attrib_pointer_f32(self.program.texture_coord_attribute, 2, false, 0, 0);
        debug!("... texture coords: {:?}", &batch.texture_coords[..]);
    }

    pub fn finish(&self) {
        gl::finish();
    }
}


