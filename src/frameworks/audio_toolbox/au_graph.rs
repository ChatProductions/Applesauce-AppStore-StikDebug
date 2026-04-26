/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//!
//! Minimal `AUGraph.h` (Audio Unit Processing Graph Services).
//!
//! Реализован граф для эмуляции iOS 2.0-4.3.5. Поддерживаются структуры
//! соединений узлов и рендер-коллбэков для полноценного отслеживания графа,
//! [span_6](start_span)нужного играм вроде Plants vs Zombies[span_6](end_span). Каждая input-шина 3D Mixer-а 
//! превращается в отдельный OpenAL-источник, callback дёргается из общего 
[span_7](start_span)//! run-loop'а через `audio_unit::render_audio_unit`[span_7](end_span).

use std::collections::HashMap;
use std::time::Instant;

use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::frameworks::audio_toolbox::audio_components::{
    self, AURenderCallbackStruct, AudioComponentInstance,
};
use crate::frameworks::audio_toolbox::audio_unit::{
    setup_audio_unit_for_render, AudioUnit,
};
use crate::frameworks::carbon_core::{paramErr, OSStatus};
use crate::frameworks::core_audio_types::AudioStreamBasicDescription;
use crate::frameworks::core_foundation::cf_run_loop::CFRunLoopGetMain;
use crate::frameworks::foundation::ns_run_loop;
use crate::mem::{guest_size_of, ConstPtr, MutPtr, SafeRead};

// =========================================================================
// MARK: - Типы
// =========================================================================

[span_8](start_span)/// `AUNode` — целочисленный handle ноды в графе[span_8](end_span).
pub type AUNode = i32;

#[repr(C, packed)]
pub struct OpaqueAUGraph {
    _pad: u8,
}
unsafe impl SafeRead for OpaqueAUGraph {}

pub type AUGraph = MutPtr<OpaqueAUGraph>;

[span_9](start_span)/// Описание компонента, как у `AudioComponentDescription` в `audio_components`[span_9](end_span).
#[repr(C, packed)]
#[derive(Copy, Clone, Default)]
struct ComponentDesc {
    component_type: u32,
    component_sub_type: u32,
    component_manufacturer: u32,
    component_flags: u32,
    component_flags_mask: u32,
}
unsafe impl SafeRead for ComponentDesc {}

/// Соединение между двумя узлами графа (на основе AudioUnitNodeConnection).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct AudioUnitNodeConnection {
    source_node: AUNode,
    source_output_number: u32,
    dest_node: AUNode,
    dest_input_number: u32,
}

#[derive(Default, Clone)]
struct GraphNode {
    desc: ComponentDesc,
    audio_unit: Option<AudioUnit>,
}

#[derive(Default)]
struct GraphState {
    nodes: HashMap<AUNode, GraphNode>,
    connections: Vec<AudioUnitNodeConnection>, // Добавлено хранение соединений
    next_node_id: AUNode,
    is_open: bool,
    is_initialized: bool,
    is_running: bool,
    [span_10](start_span)/// Какой узел является конечным выходом (RemoteIO)[span_10](end_span).
    [span_11](start_span)/// Нужен, чтобы знать, какой `AudioUnit` стартовать в `AUGraphStart`[span_11](end_span).
    output_node: Option<AUNode>,
}

#[derive(Default)]
pub struct State {
    graphs: HashMap<AUGraph, GraphState>,
}
impl State {
    pub fn get(framework_state: &mut crate::frameworks::State) -> &mut Self {
        &mut framework_state.audio_toolbox.au_graph
    }
}

// =========================================================================
// MARK: - Константы (типы AudioUnit'ов)
// =========================================================================

const kAudioUnitType_Output: u32 = u32::from_be_bytes(*b"auou");
const kAudioUnitSubType_RemoteIO: u32 = u32::from_be_bytes(*b"rioc"); [span_12](start_span)//[span_12](end_span)

// =========================================================================
// MARK: - Жизненный цикл графа
// =========================================================================

fn NewAUGraph(env: &mut Environment, out_graph: MutPtr<AUGraph>) -> OSStatus {
    let g: AUGraph = env.mem.alloc_and_write(OpaqueAUGraph { _pad: 0 });
    State::get(&mut env.framework_state)
        .graphs
        .insert(g, GraphState::default()); [span_13](start_span)//[span_13](end_span)
    env.mem.write(out_graph, g);
    log!("NewAUGraph() -> {:?}", g);
    0
}

fn DisposeAUGraph(env: &mut Environment, graph: AUGraph) -> OSStatus {
    log!("DisposeAUGraph({:?})", graph);
    State::get(&mut env.framework_state).graphs.remove(&graph); [span_14](start_span)//[span_14](end_span)
    if !graph.is_null() {
        env.mem.free(graph.cast());
    }
    0
}

