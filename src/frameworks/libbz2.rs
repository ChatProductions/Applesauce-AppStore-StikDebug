use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, MutPtr};
use crate::Environment;
use std::collections::HashMap;
use std::sync::Mutex;
use bzip2::Decompress; // Требует крейт `bzip2`

// Коды возврата BZ2
const BZ_OK: i32 = 0;
const BZ_RUN_OK: i32 = 1;
const BZ_FLUSH_OK: i32 = 2;
const BZ_FINISH_OK: i32 = 3;
const BZ_STREAM_END: i32 = 4;
const BZ_PARAM_ERROR: i32 = -2;
const BZ_DATA_ERROR: i32 = -4;

lazy_static::lazy_static! {
    // Храним состояния распаковщиков, где ключ — указатель на bz_stream гостя
    static ref DECOMPRESSORS: Mutex<HashMap<u32, Decompress>> = Mutex::new(HashMap::new());
}

// int BZ2_bzDecompressInit(bz_stream *strm, int verbosity, int small)
#[allow(non_snake_case)]
pub fn BZ2_bzDecompressInit(_env: &mut Environment, strm_ptr: u32, _verbosity: i32, small: i32) -> i32 {
    if strm_ptr == 0 {
        return BZ_PARAM_ERROR;
    }
    
    let mut map = DECOMPRESSORS.lock().unwrap();
    // small != 0 включает режим экономии памяти, поддерживаемый bzip2
    map.insert(strm_ptr, Decompress::new(small != 0));
    
    BZ_OK
}

// int BZ2_bzDecompress(bz_stream *strm)
#[allow(non_snake_case)]
pub fn BZ2_bzDecompress(env: &mut Environment, strm_ptr: u32) -> i32 {
    if strm_ptr == 0 {
        return BZ_PARAM_ERROR;
    }

    let mut map = DECOMPRESSORS.lock().unwrap();
    let decompressor = match map.get_mut(&strm_ptr) {
        Some(d) => d,
        None => return BZ_PARAM_ERROR,
    };

    // Читаем поля bz_stream из памяти гостя (смещения для 32-bit)
    // char *next_in;      (+0)
    // unsigned avail_in;  (+4)
    // char *next_out;     (+16)
    // unsigned avail_out; (+20)
    let next_in_ptr: u32 = env.mem.read(ConstPtr::<u32>::from_bits(strm_ptr));
    let avail_in: u32 = env.mem.read(ConstPtr::<u32>::from_bits(strm_ptr + 4));
    let next_out_ptr: u32 = env.mem.read(ConstPtr::<u32>::from_bits(strm_ptr + 16));
    let avail_out: u32 = env.mem.read(ConstPtr::<u32>::from_bits(strm_ptr + 20));

    // Создаем слайсы для безопасного взаимодействия с bzip2
    let in_buf = env.mem.bytes_at(ConstPtr::<u8>::from_bits(next_in_ptr), avail_in as usize);
    let out_buf = env.mem.bytes_at_mut(MutPtr::<u8>::from_bits(next_out_ptr), avail_out as usize);

    let before_in = decompressor.total_in();
    let before_out = decompressor.total_out();

    let status = match decompressor.decompress(in_buf, out_buf) {
        Ok(bzip2::Status::Ok) => BZ_OK,
        Ok(bzip2::Status::RunOk) => BZ_RUN_OK,
        Ok(bzip2::Status::FlushOk) => BZ_FLUSH_OK,
        Ok(bzip2::Status::FinishOk) => BZ_FINISH_OK,
        Ok(bzip2::Status::StreamEnd) => BZ_STREAM_END,
        Err(_) => return BZ_DATA_ERROR,
        _ => BZ_OK, // Фолбэк
    };

    let consumed_in = (decompressor.total_in() - before_in) as u32;
    let produced_out = (decompressor.total_out() - before_out) as u32;

    // Обновляем структуру bz_stream обратно в памяти гостя
    env.mem.write(MutPtr::<u32>::from_bits(strm_ptr), next_in_ptr + consumed_in);
    env.mem.write(MutPtr::<u32>::from_bits(strm_ptr + 4), avail_in - consumed_in);
    env.mem.write(MutPtr::<u32>::from_bits(strm_ptr + 16), next_out_ptr + produced_out);
    env.mem.write(MutPtr::<u32>::from_bits(strm_ptr + 20), avail_out - produced_out);

    // Обновляем total_in_lo32 (+8) и total_out_lo32 (+24)
    let total_in_lo: u32 = env.mem.read(ConstPtr::<u32>::from_bits(strm_ptr + 8));
    env.mem.write(MutPtr::<u32>::from_bits(strm_ptr + 8), total_in_lo.wrapping_add(consumed_in));
    
    let total_out_lo: u32 = env.mem.read(ConstPtr::<u32>::from_bits(strm_ptr + 24));
    env.mem.write(MutPtr::<u32>::from_bits(strm_ptr + 24), total_out_lo.wrapping_add(produced_out));

    status
}

// int BZ2_bzDecompressEnd(bz_stream *strm)
#[allow(non_snake_case)]
pub fn BZ2_bzDecompressEnd(_env: &mut Environment, strm_ptr: u32) -> i32 {
    let mut map = DECOMPRESSORS.lock().unwrap();
    if map.remove(&strm_ptr).is_some() {
        BZ_OK
    } else {
        BZ_PARAM_ERROR
    }
}

// Популярная функция извлечения за один вызов
// int BZ2_bzBuffToBuffDecompress(char* dest, unsigned int* destLen, char* source, unsigned int sourceLen, int small, int verbosity)
#[allow(non_snake_case)]
pub fn BZ2_bzBuffToBuffDecompress(
    env: &mut Environment,
    dest: u32,
    dest_len_ptr: u32,
    source: u32,
    source_len: u32,
    small: i32,
    _verbosity: i32
) -> i32 {
    let dest_len: u32 = env.mem.read(ConstPtr::<u32>::from_bits(dest_len_ptr));
    
    let in_buf = env.mem.bytes_at(ConstPtr::<u8>::from_bits(source), source_len as usize);
    let out_buf = env.mem.bytes_at_mut(MutPtr::<u8>::from_bits(dest), dest_len as usize);

    let mut decompressor = Decompress::new(small != 0);
    match decompressor.decompress(in_buf, out_buf) {
        Ok(bzip2::Status::StreamEnd) | Ok(bzip2::Status::Ok) => {
            let produced = decompressor.total_out() as u32;
            env.mem.write(MutPtr::<u32>::from_bits(dest_len_ptr), produced);
            BZ_OK
        },
        _ => BZ_DATA_ERROR
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(BZ2_bzDecompressInit(_, _, _)),
    export_c_func!(BZ2_bzDecompress(_)),
    export_c_func!(BZ2_bzDecompressEnd(_)),
    export_c_func!(BZ2_bzBuffToBuffDecompress(_, _, _, _, _, _)),
];
