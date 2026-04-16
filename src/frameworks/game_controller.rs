use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::foundation::ns_string;
use crate::objc::id;
use crate::Environment;

/// Экспортируем константы для линковщика (dyld)
pub const CONSTANTS: ConstantExports = &[
    ("_GCControllerDidConnectNotification", HostConstant::Id(get_connect_notification)),
    ("_GCControllerDidDisconnectNotification", HostConstant::Id(get_disconnect_notification)),
];

fn get_connect_notification(env: &mut Environment) -> id {
    // Возвращаем реальный NSString, чтобы NSNotificationCenter мог с ним работать
    ns_string::get_static_str(env, "GCControllerDidConnectNotification")
}

fn get_disconnect_notification(env: &mut Environment) -> id {
    ns_string::get_static_str(env, "GCControllerDidDisconnectNotification")
}