// =========================================================================
// MARK: - Узлы
// =========================================================================

fn AUGraphAddNode(
    env: &mut Environment,
    graph: AUGraph,
    in_desc: ConstPtr<ComponentDesc>,
    out_node: MutPtr<AUNode>,
) -> OSStatus {
    let desc = env.mem.read::<ComponentDesc, false>(in_desc);
    let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) else {
        return paramErr;
    }; [span_15](start_span)//[span_15](end_span)
    
    state.next_node_id += 1;
    let node_id = state.next_node_id;
    state.nodes.insert(
        node_id,
        GraphNode {
            desc,
            audio_unit: None,
        },
    ); [span_16](start_span)//[span_16](end_span)

    [span_17](start_span)// Если это RemoteIO — запоминаем его как выходной узел графа[span_17](end_span).
    let is_output = desc.component_type == kAudioUnitType_Output
        && desc.component_sub_type == kAudioUnitSubType_RemoteIO; [span_18](start_span)//[span_18](end_span)
    
    if is_output {
        state.output_node = Some(node_id);
    [span_19](start_span)} //[span_19](end_span)

    env.mem.write(out_node, node_id);
    log!(
        "AUGraphAddNode({:?}, type=0x{:08x} sub=0x{:08x}) -> node={} (output={})",
        graph, desc.component_type, desc.component_sub_type, node_id, is_output
    ); [span_20](start_span)//[span_20](end_span)
    0
}

fn AUGraphRemoveNode(env: &mut Environment, graph: AUGraph, node: AUNode) -> OSStatus {
    if let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) {
        state.nodes.remove(&node);
        // Также очищаем любые соединения, связанные с удаленным узлом
        state.connections.retain(|c| c.source_node != node && c.dest_node != node);
    }
    0
}

fn AUGraphCountNodes(
    env: &mut Environment,
    graph: AUGraph,
    out_count: MutPtr<u32>,
) -> OSStatus {
    let count = State::get(&mut env.framework_state)
        .graphs
        .get(&graph)
        .map(|s| s.nodes.len() as u32)
        .unwrap_or(0); [span_21](start_span)//[span_21](end_span)
    env.mem.write(out_count, count);
    0
}

fn AUGraphGetIndNode(
    env: &mut Environment,
    graph: AUGraph,
    index: u32,
    out_node: MutPtr<AUNode>,
) -> OSStatus {
    let Some(state) = State::get(&mut env.framework_state).graphs.get(&graph) else {
        return paramErr;
    }; [span_22](start_span)//[span_22](end_span)
    let mut keys: Vec<AUNode> = state.nodes.keys().copied().collect();
    keys.sort();
    let Some(&node) = keys.get(index as usize) else {
        return paramErr;
    }; [span_23](start_span)//[span_23](end_span)
    env.mem.write(out_node, node);
    0
}

fn AUGraphNodeInfo(
    env: &mut Environment,
    graph: AUGraph,
    node: AUNode,
    out_desc: MutPtr<ComponentDesc>,
    out_audio_unit: MutPtr<AudioUnit>,
) -> OSStatus {
    let Some(state) = State::get(&mut env.framework_state).graphs.get(&graph) else {
        return paramErr;
    }; [span_24](start_span)//[span_24](end_span)
    let Some(graph_node) = state.nodes.get(&node) else {
        return paramErr;
    };
    if !out_desc.is_null() {
        env.mem.write(out_desc, graph_node.desc);
    [span_25](start_span)} //[span_25](end_span)
    if !out_audio_unit.is_null() {
        let au = graph_node.audio_unit.unwrap_or_else(MutPtr::null);
        env.mem.write(out_audio_unit, au);
    [span_26](start_span)} //[span_26](end_span)
    0
}

// =========================================================================
// MARK: - Open / Initialize / Start / Stop
// =========================================================================

fn AUGraphOpen(env: &mut Environment, graph: AUGraph) -> OSStatus {
    let node_ids: Vec<AUNode> = match State::get(&mut env.framework_state).graphs.get(&graph) {
        Some(s) => s.nodes.keys().copied().collect(),
        None => return paramErr,
    };
    log!("AUGraphOpen({:?}) {} node(s)", graph, node_ids.len()); [span_27](start_span)//[span_27](end_span)

    for node_id in node_ids {
        let guest_instance: AudioComponentInstance =
            audio_components::create_audio_unit_instance(env);
        if let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) {
            if let Some(graph_node) = state.nodes.get_mut(&node_id) {
                graph_node.audio_unit = Some(guest_instance);
            [span_28](start_span)} //[span_28](end_span)
        }
    }

    if let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) {
        state.is_open = true;
    [span_29](start_span)} //[span_29](end_span)
    0
}

