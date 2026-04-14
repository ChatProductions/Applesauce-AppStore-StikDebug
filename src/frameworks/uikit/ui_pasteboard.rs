/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIPasteboard` and `UILocalNotification`.

use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};
use crate::frameworks::foundation::ns_string;
use std::collections::HashMap;

// MARK: - UIPasteboard host object

struct UIPasteboardHostObject {
    /// Имя буфера обмена (NSString*)
    name: id,
    /// Содержимое буфера в виде строки (NSString*)
    string: id,
    /// Хранилище данных по типам (UTI -> NSData)
    data_by_type: HashMap<String, id>,
    /// Флаг, сохраняется ли буфер обмена после завершения приложения
    persistent: bool,
}

impl HostObject for UIPasteboardHostObject {}

// MARK: - UILocalNotification host object

struct UILocalNotificationHostObject {
    /// Дата и время срабатывания уведомления (NSDate*)
    fire_date: id,
    /// Часовой пояс (NSTimeZone*)
    time_zone: id,
    /// Интервал повтора (NSCalendarUnit as NSUInteger)
    repeat_interval: u32,
    /// Календарь для повтора (NSCalendar*)
    repeat_calendar: id,
    /// Текст уведомления (NSString*)
    alert_body: id,
    /// Заголовок уведомления (NSString*)
    alert_title: id,
    /// Текст кнопки действия (NSString*)
    alert_action: id,
    /// Имя файла изображения для launch image (NSString*)
    alert_launch_image: id,
    /// Название звука (NSString*)
    sound_name: id,
    /// Номер значка приложения (NSInteger)
    application_icon_badge_number: i32,
    /// Пользовательские данные (NSDictionary*)
    user_info: id,
    /// Есть ли кнопка действия
    has_action: bool,
}

impl HostObject for UILocalNotificationHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =========================================================================
// MARK: - UIPasteboard
// =========================================================================

@implementation UIPasteboard: NSObject

// =========================================================================
// МЕТОДЫ КЛАССА (+)
// =========================================================================

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIPasteboardHostObject {
        name: nil,
        string: nil,
        data_by_type: HashMap::new(),
        persistent: false, // По умолчанию буфер не персистентный
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)generalPasteboard {
    // В большинстве игр используется для быстрого копирования текста. 
    // Создаем базовый инстанс.
    let instance: id = msg_class![env; UIPasteboard alloc];
    let instance: id = msg![env; instance init];
    
    // generalPasteboard по умолчанию persistent = YES в iOS
    env.objc.borrow_mut::<UIPasteboardHostObject>(instance).persistent = true;
    
    instance
}

+ (id)pasteboardWithName:(id)name create:(bool)_create {
    let instance: id = msg_class![env; UIPasteboard alloc];
    let instance: id = msg![env; instance init];

    // 1. Сначала делаем retain, пока env свободен
    retain(env, name);

    // 2. Затем заимствуем и сразу записываем (без сохранения переменной host)
    env.objc.borrow_mut::<UIPasteboardHostObject>(instance).name = name;

    instance
}

// =========================================================================
// МЕТОДЫ ЭКЗЕМПЛЯРА (-)
// =========================================================================

- (id)init {
    this
}

- (())dealloc {
    // Забираем данные, чтобы потом освободить их вне заимствования
    let (name, string, data_by_type) = {
        let host = env.objc.borrow_mut::<UIPasteboardHostObject>(this);
        (
            host.name,
            host.string,
            std::mem::take(&mut host.data_by_type),
        )
    };
    
    release(env, name);
    release(env, string);
    for (_, data) in data_by_type {
        release(env, data);
    }
    
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: - Свойства (Properties)

- (id)name { // NSString*
    env.objc.borrow::<UIPasteboardHostObject>(this).name
}

- (id)string { // NSString*
    env.objc.borrow::<UIPasteboardHostObject>(this).string
}

- (())setString:(id)string { // NSString*
    // Достаем старое значение, чтобы освободить его
    let old = env.objc.borrow::<UIPasteboardHostObject>(this).string;
    
    // Выполняем операции управления памятью
    release(env, old);
    retain(env, string);
    
    // Записываем новое значение
    env.objc.borrow_mut::<UIPasteboardHostObject>(this).string = string;
}

- (bool)isPersistent {
    env.objc.borrow::<UIPasteboardHostObject>(this).persistent
}

- (())setPersistent:(bool)value {
    env.objc.borrow_mut::<UIPasteboardHostObject>(this).persistent = value;
}

// MARK: - Работа с данными (NSData)

- (id)dataForPasteboardType:(id)pasteboard_type { // NSData*, NSString*
    if pasteboard_type == nil {
        return nil;
    }
    
    let type_str = ns_string::to_rust_string(env, pasteboard_type);
    let host = env.objc.borrow::<UIPasteboardHostObject>(this);
    
    // Используем .as_ref() чтобы передать &str вместо &Cow
    if let Some(&data) = host.data_by_type.get(type_str.as_ref()) {
        data
    } else {
        nil
    }
}

- (())setData:(id)data forPasteboardType:(id)pasteboard_type { // NSData*, NSString*
    if pasteboard_type == nil {
        return;
    }
    
    let type_str = ns_string::to_rust_string(env, pasteboard_type);
    
    if data != nil {
        retain(env, data);
    }
    
    // Безопасно обновляем HashMap и забираем старое значение
    let old_data = {
        let host = env.objc.borrow_mut::<UIPasteboardHostObject>(this);
        if data != nil {
            // Превращаем Cow во владеющий String через .into_owned()
            host.data_by_type.insert(type_str.into_owned(), data)
        } else {
            // Используем .as_ref() чтобы передать &str вместо &Cow
            host.data_by_type.remove(type_str.as_ref())
        }
    };
    
    // Освобождаем старое значение вне borrow_mut
    if let Some(old) = old_data {
        release(env, old);
    }
}

@end

// =========================================================================
// MARK: - UILocalNotification
// =========================================================================

@implementation UILocalNotification: NSObject

// =========================================================================
// МЕТОДЫ КЛАССА (+)
// =========================================================================

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UILocalNotificationHostObject {
        fire_date: nil,
        time_zone: nil,
        repeat_interval: 0, // NSCalendarUnitEra = 0 (no repeat)
        repeat_calendar: nil,
        alert_body: nil,
        alert_title: nil,
        alert_action: nil,
        alert_launch_image: nil,
        sound_name: nil,
        application_icon_badge_number: 0,
        user_info: nil,
        has_action: true, // По умолчанию кнопка есть
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// =========================================================================
// МЕТОДЫ ЭКЗЕМПЛЯРА (-)
// =========================================================================

- (id)init {
    this
}

- (())dealloc {
    // Забираем все объекты для освобождения
    let (fire_date, time_zone, repeat_calendar, alert_body, alert_title, 
         alert_action, alert_launch_image, sound_name, user_info) = {
        let host = env.objc.borrow_mut::<UILocalNotificationHostObject>(this);
        (
            host.fire_date,
            host.time_zone,
            host.repeat_calendar,
            host.alert_body,
            host.alert_title,
            host.alert_action,
            host.alert_launch_image,
            host.sound_name,
            host.user_info,
        )
    };
    
    release(env, fire_date);
    release(env, time_zone);
    release(env, repeat_calendar);
    release(env, alert_body);
    release(env, alert_title);
    release(env, alert_action);
    release(env, alert_launch_image);
    release(env, sound_name);
    release(env, user_info);
    
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: - Свойства (Properties)

- (id)fireDate { // NSDate*
    env.objc.borrow::<UILocalNotificationHostObject>(this).fire_date
}

- (())setFireDate:(id)fire_date { // NSDate*
    let old = env.objc.borrow::<UILocalNotificationHostObject>(this).fire_date;
    release(env, old);
    retain(env, fire_date);
    env.objc.borrow_mut::<UILocalNotificationHostObject>(this).fire_date = fire_date;
}

- (id)timeZone { // NSTimeZone*
    env.objc.borrow::<UILocalNotificationHostObject>(this).time_zone
}

- (())setTimeZone:(id)time_zone { // NSTimeZone*
    let old = env.objc.borrow::<UILocalNotificationHostObject>(this).time_zone;
    release(env, old);
    retain(env, time_zone);
    env.objc.borrow_mut::<UILocalNotificationHostObject>(this).time_zone = time_zone;
}

- (u32)repeatInterval { // NSCalendarUnit (NSUInteger)
    env.objc.borrow::<UILocalNotificationHostObject>(this).repeat_interval
}

- (())setRepeatInterval:(u32)interval {
    env.objc.borrow_mut::<UILocalNotificationHostObject>(this).repeat_interval = interval;
}

- (id)repeatCalendar { // NSCalendar*
    env.objc.borrow::<UILocalNotificationHostObject>(this).repeat_calendar
}

- (())setRepeatCalendar:(id)calendar { // NSCalendar*
    let old = env.objc.borrow::<UILocalNotificationHostObject>(this).repeat_calendar;
    release(env, old);
    retain(env, calendar);
    env.objc.borrow_mut::<UILocalNotificationHostObject>(this).repeat_calendar = calendar;
}

- (id)alertBody { // NSString*
    env.objc.borrow::<UILocalNotificationHostObject>(this).alert_body
}

- (())setAlertBody:(id)body { // NSString*
    let old = env.objc.borrow::<UILocalNotificationHostObject>(this).alert_body;
    release(env, old);
    retain(env, body);
    env.objc.borrow_mut::<UILocalNotificationHostObject>(this).alert_body = body;
}

- (id)alertTitle { // NSString*
    env.objc.borrow::<UILocalNotificationHostObject>(this).alert_title
}

- (())setAlertTitle:(id)title { // NSString*
    let old = env.objc.borrow::<UILocalNotificationHostObject>(this).alert_title;
    release(env, old);
    retain(env, title);
    env.objc.borrow_mut::<UILocalNotificationHostObject>(this).alert_title = title;
}

- (id)alertAction { // NSString*
    env.objc.borrow::<UILocalNotificationHostObject>(this).alert_action
}

- (())setAlertAction:(id)action { // NSString*
    let old = env.objc.borrow::<UILocalNotificationHostObject>(this).alert_action;
    release(env, old);
    retain(env, action);
    env.objc.borrow_mut::<UILocalNotificationHostObject>(this).alert_action = action;
}

- (bool)hasAction {
    env.objc.borrow::<UILocalNotificationHostObject>(this).has_action
}

- (())setHasAction:(bool)value {
    env.objc.borrow_mut::<UILocalNotificationHostObject>(this).has_action = value;
}

- (id)alertLaunchImage { // NSString*
    env.objc.borrow::<UILocalNotificationHostObject>(this).alert_launch_image
}

- (())setAlertLaunchImage:(id)image { // NSString*
    let old = env.objc.borrow::<UILocalNotificationHostObject>(this).alert_launch_image;
    release(env, old);
    retain(env, image);
    env.objc.borrow_mut::<UILocalNotificationHostObject>(this).alert_launch_image = image;
}

- (id)soundName { // NSString*
    env.objc.borrow::<UILocalNotificationHostObject>(this).sound_name
}

- (())setSoundName:(id)name { // NSString*
    let old = env.objc.borrow::<UILocalNotificationHostObject>(this).sound_name;
    release(env, old);
    retain(env, name);
    env.objc.borrow_mut::<UILocalNotificationHostObject>(this).sound_name = name;
}

- (i32)applicationIconBadgeNumber { // NSInteger
    env.objc.borrow::<UILocalNotificationHostObject>(this).application_icon_badge_number
}

- (())setApplicationIconBadgeNumber:(i32)number {
    env.objc.borrow_mut::<UILocalNotificationHostObject>(this).application_icon_badge_number = number;
}

- (id)userInfo { // NSDictionary*
    env.objc.borrow::<UILocalNotificationHostObject>(this).user_info
}

- (())setUserInfo:(id)info { // NSDictionary*
    let old = env.objc.borrow::<UILocalNotificationHostObject>(this).user_info;
    release(env, old);
    retain(env, info);
    env.objc.borrow_mut::<UILocalNotificationHostObject>(this).user_info = info;
}

@end

};
