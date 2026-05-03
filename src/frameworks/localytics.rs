/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 */
#![allow(dead_code)]
//! `LocalyticsAmpSession` full implementation.

use crate::objc::{
    id, msg, nil, objc_classes, ClassExports, HostObject
};

struct LocalyticsAmpSessionHostObject {}
impl HostObject for LocalyticsAmpSessionHostObject {}

pub const CLASSES: ClassExports = &objc_classes! {
    @class LocalyticsAmpSession : NSObject {
        @host_object LocalyticsAmpSessionHostObject;
    }

    @implementation LocalyticsAmpSession

    // Инициализация
    - (id)init {
        env.objc.set_host_object(this, LocalyticsAmpSessionHostObject {});
        this
    }

    // Инициализация с ключом приложения
    - (id)LocalyticsSession:(id)_app_key {
        env.objc.set_host_object(this, LocalyticsAmpSessionHostObject {});
        this
    }

    // Стандартные методы жизненного цикла Localytics
    - (())startSession:(id)_app_key {
        log!("LocalyticsAmpSession startSession: started");
    }

    - (())open {
        log!("LocalyticsAmpSession open: session opened");
    }

    - (())close {
        log!("LocalyticsAmpSession close: session closed");
    }

    - (())upload {
        log!("LocalyticsAmpSession upload: uploading data");
    }

    - (())resume {
        log!("LocalyticsAmpSession resume: session resumed");
    }

    // Трекинг событий
    - (())tagEvent:(id)_event {
        log!("LocalyticsAmpSession tagEvent: recorded");
    }

    - (())tagEvent:(id)_event attributes:(id)_attributes {
        log!("LocalyticsAmpSession tagEvent:attributes: recorded");
    }

    - (())tagScreen:(id)_screen {
        log!("LocalyticsAmpSession tagScreen: recorded");
    }

    // Класс-методы (некоторые версии SDK вызывают их напрямую у класса)
    + (())startSession:(id)_app_key {
        log!("LocalyticsAmpSession [class] startSession:");
    }
    
    + (())tagEvent:(id)_event {
        log!("LocalyticsAmpSession [class] tagEvent:");
    }

    @end
};