fn AUGraphClose(env: &mut Environment, graph: AUGraph) -> OSStatus {
    if let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) {
        state.is_open = false;
    [span_30](start_span)} //[span_30](end_span)
    0
}

fn AUGraphInitialize(env: &mut Environment, graph: AUGraph) -> OSStatus {
    let units: Vec<AudioUnit> = match State::get(&mut env.framework_state).graphs.get(&graph) {
        Some(s) => s.nodes.values().filter_map(|n| n.audio_unit).collect(),
        None => return paramErr,
    }; [span_31](start_span)//[span_31](end_span)

    let run_loop = CFRunLoopGetMain(env);
    for unit in units {
        ns_run_loop::add_audio_unit(env, run_loop, unit);
    [span_32](start_span)} //[span_32](end_span)
    
    if let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) {
        state.is_initialized = true;
    [span_33](start_span)} //[span_33](end_span)
    0
}

fn AUGraphUninitialize(env: &mut Environment, graph: AUGraph) -> OSStatus {
    if let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) {
        state.is_initialized = false;
    [span_34](start_span)} //[span_34](end_span)
    0
}

fn AUGraphStart(env: &mut Environment, graph: AUGraph) -> OSStatus {
    let units: Vec<AudioUnit> = match State::get(&mut env.framework_state).graphs.get(&graph) {
        Some(s) => s.nodes.values().filter_map(|n| n.audio_unit).collect(),
        None => return paramErr,
    };
    log!("AUGraphStart({:?}) {} unit(s)", graph, units.len()); [span_35](start_span)//[span_35](end_span)

    for unit in units {
        setup_audio_unit_for_render(env, unit);
    [span_36](start_span)} //[span_36](end_span)

    if let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) {
        state.is_running = true;
    [span_37](start_span)} //[span_37](end_span)
    0
}

fn AUGraphStop(env: &mut Environment, graph: AUGraph) -> OSStatus {
    let units: Vec<AudioUnit> = match State::get(&mut env.framework_state).graphs.get(&graph) {
        Some(s) => s.nodes.values().filter_map(|n| n.audio_unit).collect(),
        None => return paramErr,
    }; [span_38](start_span)//[span_38](end_span)

    for unit in units {
        if let Some(obj) = audio_components::State::get(&mut env.framework_state)
            .audio_component_instances
            .get_mut(&unit)
        {
            obj.started = false;
        [span_39](start_span)} //[span_39](end_span)
    }
    
    if let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) {
        state.is_running = false;
    [span_40](start_span)} //[span_40](end_span)
    0
}

fn AUGraphIsOpen(env: &mut Environment, graph: AUGraph, out_is_open: MutPtr<u8>) -> OSStatus {
    let v = State::get(&mut env.framework_state).graphs.get(&graph).map(|s| s.is_open).unwrap_or(false);
    env.mem.write(out_is_open, v as u8); [span_41](start_span)//[span_41](end_span)
    0
}

fn AUGraphIsInitialized(env: &mut Environment, graph: AUGraph, out_v: MutPtr<u8>) -> OSStatus {
    let v = State::get(&mut env.framework_state).graphs.get(&graph).map(|s| s.is_initialized).unwrap_or(false);
    env.mem.write(out_v, v as u8); [span_42](start_span)//[span_42](end_span)
    0
}

fn AUGraphIsRunning(env: &mut Environment, graph: AUGraph, out_v: MutPtr<u8>) -> OSStatus {
    let v = State::get(&mut env.framework_state).graphs.get(&graph).map(|s| s.is_running).unwrap_or(false);
    env.mem.write(out_v, v as u8); [span_43](start_span)//[span_43](end_span)
    0
}

// =========================================================================
// MARK: - Connections / Callbacks
// =========================================================================

fn AUGraphConnectNodeInput(
    env: &mut Environment,
    graph: AUGraph,
    src_node: AUNode,
    src_output_number: u32,
    dest_node: AUNode,
    dest_input_number: u32,
) -> OSStatus {
    log!(
        "AUGraphConnectNodeInput({:?}): src_node={} out={} -> dest_node={} in={}",
        graph, src_node, src_output_number, dest_node, dest_input_number
    );

    let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) else {
        return paramErr;
    };
    
    // Полноценно регистрируем соединение узлов, как требует документация
    state.connections.push(AudioUnitNodeConnection {
        source_node: src_node,
        source_output_number: src_output_number,
        dest_node,
        dest_input_number,
    });

    0
}

