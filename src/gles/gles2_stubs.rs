/*
 * OpenGL ES 2.0 Stub Functions for touchHLE
 * * Этот модуль предоставляет заглушки для функций OpenGL ES 2.0,
 * чтобы предотвратить краши на устройствах без поддержки ES 2.0,
 * не ломая при этом отрисовку в GLES 1.1 играх.
 */

use std::os::raw::{c_char, c_int, c_uchar, c_uint, c_void};
use crate::mem::MutVoidPtr;
use crate::environment::Environment;

// Определение базовых типов OpenGL
pub type GLenum = c_uint;
pub type GLuint = c_uint;
pub type GLint = c_int;
pub type GLchar = c_char;
pub type GLsizei = c_int;
pub type GLboolean = c_uchar;
pub type GLfloat = f32;
pub type GLdouble = f64;
pub type GLbitfield = c_uint;
pub type GLintptr = isize;
pub type GLsizeiptr = isize;
pub type GLubyte = u8;

// Константы ES 2.0 которых нет в ES 1.1
pub const GL_ES_VERSION_2_0: GLenum = 1;
pub const GL_VERTEX_SHADER: GLenum = 0x8B31;
pub const GL_FRAGMENT_SHADER: GLenum = 0x8B30;
pub const GL_COMPILE_STATUS: GLenum = 0x8B81;
pub const GL_LINK_STATUS: GLenum = 0x8B82;
pub const GL_INFO_LOG_LENGTH: GLenum = 0x8B84;
pub const GL_SHADER_TYPE: GLenum = 0x8B4F;
pub const GL_SHADER_SOURCE_LENGTH: GLenum = 0x8B88;
pub const GL_FLOAT_VEC2: GLenum = 0x8B50;
pub const GL_FLOAT_VEC3: GLenum = 0x8B51;
pub const GL_FLOAT_VEC4: GLenum = 0x8B52;
pub const GL_INT_VEC2: GLenum = 0x8B53;
pub const GL_INT_VEC3: GLenum = 0x8B54;
pub const GL_INT_VEC4: GLenum = 0x8B55;
pub const GL_BOOL: GLenum = 0x8B56;
pub const GL_BOOL_VEC2: GLenum = 0x8B57;
pub const GL_BOOL_VEC3: GLenum = 0x8B58;
pub const GL_BOOL_VEC4: GLenum = 0x8B59;
pub const GL_FLOAT_MAT2: GLenum = 0x8B5A;
pub const GL_FLOAT_MAT3: GLenum = 0x8B5B;
pub const GL_FLOAT_MAT4: GLenum = 0x8B5C;
pub const GL_SAMPLER_2D: GLenum = 0x8B5E;
pub const GL_SAMPLER_CUBE: GLenum = 0x8B60;
pub const GL_ACTIVE_ATTRIBUTES: GLenum = 0x8B89;
pub const GL_ACTIVE_UNIFORMS: GLenum = 0x8B86;
pub const GL_ATTACHED_SHADERS: GLenum = 0x8B85;
pub const GL_DELETE_STATUS: GLenum = 0x8B80;
pub const GL_VALIDATE_STATUS: GLenum = 0x8B83;

static mut NEXT_SHADER_ID: GLuint = 1;
static mut NEXT_PROGRAM_ID: GLuint = 1000;

fn next_shader_id() -> GLuint {
    unsafe {
        let id = NEXT_SHADER_ID;
        NEXT_SHADER_ID += 1;
        id
    }
}

fn next_program_id() -> GLuint {
    unsafe {
        let id = NEXT_PROGRAM_ID;
        NEXT_PROGRAM_ID += 1;
        id
    }
}

// MARK: - Shader Functions

#[no_mangle]
pub extern "C" fn glCreateShader(_type: GLenum) -> GLuint {
    log!("STUB: glCreateShader(type=0x{:x})", _type);
    next_shader_id()
}

#[no_mangle]
pub extern "C" fn glShaderSource(
    shader: GLuint,
    count: GLsizei,
    _string: *const *const GLchar,
    _length: *const GLint,
) {
    log!("STUB: glShaderSource(shader={}, count={})", shader, count);
}

