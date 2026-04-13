use crate::objc_classes;
use crate::objc::{id, msg, nil, HostObject, retain, release, autorelease};
use crate::Environment;

// Структура для хранения состояния анимации
pub(super) struct CAKeyframeAnimationHostObject {
    key_path: id,
    values: id,
    key_times: id,
    duration: f64,
    delegate: id,
    removed_on_completion: bool,
}
impl HostObject for CAKeyframeAnimationHostObject {}

pub const CLASSES: crate::objc::ClassExports = objc_classes! {
    (env, this, _cmd);

    // Наследуемся напрямую от NSObject, чтобы избежать конфликтов HostObject с родительскими классами
    @implementation CAKeyframeAnimation : NSObject

    + (id)alloc {
        let host_object = Box::new(CAKeyframeAnimationHostObject {
            key_path: nil,
            values: nil,
            key_times: nil,
            duration: 0.0,
            delegate: nil,
            removed_on_completion: true,
        });
        env.objc.alloc_object(this, host_object, &mut env.mem)
    }

    - (id)init {
        this
    }

    - (())dealloc {
        let host = env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this);
        if host.key_path != nil { release(env, host.key_path); }
        if host.values != nil { release(env, host.values); }
        if host.key_times != nil { release(env, host.key_times); }
        
        env.objc.dealloc_object(this, &mut env.mem)
    }

    // Тот самый метод, на котором упала игра
    + (id)animationWithKeyPath:(id)path {
        let anim: id = msg![env; this alloc];
        let anim: id = msg![env; anim init];
        if path != nil {
            () = msg![env; anim setKeyPath:path];
        }
        autorelease(env, anim)
    }

    // --- Геттеры и сеттеры свойств 애니메이션 ---

    - (id)keyPath { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).key_path }
    - (())setKeyPath:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).key_path;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).key_path = val;
        if old != nil { release(env, old); }
    }

    - (id)values { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).values }
    - (())setValues:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).values;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).values = val;
        if old != nil { release(env, old); }
    }

    - (id)keyTimes { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).key_times }
    - (())setKeyTimes:(id)val {
        let old = env.objc.borrow::<CAKeyframeAnimationHostObject>(this).key_times;
        if val != nil { retain(env, val); }
        env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).key_times = val;
        if old != nil { release(env, old); }
    }

    - (f64)duration { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).duration }
    - (())setDuration:(f64)val { env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).duration = val; }

    - (id)delegate { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).delegate }
    - (())setDelegate:(id)val { env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).delegate = val; }

    - (bool)removedOnCompletion { env.objc.borrow::<CAKeyframeAnimationHostObject>(this).removed_on_completion }
    - (())setRemovedOnCompletion:(bool)val { env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).removed_on_completion = val; }

    @end
};