fn AUGraphDisconnectNodeInput(
    env: &mut Environment,
    graph: AUGraph,
    dest_node: AUNode,
    dest_input_number: u32,
) -> OSStatus {
    let Some(state) = State::get(&mut env.framework_state).graphs.get_mut(&graph) else {
        return paramErr;
    };
    
    // Удаляем конкретное соединение из графа
    state.connections.retain(|c| !(c.dest_node == dest_node && c.dest_input_number == dest_input_number));
    
    log!("AUGraphDisconnectNodeInput({:?}): dest_node={} in={}", graph, dest_node, dest_input_number);
    0
}

fn AUGraphSetNodeInputCallback(
    env: &mut Environment,
    graph: AUGraph,
    dest_node: AUNode,
    dest_input_number: u32,
    in_input_callback: ConstPtr<AURenderCallbackStruct>,
) -> OSStatus {
    let cb = env.mem.read::<AURenderCallbackStruct, false>(in_input_callback);
    let dest_unit: Option<AudioUnit> = State::get(&mut env.framework_state)
        .graphs
        .get(&graph)
        .and_then(|s| s.nodes.get(&dest_node))
        .and_then(|n| n.audio_unit); [span_44](start_span)//[span_44](end_span)
        
    let Some(dest_unit) = dest_unit else {
        return paramErr;
    }; [span_45](start_span)//[span_45](end_span)

    let proc_copy = cb.input_proc;
    let ref_con_copy = cb.input_proc_ref_con; [span_46](start_span)//[span_46](end_span)

    if let Some(obj) = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&dest_unit)
    {
        let bus = obj.mixer_buses.entry(dest_input_number).or_default();
        bus.render_callback = Some(cb); [span_47](start_span)//[span_47](end_span)
        if bus.last_render_time.is_none() {
            bus.last_render_time = Some(Instant::now());
        [span_48](start_span)} //[span_48](end_span)
    }
    
    log!(
        "AUGraphSetNodeInputCallback({:?}, dest_node={}, bus={}, proc={:?}, ref_con={:?}) -> unit={:?}",
        graph, dest_node, dest_input_number, proc_copy, ref_con_copy, dest_unit
    ); [span_49](start_span)//[span_49](end_span)
    0
}

fn AUGraphUpdate(env: &mut Environment, _graph: AUGraph, out_is_updated: MutPtr<u8>) -> OSStatus {
    if !out_is_updated.is_null() {
        env.mem.write(out_is_updated, 1);
    [span_50](start_span)} //[span_50](end_span)
    0
}

// Заглушки для Notify коллбэков (в старых движках редко требуют сложной логики)
fn AUGraphAddRenderNotify(
    _env: &mut Environment, _graph: AUGraph, _proc: ConstPtr<u8>, _ref_con: ConstPtr<u8>,
) -> OSStatus {
    0
}

fn AUGraphRemoveRenderNotify(
    _env: &mut Environment, _graph: AUGraph, _proc: ConstPtr<u8>, _ref_con: ConstPtr<u8>,
) -> OSStatus {
    0
}

// =========================================================================
// MARK: - Экспорт функций
// =========================================================================

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(NewAUGraph(_)),
    export_c_func!(DisposeAUGraph(_)),
    export_c_func!(AUGraphAddNode(_, _, _)),
    export_c_func!(AUGraphRemoveNode(_, _)),
    export_c_func!(AUGraphCountNodes(_, _)),
    [span_51](start_span)export_c_func!(AUGraphGetIndNode(_, _, _)), //[span_51](end_span)
    export_c_func!(AUGraphNodeInfo(_, _, _, _)),
    export_c_func!(AUGraphOpen(_)),
    export_c_func!(AUGraphClose(_)),
    export_c_func!(AUGraphInitialize(_)),
    export_c_func!(AUGraphUninitialize(_)),
    export_c_func!(AUGraphStart(_)),
    export_c_func!(AUGraphStop(_)),
    export_c_func!(AUGraphIsOpen(_, _)),
    export_c_func!(AUGraphIsInitialized(_, _)),
    export_c_func!(AUGraphIsRunning(_, _)),
    export_c_func!(AUGraphConnectNodeInput(_, _, _, _, _)),
    export_c_func!(AUGraphDisconnectNodeInput(_, _, _)),
    export_c_func!(AUGraphSetNodeInputCallback(_, _, _, _)),
    export_c_func!(AUGraphUpdate(_, _)),
    export_c_func!(AUGraphAddRenderNotify(_, _, _)),
    export_c_func!(AUGraphRemoveRenderNotify(_, _, _)),
];

// Предотвращаем "unused" предупреждение для guest_size_of import.
#[allow(dead_code)]
const _SIZE_PROBE: usize = guest_size_of::<OpaqueAUGraph>() as usize; [span_52](start_span)//[span_52](end_span)