#[no_mangle]
pub extern "C" fn glCompileShader(shader: GLuint) {
    log!("STUB: glCompileShader(shader={})", shader);
}

#[no_mangle]
pub extern "C" fn glDeleteShader(shader: GLuint) {
    log!("STUB: glDeleteShader(shader={})", shader);
}

#[no_mangle]
pub extern "C" fn glGetShaderiv(shader: GLuint, pname: GLenum, params: *mut GLint) {
    log!("STUB: glGetShaderiv(shader={}, pname=0x{:x})", shader, pname);
    if !params.is_null() {
        unsafe {
            match pname {
                GL_COMPILE_STATUS => *params = 1, // Успешно скомпилирован
                GL_DELETE_STATUS => *params = 0,
                GL_SHADER_TYPE => *params = GL_VERTEX_SHADER as GLint,
                GL_INFO_LOG_LENGTH => *params = 0,
                GL_SHADER_SOURCE_LENGTH => *params = 0,
                _ => *params = 0,
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn glGetShaderInfoLog(
    shader: GLuint,
    _bufSize: GLsizei,
    length: *mut GLsizei,
    _infoLog: *mut GLchar,
) {
    log!("STUB: glGetShaderInfoLog(shader={})", shader);
    if !length.is_null() {
        unsafe { *length = 0; }
    }
}

// MARK: - Program Functions

#[no_mangle]
pub extern "C" fn glCreateProgram() -> GLuint {
    log!("STUB: glCreateProgram()");
    next_program_id()
}

#[no_mangle]
pub extern "C" fn glAttachShader(program: GLuint, shader: GLuint) {
    log!("STUB: glAttachShader(program={}, shader={})", program, shader);
}

#[no_mangle]
pub extern "C" fn glLinkProgram(program: GLuint) {
    log!("STUB: glLinkProgram(program={})", program);
}

#[no_mangle]
pub extern "C" fn glUseProgram(program: GLuint) {
    log!("STUB: glUseProgram(program={})", program);
}

#[no_mangle]
pub extern "C" fn glDeleteProgram(program: GLuint) {
    log!("STUB: glDeleteProgram(program={})", program);
}

#[no_mangle]
pub extern "C" fn glGetProgramiv(program: GLuint, pname: GLenum, params: *mut GLint) {
    log!("STUB: glGetProgramiv(program={}, pname=0x{:x})", program, pname);
    if !params.is_null() {
        unsafe {
            match pname {
                GL_LINK_STATUS => *params = 1, // Успешно слинкован
                GL_DELETE_STATUS => *params = 0,
                GL_VALIDATE_STATUS => *params = 1,
                GL_ATTACHED_SHADERS => *params = 2,
                GL_ACTIVE_ATTRIBUTES => *params = 0,
                GL_ACTIVE_UNIFORMS => *params = 0,
                GL_INFO_LOG_LENGTH => *params = 0,
                _ => *params = 0,
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn glGetProgramInfoLog(
    program: GLuint,
    _bufSize: GLsizei,
    length: *mut GLsizei,
    _infoLog: *mut GLchar,
) {
    log!("STUB: glGetProgramInfoLog(program={})", program);
    if !length.is_null() {
        unsafe { *length = 0; }
    }
}

#[no_mangle]
pub extern "C" fn glValidateProgram(program: GLuint) {
    log!("STUB: glValidateProgram(program={})", program);
}

// MARK: - Attribute & Uniform Functions

#[no_mangle]
pub extern "C" fn glGetAttribLocation(program: GLuint, name: *const GLchar) -> GLint {
    let name_str = if name.is_null() {
        "(null)"
    } else {
        unsafe {
            std::ffi::CStr::from_ptr(name).to_str().unwrap_or("invalid")
        }
    };
    log!("STUB: glGetAttribLocation(program={}, name={})", program, name_str);
    -1 // Не найдено
}

#[no_mangle]
pub extern "C" fn glGetUniformLocation(program: GLuint, name: *const GLchar) -> GLint {
    let name_str = if name.is_null() {
        "(null)"
    } else {
        unsafe {
            std::ffi::CStr::from_ptr(name).to_str().unwrap_or("invalid")
        }
    };
    log!("STUB: glGetUniformLocation(program={}, name={})", program, name_str);
    -1 // Не найдено
}

#[no_mangle]
pub extern "C" fn glVertexAttribPointer(
    index: GLuint,
    size: GLint,
    _type: GLenum,
    _normalized: GLboolean,
    _stride: GLsizei,
    _pointer: *const c_void,
) {
    log!("STUB: glVertexAttribPointer(index={}, size={})", index, size);
}

#[no_mangle]
pub extern "C" fn glEnableVertexAttribArray(index: GLuint) {
    log!("STUB: glEnableVertexAttribArray(index={})", index);
}

#[no_mangle]
pub extern "C" fn glDisableVertexAttribArray(index: GLuint) {
    log!("STUB: glDisableVertexAttribArray(index={})", index);
}

#[no_mangle]
pub extern "C" fn glVertexAttrib1f(index: GLuint, _x: GLfloat) {
    log!("STUB: glVertexAttrib1f(index={})", index);
}

#[no_mangle]
pub extern "C" fn glVertexAttrib2f(index: GLuint, _x: GLfloat, _y: GLfloat) {
    log!("STUB: glVertexAttrib2f(index={})", index);
}

#[no_mangle]
pub extern "C" fn glVertexAttrib3f(index: GLuint, _x: GLfloat, _y: GLfloat, _z: GLfloat) {
    log!("STUB: glVertexAttrib3f(index={})", index);
}

#[no_mangle]
pub extern "C" fn glVertexAttrib4f(index: GLuint, _x: GLfloat, _y: GLfloat, _z: GLfloat, _w: GLfloat) {
    log!("STUB: glVertexAttrib4f(index={})", index);
}

// MARK: - Uniform Setters

#[no_mangle]
pub extern "C" fn glUniform1f(location: GLint, _v0: GLfloat) {
    log!("STUB: glUniform1f(location={})", location);
}

#[no_mangle]
pub extern "C" fn glUniform2f(location: GLint, _v0: GLfloat, _v1: GLfloat) {
    log!("STUB: glUniform2f(location={})", location);
}

#[no_mangle]
pub extern "C" fn glUniform3f(location: GLint, _v0: GLfloat, _v1: GLfloat, _v2: GLfloat) {
    log!("STUB: glUniform3f(location={})", location);
}

#[no_mangle]
pub extern "C" fn glUniform4f(location: GLint, _v0: GLfloat, _v1: GLfloat, _v2: GLfloat, _v3: GLfloat) {
    log!("STUB: glUniform4f(location={})", location);
}

#[no_mangle]
pub extern "C" fn glUniform1i(location: GLint, _v0: GLint) {
    log!("STUB: glUniform1i(location={})", location);
}

#[no_mangle]
pub extern "C" fn glUniform2i(location: GLint, _v0: GLint, _v1: GLint) {
    log!("STUB: glUniform2i(location={})", location);
}

#[no_mangle]
pub extern "C" fn glUniform3i(location: GLint, _v0: GLint, _v1: GLint, _v2: GLint) {
    log!("STUB: glUniform3i(location={})", location);
}

#[no_mangle]
pub extern "C" fn glUniform4i(location: GLint, _v0: GLint, _v1: GLint, _v2: GLint, _v3: GLint) {
    log!("STUB: glUniform4i(location={})", location);
}

#[no_mangle]
pub extern "C" fn glUniform1fv(location: GLint, _count: GLsizei, _value: *const GLfloat) {
    log!("STUB: glUniform1fv(location={})", location);
}

#[no_mangle]
pub extern "C" fn glUniform2fv(location: GLint, _count: GLsizei, _value: *const GLfloat) {
    log!("STUB: glUniform2fv(location={})", location);
}

#[no_mangle]
pub extern "C" fn glUniform3fv(location: GLint, _count: GLsizei, _value: *const GLfloat) {
    log!("STUB: glUniform3fv(location={})", location);
}

#[no_mangle]
pub extern "C" fn glUniform4fv(location: GLint, _count: GLsizei, _value: *const GLfloat) {
    log!("STUB: glUniform4fv(location={})", location);
}

#[no_mangle]
pub extern "C" fn glUniform1iv(location: GLint, _count: GLsizei, _value: *const GLint) {
    log!("STUB: glUniform1iv(location={})", location);
}

#[no_mangle]
pub extern "C" fn glUniform2iv(location: GLint, _count: GLsizei, _value: *const GLint) {
    log!("STUB: glUniform2iv(location={})", location);
}

#[no_mangle]
pub extern "C" fn glUniform3iv(location: GLint, _count: GLsizei, _value: *const GLint) {
    log!("STUB: glUniform3iv(location={})", location);
}

#[no_mangle]
pub extern "C" fn glUniform4iv(location: GLint, _count: GLsizei, _value: *const GLint) {
    log!("STUB: glUniform4iv(location={})", location);
}

#[no_mangle]
pub extern "C" fn glUniformMatrix2fv(location: GLint, _count: GLsizei, _transpose: GLboolean, _value: *const GLfloat) {
    log!("STUB: glUniformMatrix2fv(location={})", location);
}

#[no_mangle]
pub extern "C" fn glUniformMatrix3fv(location: GLint, _count: GLsizei, _transpose: GLboolean, _value: *const GLfloat) {
    log!("STUB: glUniformMatrix3fv(location={})", location);
}

#[no_mangle]
pub extern "C" fn glUniformMatrix4fv(location: GLint, _count: GLsizei, _transpose: GLboolean, _value: *const GLfloat) {
    log!("STUB: glUniformMatrix4fv(location={})", location);
}

// MARK: - Separation Functions (ES 2.0 Specific Blends / Stencils)

#[no_mangle]
pub extern "C" fn glBlendEquationSeparate(modeRGB: GLenum, modeAlpha: GLenum) {
    log!("STUB: glBlendEquationSeparate(0x{:x}, 0x{:x})", modeRGB, modeAlpha);
}

#[no_mangle]
pub extern "C" fn glBlendFuncSeparate(srcRGB: GLenum, dstRGB: GLenum, srcAlpha: GLenum, dstAlpha: GLenum) {
    log!("STUB: glBlendFuncSeparate(0x{:x}, 0x{:x}, 0x{:x}, 0x{:x})", srcRGB, dstRGB, srcAlpha, dstAlpha);
}

#[no_mangle]
pub extern "C" fn glStencilOpSeparate(face: GLenum, _sfail: GLenum, _dpfail: GLenum, _dppass: GLenum) {
    log!("STUB: glStencilOpSeparate(face=0x{:x})", face);
}

#[no_mangle]
pub extern "C" fn glStencilFuncSeparate(face: GLenum, _func: GLenum, _ref_: GLint, _mask: GLuint) {
    log!("STUB: glStencilFuncSeparate(face=0x{:x})", face);
}

#[no_mangle]
pub extern "C" fn glStencilMaskSeparate(face: GLenum, _mask: GLuint) {
    log!("STUB: glStencilMaskSeparate(face=0x{:x})", face);
}

// MARK: - Capability Queries

#[no_mangle]
pub extern "C" fn glIsShader(shader: GLuint) -> GLboolean {
    log!("STUB: glIsShader(shader={})", shader);
    1 // GL_TRUE
}

#[no_mangle]
pub extern "C" fn glIsProgram(program: GLuint) -> GLboolean {
    log!("STUB: glIsProgram(program={})", program);
    1 // GL_TRUE
}

// MARK: - Shader Precision

#[no_mangle]
pub extern "C" fn glGetShaderPrecisionFormat(_shadertype: GLenum, _precisiontype: GLenum, range: *mut GLint, precision: *mut GLint) {
    log!("STUB: glGetShaderPrecisionFormat");
    if !range.is_null() {
        unsafe {
            *range = 127;
            *range.offset(1) = 127;
        }
    }
    if !precision.is_null() {
        unsafe {
            *precision = 23;
        }
    }
}

// MARK: - Other Functions

#[no_mangle]
pub extern "C" fn glReleaseShaderCompiler() {
    log!("STUB: glReleaseShaderCompiler()");
}

// (Функции glGetIntegerv, glGetString, glDrawArrays, glEnable, glBindTexture и другие
// общие функции удалены. Они будут корректно прокидываться в gles1_native.rs)
